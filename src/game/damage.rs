use crate::card::{
    DamageEventMatcherDef, DamageKindDef, DamageRecipientMatcherDef, DamageSourceGroupDef,
    DamageSourceMatcherDef, ObjectRefDef,
};

use super::prevention_state::{
    ResolvedDamagePrevention, ResolvedDamagePreventionCapacity, ResolvedDamagePreventionCoverage,
    ResolvedDamageRecipientMatcher, ResolvedDamageSourceMatcher,
};
use super::{
    AppliedEffectDef, CardType, CommittedTriggerEvent, ControlFlow, CounterKind, Game,
    GameObjectId, KeywordAbility, Permanent, PlayerId, RelationalSourceFilter, RetiredObject,
    StackObjectKind, Target, TriggerEventObject,
};

impl Game {
    pub(super) fn damage_target(&mut self, target: Option<Target>, amount: u16) -> u16 {
        self.damage_target_from(None, target, amount)
    }

    /// Apply resolved prevention in creation order. Consumable promises are
    /// spent before unlimited prevention, matching the engine's historical
    /// Reverse Damage-before-Safe Passage behavior. A matching event promise
    /// is spent even when half of one damage rounds down to zero.
    fn apply_resolved_damage_prevention(
        &mut self,
        source: Option<GameObjectId>,
        target: Option<Target>,
        amount: u16,
        combat: bool,
    ) -> u16 {
        if amount == 0 || target.is_none() {
            return amount;
        }

        let source_object = source.and_then(|source| self.damage_source_event_object(source));
        let source_is_spell = source.is_some_and(|source| self.damage_source_is_spell(source));
        // Detach the vector while matching so predicate evaluation can read
        // the rest of the game without allocating a parallel match bitmap on
        // every damage event.
        let mut preventions = std::mem::take(&mut self.damage_preventions);
        let mut left = amount;
        let mut gained_life = Vec::new();

        for prevention in &mut preventions {
            if left == 0
                || !self.resolved_damage_prevention_matches(
                    prevention,
                    source,
                    source_object.as_ref(),
                    source_is_spell,
                    target,
                    combat,
                )
            {
                continue;
            }
            let prevented = match &mut prevention.capacity {
                ResolvedDamagePreventionCapacity::Amount(remaining) => {
                    let prevented = Self::damage_covered(prevention.coverage, left).min(*remaining);
                    *remaining -= prevented;
                    prevented
                }
                ResolvedDamagePreventionCapacity::Events(remaining) => {
                    *remaining = remaining.saturating_sub(1);
                    Self::damage_covered(prevention.coverage, left)
                }
                ResolvedDamagePreventionCapacity::Unlimited => continue,
            };
            left -= prevented;
            if let Some(player) = prevention.gain_life
                && prevented > 0
            {
                gained_life.push((player, prevented));
            }
        }

        preventions.retain(|prevention| {
            !matches!(
                prevention.capacity,
                ResolvedDamagePreventionCapacity::Amount(0)
                    | ResolvedDamagePreventionCapacity::Events(0)
            )
        });

        if left > 0 {
            for prevention in &preventions {
                if left == 0 {
                    break;
                }
                if !matches!(
                    prevention.capacity,
                    ResolvedDamagePreventionCapacity::Unlimited
                ) || !self.resolved_damage_prevention_matches(
                    prevention,
                    source,
                    source_object.as_ref(),
                    source_is_spell,
                    target,
                    combat,
                ) {
                    continue;
                }
                let prevented = Self::damage_covered(prevention.coverage, left);
                left -= prevented;
                if let Some(player) = prevention.gain_life
                    && prevented > 0
                {
                    gained_life.push((player, prevented));
                }
            }
        }

        self.damage_preventions = preventions;
        for (player, prevented) in gained_life {
            self.gain_life(player, prevented);
        }
        left
    }

    const fn damage_covered(coverage: ResolvedDamagePreventionCoverage, amount: u16) -> u16 {
        match coverage {
            ResolvedDamagePreventionCoverage::All => amount,
            ResolvedDamagePreventionCoverage::HalfRoundedDown => amount / 2,
        }
    }

