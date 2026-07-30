use std::error::Error;
use std::fmt;

use crate::action::{Action, ActionError};
use crate::deck::ValidatedDeck;
use crate::ids::{CardDefinitionId, CardInstanceId, PlayerId};
use crate::rng::ReplayRng;
use crate::rules;

#[derive(Clone, Debug, Eq, PartialEq)]
struct CardInstance {
    id: CardInstanceId,
    definition: CardDefinitionId,
    owner: PlayerId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlayerState {
    life: u8,
    library: Vec<CardInstance>,
    hand: Vec<CardInstance>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameResult {
    Winner { winner: PlayerId, reason: WinReason },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WinReason {
    OpponentConceded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerObservation {
    pub viewer: PlayerId,
    pub life_totals: [u8; 2],
    pub hand: Vec<(CardInstanceId, CardDefinitionId)>,
    pub opponent_hand_size: usize,
    pub library_sizes: [usize; 2],
    pub result: Option<GameResult>,
    pub legal_actions: Vec<Action>,
}

#[derive(Clone, Debug)]
pub struct Game {
    seed: u64,
    players: [PlayerState; 2],
    result: Option<GameResult>,
}

impl Game {
    /// Creates a game, shuffles both decks, and draws opening hands.
    ///
    /// # Errors
    ///
    /// Returns [`GameError`] if card instance IDs are exhausted or a deck
    /// cannot supply an opening hand.
    pub fn new(decks: [ValidatedDeck; 2], seed: u64) -> Result<Self, GameError> {
        let mut rng = ReplayRng::new(seed);
        let mut next_instance_id = 0_u32;

        let mut build_player =
            |player: PlayerId, deck: ValidatedDeck| -> Result<PlayerState, GameError> {
                let definitions = deck.into_main();
                let mut library = Vec::with_capacity(definitions.len());
                for definition in definitions {
                    let id = CardInstanceId(next_instance_id);
                    next_instance_id = next_instance_id
                        .checked_add(1)
                        .ok_or(GameError::TooManyCards)?;
                    library.push(CardInstance {
                        id,
                        definition,
                        owner: player,
                    });
                }
                rng.shuffle(&mut library);
                let hand = draw_opening_hand(&mut library)?;
                Ok(PlayerState {
                    life: rules::STARTING_LIFE,
                    library,
                    hand,
                })
            };

        let [deck_one, deck_two] = decks;
        let players = [
            build_player(PlayerId::One, deck_one)?,
            build_player(PlayerId::Two, deck_two)?,
        ];

        Ok(Self {
            seed,
            players,
            result: None,
        })
    }

    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    #[must_use]
    pub fn legal_actions(&self, _player: PlayerId) -> Vec<Action> {
        if self.result.is_none() {
            vec![Action::Concede]
        } else {
            Vec::new()
        }
    }

    /// Applies one engine-enumerated action for a player.
    ///
    /// # Errors
    ///
    /// Returns [`ActionError`] when the game is over or the action is not
    /// currently legal for that player.
    pub fn apply(&mut self, player: PlayerId, action: Action) -> Result<(), ActionError> {
        if self.result.is_some() {
            return Err(ActionError::GameAlreadyFinished);
        }
        if !self.legal_actions(player).contains(&action) {
            return Err(ActionError::NotLegal { player, action });
        }
        match action {
            Action::Concede => {
                self.result = Some(GameResult::Winner {
                    winner: player.opponent(),
                    reason: WinReason::OpponentConceded,
                });
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn observe(&self, viewer: PlayerId) -> PlayerObservation {
        let player = &self.players[viewer.index()];
        let opponent = &self.players[viewer.opponent().index()];
        PlayerObservation {
            viewer,
            life_totals: [self.players[0].life, self.players[1].life],
            hand: player
                .hand
                .iter()
                .map(|card| (card.id, card.definition))
                .collect(),
            opponent_hand_size: opponent.hand.len(),
            library_sizes: [self.players[0].library.len(), self.players[1].library.len()],
            result: self.result,
            legal_actions: self.legal_actions(viewer),
        }
    }
}

fn draw_opening_hand(library: &mut Vec<CardInstance>) -> Result<Vec<CardInstance>, GameError> {
    if library.len() < rules::OPENING_HAND_SIZE {
        return Err(GameError::NotEnoughCardsForOpeningHand);
    }
    let split_at = library.len() - rules::OPENING_HAND_SIZE;
    Ok(library.split_off(split_at))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameError {
    TooManyCards,
    NotEnoughCardsForOpeningHand,
}

impl fmt::Display for GameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyCards => formatter.write_str("game contains too many card instances"),
            Self::NotEnoughCardsForOpeningHand => {
                formatter.write_str("deck cannot provide a seven-card opening hand")
            }
        }
    }
}

impl Error for GameError {}
