//! Seat-safe information accumulated until that player submits another move.

use crate::{Action, CardCatalog, DecisionObservation, Game, GameEvent, PlayerId};
use serde_json::{Value, json};

#[derive(Clone)]
enum Update {
    Event(GameEvent),
    AutomaticDecision {
        decision: DecisionObservation,
        action: Option<Box<Action>>,
    },
}

/// Seat-visible events and automatic decisions retained until that seat acts.
#[derive(Clone, Default)]
pub struct SessionUpdates([Vec<Update>; 2]);

impl SessionUpdates {
    /// Acknowledges prior updates when this seat submits an accepted command.
    pub fn clear(&mut self, seat: PlayerId) {
        self.0[seat.index()].clear();
    }

    /// Records only events and automatic decision information visible to each seat.
    pub fn record(
        &mut self,
        game: &Game,
        cursor: usize,
        decision: Option<(&DecisionObservation, &Action)>,
    ) {
        for seat in [PlayerId::One, PlayerId::Two] {
            if let Some((pending, action)) = decision
                && let Some(pending) = pending.for_viewer(seat)
            {
                let action = pending
                    .options_visible_to(seat)
                    .then(|| Box::new(action.clone()));
                self.0[seat.index()].push(Update::AutomaticDecision {
                    decision: pending,
                    action,
                });
            }
            self.0[seat.index()].extend(
                game.events_for_since(seat, cursor)
                    .into_iter()
                    .map(Update::Event),
            );
        }
    }

    /// Serializes the retained information for one seat without consuming it.
    #[must_use]
    pub fn json(&self, catalog: &CardCatalog, seat: PlayerId) -> Value {
        self.0[seat.index()].iter().filter_map(|update| match update {
            Update::Event(event) => crate::protocol::event_json(catalog, event),
            Update::AutomaticDecision { decision, action } => Some(json!({
                "type":"AutomaticDecision", "decision":crate::protocol::decision_json(catalog, decision),
                "action":action.as_deref().map(crate::protocol::action_json),
            })),
        }).collect::<Vec<_>>().into()
    }

    /// Labels retained automatic decisions for browser presentation.
    #[must_use]
    pub fn decision_labels(&self, catalog: &CardCatalog, seat: PlayerId) -> Vec<String> {
        self.0[seat.index()]
            .iter()
            .filter_map(|update| {
                let Update::AutomaticDecision { decision, .. } = update else {
                    return None;
                };
                let mut labels = vec![decision.prompt.clone()];
                for option in &decision.options {
                    labels.push(option.label.clone());
                    for (_, card) in option.card.iter().chain(option.members.iter()) {
                        if let Some(definition) =
                            card.card_definition().and_then(|id| catalog.get(id))
                        {
                            labels.push(definition.name.clone());
                        }
                    }
                    labels.extend(option.ability_text.iter().cloned());
                }
                labels.dedup();
                Some(format!("Automatic: {}", labels.join(" · ")))
            })
            .collect()
    }
}
