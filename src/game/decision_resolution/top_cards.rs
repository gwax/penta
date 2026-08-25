//! Answering a look at the top of a library.
//!
//! Split out of the parent module for the source-size budget: the three
//! shapes a look can take -- split the group in two, one card for each card
//! type, one card for each destination -- all place what was chosen and pick
//! up whatever the clause said next.

#![allow(clippy::wildcard_imports)]

use super::*;

impl Game {
    pub(super) fn resolve_top_card_continuation(
        &mut self,
        continuation: DecisionContinuation,
        offered: &[DecisionOption],
        options: &[u32],
    ) {
        match continuation {
            DecisionContinuation::TopCardSelection {
                player,
                revealed,
                selection,
                object,
                context,
                effect,
            } => {
                // Walked in the order the cards were named rather than the
                // order they were offered, because an arrangement reads that
                // sequence back below. Membership is all the ordinary path
                // asks of it, so nothing else notices.
                let selected = options
                    .iter()
                    .filter_map(|chosen| offered.iter().find(|option| option.id == *chosen))
                    .filter_map(|option| option.card.map(|(card, _)| card))
                    .collect::<Vec<_>>();
                let (mut chosen, rest): (Vec<_>, Vec<_>) = revealed
                    .into_iter()
                    .partition(|card| selected.contains(&card.id));
                // "Put them back in any order": the arrangement is the order
                // the cards were named, so it has to survive the partition,
                // which otherwise reports them in library order.
                if selection.selected_order_follows_choice {
                    chosen.sort_by_key(|card| {
                        selected
                            .iter()
                            .position(|id| *id == card.id)
                            .unwrap_or(usize::MAX)
                    });
                }
                let hider = object.source.unwrap_or(object.id);
                let matched = self.selected_card_totals(&chosen, selection.counted, hider);
                self.finish_top_card_selection_from(player, chosen, rest, selection, Some(hider));
                if let Some(then) = selection.then {
                    let mut context = context;
                    context.matched_count = Some(matched.0);
                    context.matched_mana_value = Some(matched.1);
                    self.resolve_nested_effect_before_later(
                        effect.with_effect(*then),
                        &object,
                        context,
                    );
                }
            }
            DecisionContinuation::DistributedTopCardSelection {
                mut progress,
                destinations,
                object,
                context,
                effect,
            } => {
                // Exactly one card, and it leaves the group: what is left is
                // what the next destination gets to choose from.
                let chosen = options
                    .iter()
                    .filter_map(|chosen| offered.iter().find(|option| option.id == *chosen))
                    .find_map(|option| option.card.map(|(card, _)| card));
                let destination = destinations.get(progress.next_destination).copied();
                if let (Some(chosen), Some(destination)) = (chosen, destination)
                    && let Some(index) =
                        progress.remaining.iter().position(|card| card.id == chosen)
                {
                    let card = progress.remaining.remove(index);
                    self.place_distributed_card(progress.player, card, destination, &object);
                }
                progress.next_destination += 1;
                self.queue_distributed_selection(progress, destinations, &object, context, effect);
            }
            DecisionContinuation::TypedTopCardSelection {
                mut progress,
                selection,
                object,
                context,
                effect,
            } => {
                // At most one card, and declining is an ordinary answer: the
                // clause says "you may" for every type it asks about.
                let taken = options
                    .iter()
                    .filter_map(|chosen| offered.iter().find(|option| option.id == *chosen))
                    .filter_map(|option| option.card.map(|(card, _)| card))
                    .collect::<Vec<_>>();
                let (mut chosen, rest): (Vec<_>, Vec<_>) = progress
                    .revealed
                    .into_iter()
                    .partition(|card| taken.contains(&card.id));
                progress.taken.append(&mut chosen);
                progress.revealed = rest;
                // The type just answered is done whether or not it took
                // anything, so the next question is about the next one.
                progress.next_type += 1;
                self.queue_typed_selection(progress, selection, &object, context, effect);
            }
            _ => unreachable!("only top-of-library looks reach this resolver"),
        }
    }
}