    fn resolved_damage_prevention_matches(
        &self,
        prevention: &ResolvedDamagePrevention,
        source: Option<GameObjectId>,
        source_object: Option<&TriggerEventObject>,
        source_is_spell: bool,
        target: Option<Target>,
        combat: bool,
    ) -> bool {
        (!prevention.combat_only || combat)
            && self.resolved_damage_source_matches(
                prevention.source,
                source,
                source_object,
                source_is_spell,
            )
            && self.resolved_damage_recipient_matches(prevention.recipient, target)
    }

    fn resolved_damage_source_matches(
        &self,
        matcher: ResolvedDamageSourceMatcher,
        source: Option<GameObjectId>,
        source_object: Option<&TriggerEventObject>,
        source_is_spell: bool,
    ) -> bool {
        match matcher {
            ResolvedDamageSourceMatcher::Any => true,
            ResolvedDamageSourceMatcher::Exact(expected) => source == Some(expected),
            ResolvedDamageSourceMatcher::Except(excluded) => source != Some(excluded),
            ResolvedDamageSourceMatcher::Matching {
                predicate,
                relative_to,
            } => source_object.is_some_and(|source| {
                self.trigger_object_matches(predicate, source, relative_to, source_is_spell)
            }),
            ResolvedDamageSourceMatcher::Group(group) => {
                source.is_some_and(|source| self.damage_source_is_in_group(source, group))
            }
        }
    }

    fn resolved_damage_recipient_matches(
        &self,
        matcher: ResolvedDamageRecipientMatcher,
        target: Option<Target>,
    ) -> bool {
        match matcher {
            ResolvedDamageRecipientMatcher::Any => target.is_some(),
            ResolvedDamageRecipientMatcher::Exact(expected) => target == Some(expected),
            ResolvedDamageRecipientMatcher::PlayerAndCreaturesControlledBy(player) => {
                match target {
                    Some(Target::Player(recipient)) => recipient == player,
                    Some(Target::Permanent(id)) => self
                        .battlefield
                        .iter()
                        .find(|permanent| permanent.card.id == id)
                        .is_some_and(|permanent| {
                            permanent.controller == player
                                && self
                                    .permanent_types(permanent)
                                    .is_some_and(|types| types.contains(CardType::Creature))
                        }),
                    Some(Target::Card(_) | Target::Spell(_)) | None => false,
                }
            }
        }
    }

