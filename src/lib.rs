//! Deterministic engine primitives for Eternal Central Old School 93/94.

pub mod action;
pub mod card;
pub mod deck;
pub mod game;
pub mod ids;
pub mod poc;
mod rng;
pub mod rules;

pub use action::{Action, ActionError, CombatDamageAssignment, Target};
pub use card::{
    CardBehavior, CardCatalog, CardDefinition, CardKind, CardSet, CatalogError, CreatureStats,
    ManaCost,
};
pub use deck::{Deck, DeckError, ValidatedDeck};
pub use game::{
    Game, GameError, GameEvent, GameResult, ManaPool, PlayerObservation, StackObjectKind, Step,
    WinReason,
};
pub use ids::{CardDefinitionId, CardInstanceId, PlayerId, StackObjectId};
