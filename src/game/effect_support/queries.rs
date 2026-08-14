//! Resolving an object query into the objects it matches.
//!
//! Split out of the parent module for the source-size budget.

#![allow(clippy::wildcard_imports)]

use super::*;

impl Game {
    /// Finds objects using only zone, relation, and effective-characteristic
    /// predicates. Unlike target enumeration, this does not apply hexproof,
    /// protection, or any other targeting restriction.
    pub(in crate::game) fn objects_matching_query(
        &self,
        query: ObjectQueryDef,
        evaluation_controller: PlayerId,
        source: GameObjectId,
        context: TriggerContext,
    ) -> Vec<Target> {
        self.objects_matching_query_with_prospective(
            query,
            evaluation_controller,
            source,
            context,
            None,
        )
    }

    pub(in crate::game) fn objects_matching_effect_query(
        &self,
        query: ObjectQueryDef,
        object: &StackObject,
        context: TriggerContext,
        scoped: ScopedEffect,
    ) -> Vec<Target> {
        self.objects_matching_query_with_context(
            query,
            object.controller,
            object.source.unwrap_or(object.id),
            context,
            None,
            Some((object, scoped)),
        )
    }

    pub(in crate::game) fn objects_matching_query_with_prospective(
        &self,
        query: ObjectQueryDef,
        evaluation_controller: PlayerId,
        source: GameObjectId,
        context: TriggerContext,
        prospective: Option<&Permanent>,
    ) -> Vec<Target> {
        self.objects_matching_query_with_context(
            query,
            evaluation_controller,
            source,
            context,
            prospective,
            None,
        )
    }

    fn objects_matching_query_with_context(
        &self,
        query: ObjectQueryDef,
        evaluation_controller: PlayerId,
        source: GameObjectId,
        context: TriggerContext,
        prospective: Option<&Permanent>,
        effect_context: Option<(&StackObject, ScopedEffect)>,
    ) -> Vec<Target> {
        let mut recipients = Vec::new();
        let result = self.visit_objects_matching_query_with_context(
            query,
            evaluation_controller,
            source,
            context,
            prospective,
            effect_context,
            |recipient| {
                recipients.push(recipient);
                ControlFlow::Continue(())
            },
        );
        debug_assert!(result.is_continue());
        recipients
    }

    pub(in crate::game) fn any_object_matches_query_with_prospective(
        &self,
        query: ObjectQueryDef,
        evaluation_controller: PlayerId,
        source: GameObjectId,
        context: TriggerContext,
        prospective: Option<&Permanent>,
    ) -> bool {
        self.visit_objects_matching_query_with_context(
            query,
            evaluation_controller,
            source,
            context,
            prospective,
            None,
            |_| ControlFlow::Break(()),
        )
        .is_break()
    }

    pub(in crate::game) fn visit_objects_matching_query_with_prospective(
        &self,
        query: ObjectQueryDef,
        evaluation_controller: PlayerId,
        source: GameObjectId,
        context: TriggerContext,
        prospective: Option<&Permanent>,
        visitor: impl FnMut(Target) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        self.visit_objects_matching_query_with_context(
            query,
            evaluation_controller,
            source,
            context,
            prospective,
            None,
            visitor,
        )
    }

    fn player_matches_set(
        &self,
        candidate: PlayerId,
        players: PlayerSetDef,
        evaluation_controller: PlayerId,
        context: TriggerContext,
        effect_context: Option<(&StackObject, ScopedEffect)>,
    ) -> bool {
        match players {
            PlayerSetDef::All => true,
            PlayerSetDef::Related(relation) => {
                self.player_relation_matches(candidate, relation, evaluation_controller, context)
            }
            PlayerSetDef::One(PlayerRefDef::EffectController) => candidate == evaluation_controller,
            PlayerSetDef::One(PlayerRefDef::EventPlayer) => context.event_player == Some(candidate),
            PlayerSetDef::One(reference) => effect_context.is_some_and(|(object, scoped)| {
                self.player_reference(reference, object, context, scoped) == Some(candidate)
            }),
        }
    }

    pub(in crate::game) fn query_player_constraints_match(
        &self,
        controller: Option<PlayerId>,
        owner: PlayerId,
        query: ObjectQueryDef,
        evaluation_controller: PlayerId,
        context: TriggerContext,
        effect_context: Option<(&StackObject, ScopedEffect)>,
    ) -> bool {
        query.related_player.is_none_or(|players| {
            self.player_matches_set(
                controller.unwrap_or(owner),
                players,
                evaluation_controller,
                context,
                effect_context,
            )
        }) && query.controller.is_none_or(|players| {
            controller.is_some_and(|candidate| {
                self.player_matches_set(
                    candidate,
                    players,
                    evaluation_controller,
                    context,
                    effect_context,
                )
            })
        }) && query.owner.is_none_or(|players| {
            self.player_matches_set(
                owner,
                players,
                evaluation_controller,
                context,
                effect_context,
            )
        })
    }

    fn visit_objects_matching_query_with_context(
        &self,
        query: ObjectQueryDef,
        evaluation_controller: PlayerId,
        source: GameObjectId,
        context: TriggerContext,
        prospective: Option<&Permanent>,
        effect_context: Option<(&StackObject, ScopedEffect)>,
        mut visitor: impl FnMut(Target) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        if query.zones.contains(&ZoneKind::Battlefield) {
            for permanent in &self.battlefield {
                if !self.query_player_constraints_match(
                    Some(permanent.controller),
                    permanent.card.owner,
                    query,
                    evaluation_controller,
                    context,
                    effect_context,
                ) {
                    continue;
                }
                let characteristics = prospective.map_or_else(
                    || self.trigger_event_object(permanent),
                    |prospective| {
                        self.trigger_event_object_with_prospective(permanent, prospective)
                    },
                );
                if self.trigger_object_matches(query.object, &characteristics, source, false)
                    && visitor(Target::Permanent(permanent.card.id)).is_break()
                {
                    return ControlFlow::Break(());
                }
            }
        }
        if query.zones.contains(&ZoneKind::Stack) {
            for candidate in self.stack.iter() {
                if candidate.kind != StackObjectKind::Spell
                    || !self.query_player_constraints_match(
                        Some(candidate.controller),
                        candidate.card.owner,
                        query,
                        evaluation_controller,
                        context,
                        effect_context,
                    )
                {
                    continue;
                }
                let Some(characteristics) = self.stack_trigger_event_object(candidate) else {
                    continue;
                };
                if self.trigger_object_matches(query.object, &characteristics, source, true)
                    && visitor(Target::Spell(candidate.id)).is_break()
                {
                    return ControlFlow::Break(());
                }
            }
        }
        // The same card zones the target enumerator understands. Without this
        // a sweep over graveyards matched nothing and the clause was inert.
        for zone in [
            ZoneKind::Library,
            ZoneKind::Hand,
            ZoneKind::Graveyard,
            ZoneKind::Exile,
            ZoneKind::Command,
        ] {
            if !query.zones.contains(&zone) {
                continue;
            }
            for card in self.cards_in_zone(zone) {
                if self.query_player_constraints_match(
                    None,
                    card.owner,
                    query,
                    evaluation_controller,
                    context,
                    effect_context,
                ) && self.card_object_matches(query.object, card, zone, source)
                    && visitor(Target::Card(card.id)).is_break()
                {
                    return ControlFlow::Break(());
                }
            }
        }
        ControlFlow::Continue(())
    }
}
