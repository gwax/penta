//! Companion eligibility is checked before opening hands. A single revealed
//! designation grants a once-per-game special action (CR 103.2b, 702.139).

use crate::card::ManaCost;
use crate::ids::GameObjectId;

use super::{
    Action, DecisionContinuation, DecisionPreference, DecisionVisibility, DecisionZone, Game,
    GameEvent, ManaPaymentPurpose, PlayerId, Pregame,
};

/// Public designation survives using the special action. The object is the
/// original outside-game object, never a later copy or another sideboard card.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionState {
    #[serde(rename = "objectId", with = "wire_object")]
    pub card: GameObjectId,
    pub definition: crate::CardDefinitionId,
    pub used: bool,
}

/// What taking a companion costs. The same {3} for every card that prints
/// the keyword, which is why no card writes it down.
const COMPANION_COST: ManaCost = crate::mana_cost!("{3}");

impl Game {
    /// "As a sorcery": the ordinary main-phase window, which is what the
    /// reminder text names rather than what the card's own type would allow.
    pub(super) fn add_companion_actions(&self, player: PlayerId, actions: &mut Vec<Action>) {
        let state = &self.players[player.index()];
        if state.companion.is_none_or(|companion| companion.used)
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
                .filter(|card| {
                    state
                        .companion
                        .is_some_and(|companion| companion.card == card.id)
                })
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
            .companion
            .is_some_and(|chosen| !chosen.used && chosen.card == card)
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
        self.players[player.index()]
            .companion
            .as_mut()
            .expect("chosen companion")
            .used = true;
    }
}

impl Game {
    pub(super) fn eligible_companions(&self, player: PlayerId) -> Vec<super::CardInstance> {
        let state = &self.players[player.index()];
        state
            .outside_game
            .iter()
            .filter(|card| {
                crate::deck::validate_companion_requirement(
                    card.definition,
                    &self.catalog,
                    state
                        .library
                        .iter()
                        .chain(&state.command)
                        .map(|card| card.definition),
                    self.format,
                )
                .is_ok()
            })
            .cloned()
            .collect()
    }

    pub(super) fn begin_companion_selection(&mut self, player: PlayerId) {
        let candidates = self.eligible_companions(player);
        if candidates.is_empty() {
            self.finish_companion_selection(player, None);
            return;
        }
        self.pregame = Some(Pregame::Companion(player));
        self.priority = player;
        self.queue_decision(
            player,
            "Reveal a companion, or continue without one",
            DecisionVisibility::Private,
            DecisionPreference::Neutral,
            0..=1,
            false,
            self.card_decision_options(&candidates, DecisionZone::OutsideGame),
            DecisionContinuation::ChooseCompanion { player },
        );
    }

    pub(super) fn finish_companion_selection(
        &mut self,
        player: PlayerId,
        card: Option<GameObjectId>,
    ) {
        if let Some(id) = card {
            let card = self.players[player.index()]
                .outside_game
                .iter()
                .find(|card| card.id == id)
                .expect("the choice names an eligible outside-game card");
            self.players[player.index()].companion = Some(CompanionState {
                card: id,
                definition: card.definition,
                used: false,
            });
            self.events.push(GameEvent::CardRevealed {
                player,
                card: id,
                definition: self.players[player.index()].companion.unwrap().definition,
            });
        }
        if player == self.starting_player {
            self.begin_companion_selection(player.opponent());
        } else {
            for seat in [PlayerId::One, PlayerId::Two] {
                let count = self
                    .format
                    .rules()
                    .opening_hand_size
                    .min(self.players[seat.index()].library.len());
                let cards = super::lifecycle::draw_opening_hand(
                    &mut self.players[seat.index()].library,
                    count,
                )
                .expect("bounded opening hand");
                for card in cards {
                    let (card, _) = self.zone_change_card(card);
                    self.players[seat.index()].hand.push(card);
                }
            }
            self.pregame = Some(Pregame::Mulligan(self.starting_player));
            self.priority = self.starting_player;
        }
    }
}

mod wire_object {
    use super::GameObjectId;
    use serde::{Deserialize, Deserializer, Serializer};
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub(super) fn serialize<S: Serializer>(
        id: &GameObjectId,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(id.0)
    }
    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<GameObjectId, D::Error> {
        u32::deserialize(deserializer).map(GameObjectId)
    }
}

impl Game {
    pub(super) fn validate_companion_checkpoint(&self) -> Result<(), String> {
        if let Some(Pregame::Companion(player)) = self.pregame {
            if self.players.iter().any(|state| !state.hand.is_empty())
                || self.players[player.index()].companion.is_some()
            {
                return Err(
                    "companion selection must precede opening hands and designation".into(),
                );
            }
            let pending = self
                .pending_decisions
                .first()
                .ok_or("missing companion selection decision")?;
            let expected = self.card_decision_options(
                &self.eligible_companions(player),
                DecisionZone::OutsideGame,
            );
            if !matches!(pending.continuation, DecisionContinuation::ChooseCompanion { player: chooser } if chooser == player)
                || pending.observation.options != expected
                || pending.observation.minimum != 0
                || pending.observation.maximum != 1
                || expected.is_empty()
            {
                return Err("companion selection disagrees with starting-deck eligibility".into());
            }
        }
        Ok(())
    }
}
