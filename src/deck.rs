use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use crate::CardDefinitionId;
use crate::card::CardCatalog;
use crate::rules;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deck {
    pub main: Vec<CardDefinitionId>,
    pub sideboard: Vec<CardDefinitionId>,
}

impl Deck {
    /// Checks this deck against the fixed Eternal Central construction rules.
    ///
    /// # Errors
    ///
    /// Returns [`DeckError`] when the deck size, card identities, banned list,
    /// restricted list, or copy limits are invalid.
    pub fn validate(self, catalog: &CardCatalog) -> Result<ValidatedDeck, DeckError> {
        if self.main.len() < rules::MINIMUM_MAIN_DECK_SIZE {
            return Err(DeckError::MainDeckTooSmall {
                actual: self.main.len(),
            });
        }
        if self.sideboard.len() > rules::MAXIMUM_SIDEBOARD_SIZE {
            return Err(DeckError::SideboardTooLarge {
                actual: self.sideboard.len(),
            });
        }

        let mut counts = HashMap::<CardDefinitionId, usize>::new();
        for id in self.main.iter().chain(&self.sideboard) {
            let Some(card) = catalog.get(*id) else {
                return Err(DeckError::UnknownCard(*id));
            };
            if catalog.is_banned(*id) {
                return Err(DeckError::BannedCard(card.name.clone()));
            }
            *counts.entry(*id).or_default() += 1;
        }

        for (id, count) in counts {
            let Some(card) = catalog.get(id) else {
                return Err(DeckError::UnknownCard(id));
            };
            let limit = if card.is_basic_land {
                usize::MAX
            } else if catalog.is_restricted(id) {
                1
            } else {
                rules::MAXIMUM_COPIES
            };
            if count > limit {
                return Err(DeckError::TooManyCopies {
                    card: card.name.clone(),
                    count,
                    limit,
                });
            }
        }

        Ok(ValidatedDeck(self))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedDeck(Deck);

impl ValidatedDeck {
    pub(crate) fn into_main(self) -> Vec<CardDefinitionId> {
        self.0.main
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeckError {
    MainDeckTooSmall {
        actual: usize,
    },
    SideboardTooLarge {
        actual: usize,
    },
    UnknownCard(CardDefinitionId),
    BannedCard(String),
    TooManyCopies {
        card: String,
        count: usize,
        limit: usize,
    },
}

impl fmt::Display for DeckError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MainDeckTooSmall { actual } => write!(
                formatter,
                "main deck has {actual} cards; at least {} are required",
                rules::MINIMUM_MAIN_DECK_SIZE
            ),
            Self::SideboardTooLarge { actual } => write!(
                formatter,
                "sideboard has {actual} cards; at most {} are allowed",
                rules::MAXIMUM_SIDEBOARD_SIZE
            ),
            Self::UnknownCard(id) => write!(formatter, "unknown card definition ID {id:?}"),
            Self::BannedCard(card) => write!(formatter, "{card} is banned"),
            Self::TooManyCopies { card, count, limit } => {
                write!(
                    formatter,
                    "{card} appears {count} times; the limit is {limit}"
                )
            }
        }
    }
}

impl Error for DeckError {}
