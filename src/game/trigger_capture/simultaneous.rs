// Matching and counting share the same frozen event snapshots. The event
// publisher owns simultaneity; the listener owns filtering and aggregation.
enum SimultaneousOccurrence {
    Individual,
    Group(usize),
}

impl Game {
    fn simultaneous_occurrence(
        &self,
        listener: &BattlefieldTriggerListener,
        events: &[CommittedTriggerEvent],
        matched_groups: &mut Vec<(AbilitySourceRef, Option<u32>)>,
    ) -> Option<SimultaneousOccurrence> {
        let TriggerEventDef::Simultaneous(definition) = listener.event else {
            return Some(SimultaneousOccurrence::Individual);
        };
        let mut count = 0;
        let mut includes_required = definition.required_member.is_none();
        for event in events {
            if !self.simultaneous_member_matches(listener, event) {
                continue;
            }
            count += 1;
            if let Some(required) = definition.required_member {
                includes_required |= self.trigger_event_matches_for_controller(
                    *required,
                    event,
                    listener.capture.source.object,
                    Some(listener.capture.controller),
                );
            }
        }
        if !definition.accepts(count) || !includes_required {
            return None;
        }
        if !Self::groups_simultaneous_matches(listener) {
            return Some(SimultaneousOccurrence::Individual);
        }
        let key = (listener.capture.source, listener.installed);
        if matched_groups.contains(&key) {
            return None;
        }
        matched_groups.push(key);
        Some(SimultaneousOccurrence::Group(count))
    }

    fn simultaneous_member_matches(
        &self,
        listener: &BattlefieldTriggerListener,
        event: &CommittedTriggerEvent,
    ) -> bool {
        let TriggerEventDef::Simultaneous(definition) = listener.event else {
            return false;
        };
        let mut member = listener.clone();
        member.event = *definition.event;
        self.trigger_event_matches_for_controller(
            member.event,
            event,
            member.capture.source.object,
            Some(member.capture.controller),
        ) && self.modified_trigger_occurrences(&member, event, std::slice::from_ref(event)) > 0
    }

    fn contributing_trigger_events(
        &self,
        listener: &BattlefieldTriggerListener,
        event: &CommittedTriggerEvent,
        events: &[CommittedTriggerEvent],
    ) -> Vec<CommittedTriggerEvent> {
        if Self::groups_simultaneous_matches(listener) {
            events
                .iter()
                .filter(|candidate| self.simultaneous_member_matches(listener, candidate))
                .cloned()
                .collect()
        } else {
            vec![event.clone()]
        }
    }

    fn groups_simultaneous_matches(listener: &BattlefieldTriggerListener) -> bool {
        matches!(listener.event, TriggerEventDef::Simultaneous(definition)
            if definition.aggregation == crate::card::TriggerAggregationDef::Once)
    }

    fn simultaneous_group_context(count: usize) -> TriggerContext {
        let mut context = TriggerContext::empty();
        context.amount = Some(count.try_into().unwrap_or(i32::MAX));
        context
    }
}