    /// Static prevention is derived live. Both prospective participants are
    /// visited because an applied effect can describe damage either to or by
    /// its affected object. A departed damage source is still represented by
    /// its last-known characteristics when a live recipient's predicate asks
    /// what dealt the damage.
    fn static_damage_is_prevented(
        &self,
        source: Option<GameObjectId>,
        source_object: Option<&TriggerEventObject>,
        source_is_spell: bool,
        target: Option<Target>,
        combat: bool,
    ) -> bool {
        let target_permanent = target.and_then(|target| match target {
            Target::Permanent(id) => self
                .battlefield
                .iter()
                .find(|permanent| permanent.card.id == id),
            Target::Player(_) | Target::Card(_) | Target::Spell(_) => None,
        });
        if target_permanent.is_some_and(|affected| {
            self.static_damage_is_prevented_on(
                affected,
                source,
                source_object,
                source_is_spell,
                target,
                combat,
            )
        }) {
            return true;
        }

        source
            .and_then(|source| {
                self.battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == source)
            })
            .filter(|affected| {
                target_permanent.is_none_or(|target| target.card.id != affected.card.id)
            })
            .is_some_and(|affected| {
                self.static_damage_is_prevented_on(
                    affected,
                    source,
                    source_object,
                    source_is_spell,
                    target,
                    combat,
                )
            })
    }

    fn static_damage_is_prevented_on(
        &self,
        affected: &Permanent,
        source: Option<GameObjectId>,
        source_object: Option<&TriggerEventObject>,
        source_is_spell: bool,
        target: Option<Target>,
        combat: bool,
    ) -> bool {
        self.visit_static_applied_effects(affected, |applied| {
            if matches!(applied.effect, AppliedEffectDef::PreventDamage(matcher)
            if self.static_damage_matcher_matches(
                matcher,
                applied.source,
                affected.card.id,
                source,
                source_object,
                source_is_spell,
                target,
                combat,
            )) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .is_break()
    }

    #[allow(clippy::too_many_arguments)]
    fn static_damage_matcher_matches(
        &self,
        matcher: DamageEventMatcherDef,
        effect_source: GameObjectId,
        affected: GameObjectId,
        source: Option<GameObjectId>,
        source_object: Option<&TriggerEventObject>,
        source_is_spell: bool,
        target: Option<Target>,
        combat: bool,
    ) -> bool {
        (matcher.kind == DamageKindDef::Any || combat)
            && match matcher.source {
                DamageSourceMatcherDef::Any => true,
                DamageSourceMatcherDef::AffectedObject => source == Some(affected),
                DamageSourceMatcherDef::Object(reference) => self
                    .static_object_reference(reference, effect_source)
                    .is_some_and(|expected| source == Some(expected)),
                DamageSourceMatcherDef::Except(reference) => self
                    .static_object_reference(reference, effect_source)
                    .is_some_and(|excluded| source != Some(excluded)),
                DamageSourceMatcherDef::Matching(predicate) => {
                    source_object.is_some_and(|source| {
                        self.trigger_object_matches(
                            predicate,
                            source,
                            effect_source,
                            source_is_spell,
                        )
                    })
                }
                DamageSourceMatcherDef::Group(group) => source.is_some_and(|source| {
                    self.damage_source_is_in_group(source, Self::relational_source_filter(group))
                }),
            }
            && match matcher.recipient {
                DamageRecipientMatcherDef::Any => target.is_some(),
                DamageRecipientMatcherDef::AffectedObject => {
                    target == Some(Target::Permanent(affected))
                }
                DamageRecipientMatcherDef::Recipients(recipients) => recipients
                    .object_reference()
                    .and_then(|reference| self.static_object_reference(reference, effect_source))
                    .is_some_and(|recipient| target == Some(Target::Permanent(recipient))),
                DamageRecipientMatcherDef::PlayerAndCreaturesControlledBy(_) => false,
            }
    }

    const fn relational_source_filter(group: DamageSourceGroupDef) -> RelationalSourceFilter {
        match group {
            DamageSourceGroupDef::CreaturesWithFlying => {
                RelationalSourceFilter::CreaturesWithFlying
            }
            DamageSourceGroupDef::AttackingCreaturesWithoutFlying => {
                RelationalSourceFilter::AttackingCreaturesWithoutFlying
            }
            DamageSourceGroupDef::Artifacts => RelationalSourceFilter::Artifacts,
            DamageSourceGroupDef::UnblockedCreatures => RelationalSourceFilter::UnblockedCreatures,
        }
    }

    /// Whether one source belongs to a named group. Membership is evaluated
    /// when damage would be dealt, so attacking and keyword state stay live.
    fn damage_source_is_in_group(
        &self,
        source: GameObjectId,
        group: RelationalSourceFilter,
    ) -> bool {
        let Some(permanent) = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == source)
        else {
            return false;
        };
        if group == super::RelationalSourceFilter::Artifacts {
            return self
                .permanent_types(permanent)
                .is_some_and(|types| types.contains(CardType::Artifact));
        }
        if !self
            .permanent_types(permanent)
            .is_some_and(|types| types.contains(CardType::Creature))
        {
            return false;
        }
        let flying = self.permanent_has_executable_keyword(permanent, KeywordAbility::Flying);
        match group {
            RelationalSourceFilter::CreaturesWithFlying => flying,
            RelationalSourceFilter::AttackingCreaturesWithoutFlying => {
                permanent.attacking && !flying
            }
            super::RelationalSourceFilter::UnblockedCreatures => {
                permanent.attacking
                    && !self
                        .battlefield
                        .iter()
                        .any(|blocker| blocker.blocking == Some(source))
            }
            // Not a creature question at all, so it is asked before the one
            // above rather than through it.
            super::RelationalSourceFilter::Artifacts => {
                unreachable!("handled before the type gate")
            }
        }
    }

    fn static_object_reference(
        &self,
        reference: ObjectRefDef,
        effect_source: GameObjectId,
    ) -> Option<GameObjectId> {
        match reference {
            ObjectRefDef::Source | ObjectRefDef::ResolvingObject => Some(effect_source),
            ObjectRefDef::AttachedToSource => {
                self.current_or_last_known_attached_host(effect_source)
            }
            ObjectRefDef::Binding(_) | ObjectRefDef::Target(_) | ObjectRefDef::TriggeringObject => {
                None
            }
        }
    }

    fn damage_source_is_spell(&self, source: GameObjectId) -> bool {
        self.stack
            .iter()
            .find(|object| object.id == source)
            .is_some_and(|object| object.kind == StackObjectKind::Spell)
            || matches!(
                self.retired_objects.get(&source),
                Some(RetiredObject::Stack(object)) if object.kind == StackObjectKind::Spell
            )
    }

    pub(super) fn damage_target_from(
        &mut self,
        source: Option<GameObjectId>,
        target: Option<Target>,
        amount: u16,
    ) -> u16 {
        self.damage_target_from_kind(source, target, amount, false)
    }

    /// Where damage actually lands. A permanent whose static effect redirects
    /// its controller's damage takes it instead, provided the source is in
    /// the group that effect names.
    fn redirected_damage_target(
        &self,
        source: Option<GameObjectId>,
        target: Option<Target>,
    ) -> Option<Target> {
        let Some(Target::Player(player)) = target else {
            return target;
        };
        let Some(source) = source else {
            return target;
        };
        if let Some(destination) = self
            .damage_redirects
            .iter()
            .find(|redirect| redirect.player == player && redirect.source == source)
            .map(|redirect| redirect.destination)
        {
            return Some(Target::Permanent(destination));
        }
        for candidate in &self.battlefield {
            if candidate.controller != player {
                continue;
            }
            let mut redirects = false;
            let _ = self.visit_static_applied_effects(candidate, |applied| {
                if let AppliedEffectDef::RedirectPlayerDamageToThis(group) = applied.effect
                    && self.damage_source_is_in_group(source, Self::relational_source_filter(group))
                {
                    redirects = true;
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            });
            if redirects {
                return Some(Target::Permanent(candidate.card.id));
            }
        }
        target
    }

    pub(super) fn damage_target_from_kind(
        &mut self,
        source: Option<GameObjectId>,
        target: Option<Target>,
        amount: u16,
        combat: bool,
    ) -> u16 {
        // CR 614.9: redirection applies before the damage is dealt, so the
        // preventions below all answer the permanent it lands on
        // rather than the player it was aimed at.
        let target = self.redirected_damage_target(source, target);
        let amount = self.apply_resolved_damage_prevention(source, target, amount, combat);
        if amount == 0 {
            return 0;
        }
        let source_object = source.and_then(|source| self.damage_source_event_object(source));
        let source_is_spell = source.is_some_and(|source| self.damage_source_is_spell(source));
        let source_colors = source.map_or([false; 5], |source| self.object_colors(source));
        if self.static_damage_is_prevented(
            source,
            source_object.as_ref(),
            source_is_spell,
            target,
            combat,
        ) || target.is_some_and(|target| match target {
            Target::Permanent(id) => self
                .battlefield
                .iter()
                .find(|permanent| permanent.card.id == id)
                .is_some_and(|permanent| self.is_protected_from_colors(permanent, source_colors)),
            Target::Player(_) | Target::Card(_) | Target::Spell(_) => false,
        }) {
            return 0;
        }
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
        if dealt_damage { amount } else { 0 }
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
