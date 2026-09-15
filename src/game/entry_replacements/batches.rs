impl Game {
    /// Raises one permanent's arrival, or holds it back until the rest of
    /// its batch has arrived. Everything a batch does before this point is
    /// per permanent -- replacements are applied to each one on its own (CR
    /// 614.12) -- and only what watches the arrivals waits.
    fn capture_entry_event(&mut self, event: CommittedTriggerEvent) {
        if let Some(batch) = self.entry_event_batch.as_mut() {
            batch.push(event);
            return;
        }
        self.capture_battlefield_triggers(&event);
    }

    /// Prepare every entry against the pre-entry battlefield, including any
    /// replacement choices, before committing and publishing the group.
    pub(in crate::game) fn entering_together(&mut self, enter: impl FnOnce(&mut Self)) {
        if self.ready_entry_batch.is_some() {
            enter(self);
            return;
        }
        self.ready_entry_batch = Some(Vec::new());
        self.building_entry_batch = true;
        enter(self);
        self.building_entry_batch = false;
        self.finish_ready_entry_batch();
    }

    fn finish_ready_entry_batch(&mut self) {
        if self.building_entry_batch
            || !self.pending_events.is_empty()
            || !self.pending_decisions.is_empty()
        {
            return;
        }
        let Some(ready) = self.ready_entry_batch.take() else {
            return;
        };
        let token_creations = std::mem::take(&mut self.deferred_token_creations);
        self.capture_entries_together(|game| {
            for pending in ready {
                game.commit_pending_event(pending);
            }
            for (controller, tokens) in token_creations {
                if let Some(event) = game.tokens_created_event(controller, &tokens) {
                    game.capture_entry_event(event);
                }
            }
        });
    }

    fn capture_entries_together(&mut self, enter: impl FnOnce(&mut Self)) {
        if self.entry_event_batch.is_some() {
            enter(self);
            return;
        }
        self.entry_event_batch = Some(Vec::new());
        enter(self);
        let mut batch = self.entry_event_batch.take().unwrap_or_default();
        // All entrants and their continuous effects exist when this atomic
        // arrival is observed, including effects from the last entrant.
        for event in &mut batch {
            if let CommittedTriggerEvent::ZoneChanged {
                after: Some(after),
                to: ZoneKind::Battlefield,
                ..
            } = event
                && let Some(permanent) = self.battlefield.iter().find(|p| p.card.id == after.id)
            {
                *after = self.targeting_event_object(permanent);
            }
        }
        let listeners = self.battlefield_trigger_listeners();
        self.capture_battlefield_trigger_batch_from_snapshot(&listeners, &batch);
    }
}
