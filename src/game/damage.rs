use super::{
    AppliedEffectDef, CardType, CommittedTriggerEvent, ControlFlow, CounterKind, Game,
    GameObjectId, KeywordAbility, Permanent, PlayerId, RetiredObject, Target, TriggerEventObject,
};

impl Game {
    pub(super) fn damage_target(&mut self, target: Option<Target>, amount: u16) {
        self.damage_target_from(None, target, amount);
    }

    /// Whether a static prevention on this permanent stops damage from this
    /// particular source. The source has to be a permanent, which is what
    /// "damage from artifact creatures" is about; damage from a spell is
    /// never prevented this way.
    pub(super) fn damage_is_prevented_from(
        &self,
        permanent: &Permanent,
        source: Option<GameObjectId>,
    ) -> bool {
        let Some(source) = source.and_then(|source| {
            self.battlefield
                .iter()
                .find(|candidate| candidate.card.id == source)
        }) else {
            return false;
        };
        let subject = self.trigger_event_object(source);
        self.visit_static_applied_effects(permanent, |applied| {
            if matches!(applied.effect, AppliedEffectDef::PreventDamageFrom(predicate)
                if self.trigger_object_matches(predicate, &subject, permanent.card.id, false))
            {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .is_break()
    }

    pub(super) fn damage_target_from(
        &mut self,
        source: Option<GameObjectId>,
        target: Option<Target>,
        amount: u16,
    ) {
        self.damage_target_from_kind(source, target, amount, false);
    }

    pub(super) fn damage_target_from_kind(
        &mut self,
        source: Option<GameObjectId>,
        target: Option<Target>,
        amount: u16,
        combat: bool,
    ) {
        let source_colors = source.map_or([false; 5], |source| self.object_colors(source));
        let lifelink_controller = source.and_then(|source| {
            self.source_controller_with_keyword(source, KeywordAbility::Lifelink)
        });
        let has_deathtouch = source.is_some_and(|source| {
            self.source_controller_with_keyword(source, KeywordAbility::Deathtouch)
                .is_some()
        });
        let dealt_damage = match target {
            Some(Target::Player(player)) => {
                self.deal_damage(player, amount);
                if amount > 0
                    && let Some(damager) = source.and_then(|source| {
                        self.battlefield
                            .iter_mut()
                            .find(|permanent| permanent.card.id == source)
                    })
                    && damager.controller != player
                {
                    damager.dealt_damage_to_opponent_this_turn = true;
                }
                self.publish_damage_to_player(source, player, amount);
                true
            }
            Some(Target::Permanent(id)) => {
                if let Some(index) = self
                    .battlefield
                    .iter()
                    .position(|permanent| permanent.card.id == id)
                {
                    if self.is_protected_from_colors(&self.battlefield[index], source_colors)
                        || self.damage_is_prevented_from(&self.battlefield[index], source)
                    {
                        return;
                    }
                    if self
                        .permanent_types(&self.battlefield[index])
                        .is_some_and(|types| types.contains(CardType::Planeswalker))
                    {
                        let remaining = self.battlefield[index]
                            .counters(CounterKind::Loyalty)
                            .saturating_sub(amount);
                        self.battlefield[index].set_counters(CounterKind::Loyalty, remaining);
                        true
                    } else {
                        let permanent = &mut self.battlefield[index];
                        permanent.damage = permanent.damage.saturating_add(amount);
                        if amount > 0 {
                            permanent.deathtouch_damage |= has_deathtouch;
                            if let Some(source) = source
                                && !permanent.damage_sources.contains(&source)
                            {
                                permanent.damage_sources.push(source);
                            }
                        }
                        true
                    }
                } else {
                    false
                }
            }
            Some(Target::Card(_) | Target::Spell(_)) | None => false,
        };
        if dealt_damage
            && amount > 0
            && let Some(controller) = lifelink_controller
        {
            self.gain_life(controller, amount);
        }
        if dealt_damage
            && amount > 0
            && let Some(source) = source
            && let Some(recipient) = target
            && let Some(source) = self.damage_source_event_object(source)
        {
            let event = CommittedTriggerEvent::DamageDealt {
                source,
                recipient,
                amount,
                combat,
            };
            self.capture_battlefield_triggers(&event);
        }
    }

    pub(super) fn damage_source_event_object(
        &self,
        source: GameObjectId,
    ) -> Option<TriggerEventObject> {
        if let Some(permanent) = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == source)
        {
            return Some(self.trigger_event_object(permanent));
        }
        if let Some(object) = self.stack.iter().find(|object| object.id == source) {
            return self.stack_trigger_event_object(object);
        }
        match self.retired_objects.get(&source) {
            Some(RetiredObject::Permanent { permanent, .. }) => {
                Some(self.trigger_event_object(permanent))
            }
            Some(RetiredObject::Stack(object)) => self.stack_trigger_event_object(object),
            Some(RetiredObject::Card(_)) | None => None,
        }
    }

    pub(super) fn damage_targets(&self) -> Vec<Target> {
        let mut targets = vec![Target::Player(PlayerId::One), Target::Player(PlayerId::Two)];
        targets.extend(
            self.battlefield
                .iter()
                .filter(|permanent| {
                    self.power(permanent).is_some()
                        || self
                            .permanent_types(permanent)
                            .is_some_and(|types| types.contains(CardType::Planeswalker))
                })
                .map(|permanent| Target::Permanent(permanent.card.id)),
        );
        targets
    }
}
