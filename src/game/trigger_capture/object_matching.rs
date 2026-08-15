impl Game {
    /// Whether `object` satisfies `predicate`. `source` is the ability's own
    /// object, which is what a controller relation is measured against.
    /// The predicates comparing a stat against a value read off the ability's
    /// own source. They share a shape, so they share a body.
    fn computed_stat_matches(
        &self,
        predicate: ObjectPredicateDef,
        object: &TriggerEventObject,
        source: GameObjectId,
    ) -> bool {
        let (value, stat, greater) = match predicate {
            ObjectPredicateDef::ToughnessLessThan(value) => (value, object.toughness, false),
            ObjectPredicateDef::PowerGreaterThan(value) => (value, object.power, true),
            ObjectPredicateDef::PowerLessThan(value) => (value, object.power, false),
            ObjectPredicateDef::ToughnessGreaterThan(value) => (value, object.toughness, true),
            _ => return false,
        };
        self.value_from_source(value, source)
            .zip(stat)
            .is_some_and(|(limit, stat)| {
                if greater {
                    i32::from(stat) > limit
                } else {
                    i32::from(stat) < limit
                }
            })
    }

    /// The predicates answered by looking at the battlefield rather than at
    /// the object's own recorded characteristics.
    fn battlefield_relationship_matches(
        &self,
        predicate: ObjectPredicateDef,
        object: &TriggerEventObject,
        source: GameObjectId,
        controller: Option<PlayerId>,
    ) -> bool {
        match predicate {
            ObjectPredicateDef::HasNonManaActivatedAbility => self
                .battlefield
                .iter()
                .find(|permanent| permanent.card.id == object.id)
                .is_some_and(|permanent| self.has_nonmana_activated_ability(permanent)),
            ObjectPredicateDef::AttachedToSource => self
                .battlefield
                .iter()
                .find(|permanent| permanent.card.id == source)
                .and_then(|permanent| permanent.attached_to)
                .is_some_and(|host| host == object.id),
            // Read from the source: the Wall knows what it blocked, and the
            // attacker's own record does not name its blockers.
            // Last-known, because a creature that died in combat still knows
            // what it had blocked and its death trigger has to read it.
            ObjectPredicateDef::BlockedBySource => self
                .current_or_last_known_blocking(source)
                .is_some_and(|attacker| attacker == object.id),
            // The other direction, read from the candidate: a blocker records
            // what it blocked, so this one needs no lookup on the source.
            ObjectPredicateDef::BlockingSource => self
                .battlefield
                .iter()
                .find(|candidate| candidate.card.id == object.id)
                .and_then(|candidate| candidate.blocking)
                .is_some_and(|attacker| attacker == source),
            ObjectPredicateDef::Enchanted => self.battlefield.iter().any(|candidate| {
                candidate.attached_to == Some(object.id) && self.is_aura_permanent(candidate)
            }),
            // The Aura's own side of the question: what is it on?
            ObjectPredicateDef::AttachedTo(predicate) => self
                .battlefield
                .iter()
                .find(|candidate| candidate.card.id == object.id)
                .and_then(|candidate| candidate.attached_to)
                .and_then(|host| {
                    self.battlefield
                        .iter()
                        .find(|candidate| candidate.card.id == host)
                })
                .is_some_and(|host| {
                    self.trigger_object_matches_for_controller(
                        *predicate,
                        &self.trigger_event_object(host),
                        source,
                        false,
                        controller,
                    )
                }),
            _ => unreachable!("only the battlefield-reading predicates arrive here"),
        }
    }

    pub(super) fn trigger_object_matches(
        &self,
        predicate: ObjectPredicateDef,
        object: &TriggerEventObject,
        source: GameObjectId,
        is_spell: bool,
    ) -> bool {
        self.trigger_object_matches_for_controller(
            predicate,
            object,
            source,
            is_spell,
            self.controller_of_object(source),
        )
    }

    fn trigger_object_matches_for_controller(
        &self,
        predicate: ObjectPredicateDef,
        object: &TriggerEventObject,
        source: GameObjectId,
        is_spell: bool,
        controller: Option<PlayerId>,
    ) -> bool {
        match predicate {
            ObjectPredicateDef::Any => true,
            ObjectPredicateDef::Source => object.id == source,
            ObjectPredicateDef::Token => object.token,
            ObjectPredicateDef::HasType(card_type) => object.types.contains(card_type),
            ObjectPredicateDef::HasAnyBasicLandType(land_types) => {
                object.types.contains(CardType::Land)
                    && land_types
                        .iter()
                        .any(|land_type| object.subtypes.contains(&land_type.subtype()))
            }
            ObjectPredicateDef::Spell => is_spell,
            ObjectPredicateDef::NoncreatureSpell => {
                is_spell && !object.types.contains(CardType::Creature)
            }
            ObjectPredicateDef::Color(color) => color
                .color_index()
                .is_some_and(|index| object.colors[index]),
            ObjectPredicateDef::ColorCount(count) => {
                object.colors.iter().filter(|present| **present).count() == usize::from(count)
            }
            ObjectPredicateDef::Subtype(subtype) => object.subtypes.contains(&subtype),
            ObjectPredicateDef::ManaValueAtMost(limit) => object.mana_value <= u16::from(limit),
            ObjectPredicateDef::ManaValueEqualTo(value) => self
                .value_from_source(value, source)
                .is_some_and(|value| value == i32::from(object.mana_value)),
            ObjectPredicateDef::ManaValueAtMostValue(value) => self
                .value_from_source(value, source)
                .is_some_and(|value| i32::from(object.mana_value) <= value),
            ObjectPredicateDef::PowerAtLeast(minimum) => {
                object.power.is_some_and(|power| power >= minimum)
            }
            ObjectPredicateDef::PowerExactly(exact) => object.power == Some(exact),
            ObjectPredicateDef::ToughnessExactly(exact) => object.toughness == Some(exact),
            ObjectPredicateDef::ToughnessLessThan(_)
            | ObjectPredicateDef::PowerGreaterThan(_)
            | ObjectPredicateDef::PowerLessThan(_)
            | ObjectPredicateDef::ToughnessGreaterThan(_) => {
                self.computed_stat_matches(predicate, object, source)
            }
            ObjectPredicateDef::Supertype(supertype) => object.supertypes[supertype.index()],
            // Read from the definition rather than the object: what matters
            // is where the card was first printed, not what it has become.
            ObjectPredicateDef::DebutSet(set) => self
                .object_debut_set(object.id)
                .is_some_and(|debut| debut == set),
            ObjectPredicateDef::AttackingOrBlocking => object.attacking_or_blocking,
            ObjectPredicateDef::SharesNameWithSource => {
                let name = self.object_card_name(object.id);
                name.is_some() && name == self.object_card_name(source)
            }
            ObjectPredicateDef::HasKeyword(keyword) => keyword
                .simple_index()
                .is_some_and(|index| object.keywords & (1 << index) != 0),
            // Counters are permanent state rather than a characteristic, so
            // reading them live cannot feed back into the layer being
            // computed the way a keyword or a stat could.
            ObjectPredicateDef::HasCounter(kind) => {
                self.current_or_last_known_counters(object.id, kind) > 0
            }
            ObjectPredicateDef::ControlledBy(relation) => controller.is_some_and(|controller| {
                self.player_relation_matches(
                    object.controller,
                    relation,
                    controller,
                    TriggerContext::empty(),
                )
            }),
            ObjectPredicateDef::Attacking
            | ObjectPredicateDef::AttackedThisTurn
            | ObjectPredicateDef::AttackedDuringControllersLastTurn
            | ObjectPredicateDef::Blocking => combat_state_matches(predicate, object),
            ObjectPredicateDef::AttachedToSource => self
                .current_or_last_known_attached_host(source)
                .is_some_and(|host| host == object.id),
            ObjectPredicateDef::HasNonManaActivatedAbility
            | ObjectPredicateDef::BlockedBySource
            | ObjectPredicateDef::BlockingSource
            | ObjectPredicateDef::Enchanted
            | ObjectPredicateDef::AttachedTo(_) => {
                self.battlefield_relationship_matches(predicate, object, source, controller)
            }
            ObjectPredicateDef::Tapped => object.tapped,
            ObjectPredicateDef::All(predicates) => predicates.iter().all(|predicate| {
                self.trigger_object_matches_for_controller(
                    *predicate, object, source, is_spell, controller,
                )
            }),
            ObjectPredicateDef::AnyOf(predicates) => predicates.iter().any(|predicate| {
                self.trigger_object_matches_for_controller(
                    *predicate, object, source, is_spell, controller,
                )
            }),
            ObjectPredicateDef::Not(predicate) => !self.trigger_object_matches_for_controller(
                *predicate, object, source, is_spell, controller,
            ),
            ObjectPredicateDef::Special(_) => false,
        }
    }

    pub(super) fn player_relation_matches(
        &self,
        player: PlayerId,
        relation: PlayerRelation,
        controller: PlayerId,
        context: TriggerContext,
    ) -> bool {
        match relation {
            PlayerRelation::Any => true,
            PlayerRelation::You => player == controller,
            PlayerRelation::NotYou => player != controller,
            PlayerRelation::Opponent => player == controller.opponent(),
            PlayerRelation::ActivePlayer => player == self.active_player,
            PlayerRelation::NonactivePlayer => player == self.active_player.opponent(),
            PlayerRelation::EventPlayer => context.event_player == Some(player),
            // Both of these live on the ability's source, which this does not
            // have. The triggers that name them resolve the relation where
            // the source is known.
            PlayerRelation::ChosenPlayer | PlayerRelation::ControllerOfAttachedPermanent => false,
        }
    }

    /// Whoever controls what this permanent is attached to. An Aura that has
    /// come loose is attached to nothing and so matches nobody.
    pub(super) fn attached_host_controller_of(&self, source: GameObjectId) -> Option<PlayerId> {
        self.current_or_last_known_attached_host(source)
            .and_then(|host| self.current_or_last_known_controller(host))
    }

    /// The player a permanent chose as it entered.
    pub(super) fn chosen_player_of(&self, source: GameObjectId) -> Option<PlayerId> {
        self.battlefield
            .iter()
            .find(|permanent| permanent.card.id == source)
            .and_then(|permanent| permanent.chosen_player)
    }
}

/// Combat facts about a creature, all read straight off the snapshot.
///
/// They are grouped because they answer four versions of one question and
/// each is worthless for anything that is not a creature: an artifact is
/// never "attacking", whatever else is true of it.
fn combat_state_matches(predicate: ObjectPredicateDef, object: &TriggerEventObject) -> bool {
    object.types.contains(CardType::Creature)
        && match predicate {
            ObjectPredicateDef::Attacking => object.attacking,
            // Still attacking is not the question: this asks whether the
            // creature attacked at any point this turn, which is what an
            // end-step check has to read once combat is over.
            ObjectPredicateDef::AttackedThisTurn => object.attacked_this_turn,
            ObjectPredicateDef::AttackedDuringControllersLastTurn => {
                object.attacked_during_controllers_last_turn
            }
            ObjectPredicateDef::Blocking => object.attacking_or_blocking && !object.attacking,
            _ => false,
        }
}
