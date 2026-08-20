// Resolving the object and player references an effect names, from the
// resolving object's own context: target slots, bindings, player sets, and
// the recipients and object sets those add up to.
//
// Split out of `effect_support.rs` only to keep one file readable; these are
// ordinary members of the same `impl Game`. The paths and imports are the
// parent module's.

impl Game {
    fn raw_target_reference(
        slot: TargetIndex,
        object: &StackObject,
        scoped: ScopedEffect,
    ) -> Option<Target> {
        Self::chosen_targets(object, scoped.target_slot(slot)).next()
    }

    fn object_reference_target(
        &self,
        reference: ObjectRefDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Option<Target> {
        match reference {
            // An ability activated from the graveyard or from hand has a
            // card, not a permanent, as its source, and "return this card to
            // your hand" has to name it as one. A source that is on the
            // battlefield, or that has left every zone, still answers as a
            // permanent: that is the last-known information every
            // "sacrifice this" clause reads after the thing is already gone.
            ObjectRefDef::Source => object.source.map(|source| {
                if self
                    .battlefield
                    .iter()
                    .any(|permanent| permanent.card.id == source)
                {
                    return Target::Permanent(source);
                }
                match self.card_in_nonbattlefield_zone(source) {
                    Some(_) => Target::Card(source),
                    None => Target::Permanent(source),
                }
            }),
            ObjectRefDef::ResolvingObject => self.live_object_target(object.id),
            ObjectRefDef::SourceOfTargetedStackObject(target) => self
                .targeted_stack_object_source(target, object, scoped)
                .map(Target::Permanent),
            ObjectRefDef::Binding(binding) => context.single_object(binding),
            ObjectRefDef::AttachedToSource => object
                .source
                .and_then(|source| self.current_or_last_known_attached_host(source))
                .map(Target::Permanent),
            ObjectRefDef::Target(target) => {
                let slot = scoped.target_slot(target);
                Self::raw_target_reference(target, object, scoped)
                    .filter(|target| !matches!(target, Target::Player(_)))
                    .filter(|target| self.stack_ability_target_is_legal(object, slot, *target))
            }
            ObjectRefDef::TriggeringObject => context
                .trigger
                .object
                .and_then(|triggering| self.live_object_target(triggering)),
        }
    }

    fn object_reference_id(
        &self,
        reference: ObjectRefDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Option<GameObjectId> {
        match reference {
            ObjectRefDef::Source => object.source,
            ObjectRefDef::ResolvingObject => Some(object.id),
            ObjectRefDef::Binding(binding) => {
                context
                    .single_object(binding)
                    .and_then(|target| match target {
                        Target::Card(id) | Target::Permanent(id) | Target::Spell(id) => Some(id),
                        Target::Player(_) => None,
                    })
            }
            ObjectRefDef::AttachedToSource => object
                .source
                .and_then(|source| self.current_or_last_known_attached_host(source)),
            ObjectRefDef::Target(target) => {
                let slot = scoped.target_slot(target);
                Self::raw_target_reference(target, object, scoped)
                    .filter(|target| self.stack_ability_target_is_legal(object, slot, *target))
                    .and_then(|target| match target {
                        Target::Card(id) | Target::Permanent(id) | Target::Spell(id) => Some(id),
                        Target::Player(_) => None,
                    })
            }
            ObjectRefDef::SourceOfTargetedStackObject(target) => {
                self.targeted_stack_object_source(target, object, scoped)
            }
            ObjectRefDef::TriggeringObject => context.trigger.object,
        }
    }

    /// The permanent a targeted stack ability came from. Read after the
    /// ability has left the stack -- which is when "destroy that permanent"
    /// asks -- so the retired record is the one that answers, and a targeted
    /// spell has no such source at all.
    fn targeted_stack_object_source(
        &self,
        target: crate::TargetIndex,
        object: &StackObject,
        scoped: ScopedEffect,
    ) -> Option<GameObjectId> {
        let Some(Target::Spell(id)) = Self::raw_target_reference(target, object, scoped) else {
            return None;
        };
        let source = self
            .stack
            .iter()
            .find(|candidate| candidate.id == id)
            .map_or_else(|| self.retired_stack_object_source(id), |stack| stack.source)?;
        self.battlefield
            .iter()
            .any(|permanent| permanent.card.id == source)
            .then_some(source)
    }


    pub(in crate::game) fn player_reference(
        &self,
        reference: PlayerRefDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Option<PlayerId> {
        match reference {
            PlayerRefDef::EffectController => Some(object.controller),
            PlayerRefDef::Opponent => Some(object.controller.opponent()),
            PlayerRefDef::EventPlayer => context.trigger.event_player,
            PlayerRefDef::Target(target) => {
                let slot = scoped.target_slot(target);
                Self::chosen_targets(object, slot)
                    .find(|target| self.stack_ability_target_is_legal(object, slot, *target))
                    .and_then(|target| match target {
                        Target::Player(player) => Some(player),
                        Target::Card(_) | Target::Permanent(_) | Target::Spell(_) => None,
                    })
            }
            // A direct object recipient still checks whether its target is
            // legal. Derived identity is different: a later instruction in
            // the same resolving effect may ask who controlled or owned an
            // object that an earlier instruction already moved. Preserve the
            // announced target here and answer from last-known information.
            PlayerRefDef::ControllerOf(ObjectRefDef::Target(target)) => {
                Self::raw_target_reference(target, object, scoped).and_then(|target| match target {
                    Target::Player(player) => Some(player),
                    Target::Card(id) | Target::Permanent(id) | Target::Spell(id) => {
                        self.current_or_last_known_controller(id)
                    }
                })
            }
            PlayerRefDef::OwnerOf(ObjectRefDef::Target(target)) => {
                Self::raw_target_reference(target, object, scoped).and_then(|target| match target {
                    Target::Card(id) | Target::Permanent(id) | Target::Spell(id) => {
                        self.current_or_last_known_owner(id)
                    }
                    Target::Player(_) => None,
                })
            }
            PlayerRefDef::ControllerOf(ObjectRefDef::TriggeringObject) => context
                .trigger
                .object
                .and_then(|triggering| self.current_or_last_known_controller(triggering))
                .or(context.trigger.object_controller),
            PlayerRefDef::ControllerOf(reference) => self
                .object_reference_id(reference, object, context, scoped)
                .and_then(|referenced| self.current_or_last_known_controller(referenced)),
            PlayerRefDef::OwnerOf(reference) => self
                .object_reference_id(reference, object, context, scoped)
                .and_then(|referenced| self.current_or_last_known_owner(referenced)),
        }
    }

    fn players_in_set(
        &self,
        players: PlayerSetDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Vec<PlayerId> {
        match players {
            PlayerSetDef::All => vec![object.controller, object.controller.opponent()],
            PlayerSetDef::One(reference) => self
                .player_reference(reference, object, context, scoped)
                .into_iter()
                .collect(),
            PlayerSetDef::LegalTargets(target) => {
                let slot = scoped.target_slot(target);
                Self::chosen_targets(object, slot)
                    .filter(|target| self.stack_ability_target_is_legal(object, slot, *target))
                    .filter_map(|target| match target {
                        Target::Player(player) => Some(player),
                        Target::Card(_) | Target::Permanent(_) | Target::Spell(_) => None,
                    })
                    .collect()
            }
            PlayerSetDef::Related(relation) => [object.controller, object.controller.opponent()]
                .into_iter()
                .filter(|candidate| {
                    self.player_relation_matches(
                        *candidate,
                        relation,
                        object.controller,
                        context.trigger,
                    )
                })
                .collect(),
        }
    }

    pub(super) fn effect_object_reference_id(
        &self,
        reference: ObjectRefDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Option<GameObjectId> {
        self.object_reference_id(reference, object, context, scoped)
    }

    pub(super) fn effect_player_reference(
        &self,
        reference: PlayerRefDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Option<PlayerId> {
        self.player_reference(reference, object, context, scoped)
    }

    pub(super) fn effect_players(
        &self,
        players: PlayerSetDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Vec<PlayerId> {
        self.players_in_set(players, object, context, scoped)
    }

    fn objects_sharing_name_with_reference(
        &self,
        reference: ObjectRefDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Vec<Target> {
        if let ObjectRefDef::Target(target) = reference {
            return self.objects_sharing_name_with_target(scoped.target_slot(target), object);
        }
        let Some(name) = self
            .object_reference_id(reference, object, context, scoped)
            .and_then(|referenced| self.object_card_name(referenced))
        else {
            return Vec::new();
        };
        self.battlefield
            .iter()
            .filter(|permanent| self.permanent_card_name(permanent.card.id) == Some(name))
            .map(|permanent| Target::Permanent(permanent.card.id))
            .collect()
    }

    pub(super) fn effect_recipients(
        &self,
        recipient: EffectRecipientDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Vec<Target> {
        match recipient.0 {
            EffectRecipientSetDef::LegalTargets(target) => {
                let slot = scoped.target_slot(target);
                Self::chosen_targets(object, slot)
                    .filter(|target| self.stack_ability_target_is_legal(object, slot, *target))
                    .collect()
            }
            EffectRecipientSetDef::Objects(objects) => {
                self.effect_objects(objects, object, context, scoped)
            }
            EffectRecipientSetDef::Players(players) => self
                .players_in_set(players, object, context, scoped)
                .into_iter()
                .map(Target::Player)
                .collect(),
        }
    }

    pub(super) fn effect_objects(
        &self,
        objects: ObjectSetDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> Vec<Target> {
        match objects {
            ObjectSetDef::One(reference) => self
                .object_reference_target(reference, object, context, scoped)
                .into_iter()
                .collect(),
            ObjectSetDef::LegalTargets(target) => {
                let slot = scoped.target_slot(target);
                Self::chosen_targets(object, slot)
                    .filter(|target| self.stack_ability_target_is_legal(object, slot, *target))
                    .filter(|target| !matches!(target, Target::Player(_)))
                    .collect()
            }
            ObjectSetDef::Binding(binding) => context.object_group(binding).to_vec(),
            ObjectSetDef::MatchingBinding {
                binding,
                object: predicate,
            } => context
                .object_group(binding)
                .iter()
                .copied()
                .filter(|bound| {
                    let (Target::Card(id) | Target::Permanent(id) | Target::Spell(id)) = bound
                    else {
                        return false;
                    };
                    self.card_in_nonbattlefield_zone(*id).is_some_and(|(zone, card)| {
                        self.card_object_matches(predicate, card, zone, object.id)
                    })
                })
                .collect(),
            ObjectSetDef::Query(query) => {
                self.objects_matching_effect_query(query, object, context, scoped)
            }
            ObjectSetDef::SharingNameWith(reference) => {
                self.objects_sharing_name_with_reference(reference, object, context, scoped)
            }
            ObjectSetDef::SharingNameWithBinding {
                binding,
                player,
                zone,
            } => {
                let Some(player) = self.player_reference(player, object, context, scoped) else {
                    return Vec::new();
                };
                let names: Vec<&str> = context
                    .object_group(binding)
                    .iter()
                    .filter_map(|bound| match bound {
                        Target::Card(id) | Target::Permanent(id) | Target::Spell(id) => {
                            self.object_card_name(*id)
                        }
                        Target::Player(_) => None,
                    })
                    .collect();
                let mut found = Vec::new();
                for name in names {
                    for card in self.cards_named_in_zone(player, zone, name) {
                        if !found.contains(&card) {
                            found.push(card);
                        }
                    }
                }
                found
            }
            // The back of the vector is the newest card, which is the one on
            // top of the pile.
            ObjectSetDef::TopOfGraveyardMatching {
                player,
                object: predicate,
            } => {
                let Some(player) = self.player_reference(player, object, context, scoped)
                else {
                    return Vec::new();
                };
                let source = object.source.unwrap_or(object.id);
                self.players[player.index()]
                    .graveyard
                    .iter()
                    .rev()
                    .find(|card| {
                        self.card_object_matches(predicate, card, ZoneKind::Graveyard, source)
                    })
                    .map(|card| Target::Card(card.id))
                    .into_iter()
                    .collect()
            }
            // The front of the vector is the oldest card, which is the one at
            // the bottom of the pile.
            ObjectSetDef::BottomOfGraveyard(player) => self
                .player_reference(player, object, context, scoped)
                .and_then(|player| self.players[player.index()].graveyard.first())
                .map(|card| Target::Card(card.id))
                .into_iter()
                .collect(),
        }
    }
}
