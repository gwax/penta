// Aggregation belongs to the committed move, independently of its origin zone.
impl Game {
    fn zone_group_context(
        &self,
        listener: &BattlefieldTriggerListener,
        events: &[CommittedTriggerEvent],
    ) -> TriggerContext {
        let mut context = TriggerContext::empty();
        context.amount = Some(
            events
                .iter()
                .filter(|candidate| self.groups_zone_changes(listener, candidate))
                .count()
                .try_into()
                .unwrap_or(i32::MAX),
        );
        context
    }

    fn groups_zone_changes(
        &self,
        listener: &BattlefieldTriggerListener,
        event: &CommittedTriggerEvent,
    ) -> bool {
        self.event_groups_zone_changes(
            listener.event,
            event,
            listener.capture.source.object,
            listener.capture.controller,
        )
    }

    fn event_groups_zone_changes(
        &self,
        definition: TriggerEventDef,
        committed: &CommittedTriggerEvent,
        source: GameObjectId,
        controller: PlayerId,
    ) -> bool {
        match definition {
            TriggerEventDef::ZoneChanged(matcher) if matcher.one_or_more => self
                .trigger_event_matches_for_controller(
                    definition,
                    committed,
                    source,
                    Some(controller),
                ),
            TriggerEventDef::While { event, .. } => {
                self.trigger_event_matches_for_controller(
                    definition,
                    committed,
                    source,
                    Some(controller),
                ) && self.event_groups_zone_changes(*event, committed, source, controller)
            }
            TriggerEventDef::AnyOf(events) => events
                .iter()
                .any(|event| self.event_groups_zone_changes(*event, committed, source, controller)),
            _ => false,
        }
    }
}
