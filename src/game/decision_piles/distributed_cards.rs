// Sending each card of one look somewhere different.
//
// Split out of `decision_piles.rs` for the source-size budget along the seam
// the question has: the file next door splits an inspected group in two, and
// this one hands out its cards one destination at a time. Included textually,
// so the imports here are that module's.

impl Game {
    /// "Look at the top three cards of your library": the cards leave the
    /// library for the length of the distribution, so a card taken by an
    /// earlier destination is not there for the next question.
    pub(super) fn queue_distributed_selection_start(
        &mut self,
        player: PlayerId,
        count: usize,
        destinations: &'static [SelectionDestinationDef],
        object: &StackObject,
        context: EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        let looked = self.take_top_of_library(player, count);
        if looked.is_empty() {
            return;
        }
        self.queue_distributed_selection(
            DistributedSelectionProgress {
                player,
                remaining: looked,
                next_destination: 0,
            },
            destinations,
            object,
            context,
            scoped,
        );
    }

    /// Asks about the next destination, or settles what is left. A single
    /// remaining card has nothing to decide: the destination it is asked
    /// about has only one answer, so it is simply placed.
    pub(super) fn queue_distributed_selection(
        &mut self,
        progress: DistributedSelectionProgress,
        destinations: &'static [SelectionDestinationDef],
        object: &StackObject,
        context: EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        let mut progress = progress;
        while let Some(destination) = destinations.get(progress.next_destination).copied() {
            if progress.remaining.is_empty() {
                return;
            }
            if progress.remaining.len() > 1 {
                let options =
                    self.card_decision_options(&progress.remaining, DecisionZone::Library);
                self.queue_decision(
                    progress.player,
                    Self::distributed_selection_prompt(destination),
                    DecisionVisibility::Private,
                    if destination.zone == ZoneKind::Hand {
                        DecisionPreference::HigherCardValue
                    } else {
                        DecisionPreference::LowerCardValue
                    },
                    1..=1,
                    false,
                    options,
                    DecisionContinuation::DistributedTopCardSelection {
                        progress,
                        destinations,
                        object: Box::new(object.clone()),
                        context,
                        effect: scoped,
                    },
                );
                return;
            }
            // The last card, and nobody chooses which: it goes where the
            // destination being asked about says.
            let card = progress.remaining.remove(0);
            self.place_distributed_card(progress.player, card, destination, object);
            progress.next_destination += 1;
        }
    }

    /// What the looker is asked for one destination. Shared with the
    /// checkpoint, which compares the prompt it rebuilds against the one the
    /// observation carries.
    pub(super) fn distributed_selection_prompt(
        destination: SelectionDestinationDef,
    ) -> &'static str {
        match (destination.zone, destination.placement) {
            (ZoneKind::Hand, _) => "Put a card into your hand",
            (ZoneKind::Library, ZonePlacement::Top) => "Put a card on top of your library",
            (ZoneKind::Library, ZonePlacement::Bottom) => "Put a card on the bottom of your library",
            (ZoneKind::Graveyard, _) => "Put a card into your graveyard",
            _ => "Exile a card",
        }
    }

    /// Places one card where its destination says, and grants what being
    /// there lets it do.
    pub(super) fn place_distributed_card(
        &mut self,
        player: PlayerId,
        card: CardInstance,
        destination: SelectionDestinationDef,
        object: &StackObject,
    ) {
        let moved = card.id;
        self.place_revealed_remainder(
            player,
            vec![card],
            destination.zone,
            destination.placement,
        );
        // "You may play the exiled card this turn." What landed in exile is
        // a new object, so the permission names the successor rather than the
        // card that was looked at.
        if destination.playable_this_turn {
            let exiled = self.successors.get(&moved).copied().unwrap_or(moved);
            self.permit_cast_this_turn(exiled, object.controller);
        }
    }
}
