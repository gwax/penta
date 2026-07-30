//! Deterministic engine primitives for Eternal Central Old School 93/94.

pub mod action;
pub mod card;
pub mod deck;
pub mod game;
pub mod ids;
mod rng;
pub mod rules;

pub use action::{Action, ActionError};
pub use card::{CardCatalog, CardDefinition, CardSet, CatalogError};
pub use deck::{Deck, DeckError, ValidatedDeck};
pub use game::{Game, GameError, GameResult, PlayerObservation};
pub use ids::{CardDefinitionId, CardInstanceId, PlayerId};
