//! Companion (CR 702.139).
//!
//! Two halves, and only the second is here. The first is a rule of deck
//! construction -- the condition the card prints, asked of the deck it sits
//! beside -- and is answered in the deck layer before a game exists; what a
//! game keeps of it is the list of companions each player may still take.
//!
//! The second is this: a special action, at sorcery speed, that pays {3} and
//! moves the companion from outside the game into hand. No stack, nothing to
//! respond to. Taking one empties the list, because however many a sideboard
//! made legal, a game has one companion.
//!
//! Which of several legal companions is "your chosen companion" is settled by
//! taking it rather than before the first turn. The engine has no pre-game
//! reveal for a designation to happen in, and the choice a player would make
//! there is the same one they make here.

use crate::card::ManaCost;
use crate::ids::GameObjectId;

use super::{Action, Game, ManaPaymentPurpose, PlayerId};

/// What taking a companion costs. The same {3} for every card that prints
/// the keyword, which is why no card writes it down.
const COMPANION_COST: ManaCost = crate::mana_cost!("{3}");

impl Game {
    /// "As a sorcery": the ordinary main-phase window, which is what the
    /// reminder text names rather than what the card's own type would allow.
    pub(super) fn add_companion_actions(&self, player: PlayerId, actions: &mut Vec<Action>) {
        let state = &self.players[player.index()];
        if state.companions.is_empty()
            || player != self.active_player
            || !self.step.is_main()
            || !self.stack.is_empty()
        {
            return;
        }
        if !self.can_pay_cost_for(player, COMPANION_COST, 0, &ManaPaymentPurpose::Other) {
            return;
        }
        actions.extend(
            state
                .outside_game
                .iter()
                .filter(|card| state.companions.contains(&card.definition))
                .map(|card| Action::TakeCompanion { card: card.id }),
        );
    }

    pub(super) fn take_companion(&mut self, player: PlayerId, card: GameObjectId) {
        let state = &self.players[player.index()];
        let Some(index) = state
            .outside_game
            .iter()
            .position(|candidate| candidate.id == card)
        else {
            return;
        };
        if !state
            .companions
            .contains(&state.outside_game[index].definition)
        {
            return;
        }
        self.activate_mana_for_cost(player, COMPANION_COST, 0);
        let _spent = self.pay_player_cost(player, COMPANION_COST, 0);
        let moved = self.players[player.index()].outside_game.remove(index);
        // Outside the game is not a zone, but arriving in a hand is still a
        // new object: nothing that watched the card out there may follow it
        // in.
        let (moved, _zone_change) = self.zone_change_card(moved);
        let owner = moved.owner;
        self.players[owner.index()].hand.push(moved);
        self.players[player.index()].companions.clear();
    }
}
