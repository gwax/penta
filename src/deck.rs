use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use crate::CardDefinitionId;
use crate::Format;
use crate::card::CardCatalog;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deck {
    pub main: Vec<CardDefinitionId>,
    pub sideboard: Vec<CardDefinitionId>,
}

impl Deck {
    /// Checks this deck against the default Eternal Central construction rules.
    ///
    /// # Errors
    ///
    /// Returns [`DeckError`] when the deck size, card identities, banned list,
    /// restricted list, or copy limits are invalid.
    pub fn validate(self, catalog: &CardCatalog) -> Result<ValidatedDeck, DeckError> {
        self.validate_for_format(catalog, Format::OldSchool9394)
    }

    /// Checks this deck against the construction rules and card legality of
    /// `format`.
    ///
    /// # Errors
    ///
    /// Returns [`DeckError`] when the deck size, card identities, format
    /// legality, banned list, restricted list, or copy limits are invalid.
    pub fn validate_for_format(
        self,
        catalog: &CardCatalog,
        format: Format,
    ) -> Result<ValidatedDeck, DeckError> {
        let format_rules = format.rules();
        if self.main.len() < format_rules.minimum_main_deck_size {
            return Err(DeckError::MainDeckTooSmall {
                actual: self.main.len(),
                minimum: format_rules.minimum_main_deck_size,
            });
        }
        if self.sideboard.len() > format_rules.maximum_sideboard_size {
            return Err(DeckError::SideboardTooLarge {
                actual: self.sideboard.len(),
                maximum: format_rules.maximum_sideboard_size,
            });
        }

        let mut counts = HashMap::<CardDefinitionId, usize>::new();
        for id in self.main.iter().chain(&self.sideboard) {
            let Some(card) = catalog.get(*id) else {
                return Err(DeckError::UnknownCard(*id));
            };
            if !catalog.is_allowed_in(*id, format) {
                return Err(DeckError::CardNotAllowed {
                    card: card.name.clone(),
                    format,
                });
            }
            if catalog.is_banned_in(*id, format) {
                return Err(DeckError::BannedCard(card.name.clone()));
            }
            *counts.entry(*id).or_default() += 1;
        }

        for (id, count) in counts {
            let Some(card) = catalog.get(id) else {
                return Err(DeckError::UnknownCard(id));
            };
            let limit = if card.is_basic_land() {
                usize::MAX
            } else if catalog.is_restricted_in(id, format) {
                1
            } else {
                format_rules.maximum_copies
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

    /// Checks this deck as a Commander deck led by `commander`.
    ///
    /// This is deck construction only: the engine plays no format with a
    /// command zone, so a validated commander deck is a legal list rather
    /// than a game that can be started from it. What is checked is what the
    /// singleton rules say -- who may lead (CR 903.3), that the leader is
    /// not also in the deck, and that nothing else is duplicated (CR 903.5b)
    /// -- and deliberately not colour identity, which no card in this
    /// catalog prints and nothing here can yet compute.
    ///
    /// # Errors
    ///
    /// Returns [`DeckError`] when the commander is unknown or may not lead,
    /// when it also appears in the deck, or when the list is the wrong size
    /// or not singleton.
    pub fn validate_as_commander_deck(
        self,
        catalog: &CardCatalog,
        commander: CardDefinitionId,
    ) -> Result<ValidatedDeck, DeckError> {
        let Some(leader) = catalog.get(commander) else {
            return Err(DeckError::UnknownCard(commander));
        };
        if !leader.may_be_commander() {
            return Err(DeckError::NotALegalCommander(leader.name.clone()));
        }
        if self.main.contains(&commander) {
            return Err(DeckError::CommanderInDeck(leader.name.clone()));
        }
        // The commander is one of the hundred; the list beside it is the
        // other ninety-nine.
        if self.main.len() != COMMANDER_DECK_SIZE - 1 {
            return Err(DeckError::MainDeckTooSmall {
                actual: self.main.len(),
                minimum: COMMANDER_DECK_SIZE - 1,
            });
        }

        let mut counts = HashMap::<CardDefinitionId, usize>::new();
        for id in &self.main {
            if catalog.get(*id).is_none() {
                return Err(DeckError::UnknownCard(*id));
            }
            *counts.entry(*id).or_default() += 1;
        }
        for (id, count) in counts {
            let Some(card) = catalog.get(id) else {
                return Err(DeckError::UnknownCard(id));
            };
            if count > 1 && !card.is_basic_land() {
                return Err(DeckError::TooManyCopies {
                    card: card.name.clone(),
                    count,
                    limit: 1,
                });
            }
        }

        Ok(ValidatedDeck(self))
    }
}

/// A Commander deck is a hundred cards counting the commander (CR 903.5a).
const COMMANDER_DECK_SIZE: usize = 100;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedDeck(Deck);

impl ValidatedDeck {
    pub(crate) fn into_parts(self) -> (Vec<CardDefinitionId>, Vec<CardDefinitionId>) {
        (self.0.main, self.0.sideboard)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeckError {
    MainDeckTooSmall {
        actual: usize,
        minimum: usize,
    },
    SideboardTooLarge {
        actual: usize,
        maximum: usize,
    },
    UnknownCard(CardDefinitionId),
    CardNotAllowed {
        card: String,
        format: Format,
    },
    BannedCard(String),
    /// The named card is neither a legendary creature nor a card that prints
    /// permission to lead a deck.
    NotALegalCommander(String),
    /// The commander was also listed among the ninety-nine.
    CommanderInDeck(String),
    TooManyCopies {
        card: String,
        count: usize,
        limit: usize,
    },
}

impl fmt::Display for DeckError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MainDeckTooSmall { actual, minimum } => write!(
                formatter,
                "main deck has {actual} cards; at least {minimum} are required"
            ),
            Self::SideboardTooLarge { actual, maximum } => write!(
                formatter,
                "sideboard has {actual} cards; at most {maximum} are allowed"
            ),
            Self::UnknownCard(id) => write!(formatter, "unknown card definition ID {id:?}"),
            Self::CardNotAllowed { card, format } => {
                write!(formatter, "{card} is not legal in {format}")
            }
            Self::BannedCard(card) => write!(formatter, "{card} is banned"),
            Self::NotALegalCommander(card) => {
                write!(formatter, "{card} cannot be your commander")
            }
            Self::CommanderInDeck(card) => write!(
                formatter,
                "{card} is the commander and cannot also be in the deck"
            ),
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

#[cfg(test)]
mod tests {
    use super::{Deck, DeckError};
    use crate::CardDefinitionId;
    use crate::card::{CardCatalog, cards};

    fn catalog() -> CardCatalog {
        crate::card::catalog().expect("catalog builds")
    }

    /// Ninety-nine distinct cards, which is a legal Commander list's size
    /// whatever those cards happen to be. Anything named is skipped so the
    /// caller can lead with it.
    fn ninety_nine(catalog: &CardCatalog, skip: &[CardDefinitionId]) -> Vec<CardDefinitionId> {
        let main: Vec<CardDefinitionId> = catalog
            .definitions()
            .iter()
            .map(|definition| definition.id)
            .filter(|id| !skip.contains(id))
            .take(99)
            .collect();
        assert_eq!(main.len(), 99, "the catalog is large enough to fill a list");
        main
    }

    /// Minsc & Boo is a planeswalker, so nothing but the printed sentence
    /// makes them a legal commander.
    #[test]
    fn a_card_that_says_so_can_be_your_commander() {
        let catalog = catalog();
        let minsc = catalog
            .get(cards::MINSC_BOO_TIMELESS_HEROES)
            .expect("cataloged");

        assert!(minsc.may_be_commander(), "the card says it can");
    }

    /// The ordinary permission is the type line, and a creature without the
    /// supertype has neither.
    #[test]
    fn a_legendary_creature_can_and_an_ordinary_one_cannot() {
        let catalog = catalog();

        assert!(
            catalog
                .get(cards::EMRY_LURKER_OF_THE_LOCH)
                .expect("cataloged")
                .may_be_commander(),
        );
        assert!(
            !catalog
                .get(cards::GRIZZLY_BEARS)
                .expect("cataloged")
                .may_be_commander(),
        );
    }

    #[test]
    fn a_hundred_singleton_cards_led_by_minsc_are_legal() {
        let catalog = catalog();
        let deck = Deck {
            main: ninety_nine(&catalog, &[cards::MINSC_BOO_TIMELESS_HEROES]),
            sideboard: Vec::new(),
        };

        deck.validate_as_commander_deck(&catalog, cards::MINSC_BOO_TIMELESS_HEROES)
            .expect("ninety-nine distinct cards and a legal leader");
    }

    #[test]
    fn a_card_without_the_permission_cannot_lead() {
        let catalog = catalog();
        let deck = Deck {
            main: ninety_nine(&catalog, &[cards::GRIZZLY_BEARS]),
            sideboard: Vec::new(),
        };

        let error = deck
            .validate_as_commander_deck(&catalog, cards::GRIZZLY_BEARS)
            .expect_err("a Grizzly Bears leads nothing");

        assert!(matches!(error, DeckError::NotALegalCommander(_)));
    }

    #[test]
    fn the_commander_may_not_also_be_one_of_the_ninety_nine() {
        let catalog = catalog();
        let mut main = ninety_nine(&catalog, &[cards::MINSC_BOO_TIMELESS_HEROES]);
        main[0] = cards::MINSC_BOO_TIMELESS_HEROES;
        let deck = Deck {
            main,
            sideboard: Vec::new(),
        };

        let error = deck
            .validate_as_commander_deck(&catalog, cards::MINSC_BOO_TIMELESS_HEROES)
            .expect_err("the leader is not also in the deck");

        assert!(matches!(error, DeckError::CommanderInDeck(_)));
    }

    /// Singleton is the point of the format: a second copy of anything but a
    /// basic land is illegal, and the size check must not mask it.
    #[test]
    fn a_second_copy_of_a_nonbasic_is_illegal() {
        let catalog = catalog();
        let mut main = ninety_nine(
            &catalog,
            &[cards::MINSC_BOO_TIMELESS_HEROES, cards::GRIZZLY_BEARS],
        );
        main[0] = cards::GRIZZLY_BEARS;
        main[1] = cards::GRIZZLY_BEARS;
        let deck = Deck {
            main,
            sideboard: Vec::new(),
        };

        let error = deck
            .validate_as_commander_deck(&catalog, cards::MINSC_BOO_TIMELESS_HEROES)
            .expect_err("two of something is not singleton");

        assert!(matches!(error, DeckError::TooManyCopies { count: 2, .. }));
    }

    #[test]
    fn basic_lands_are_exempt_from_singleton() {
        let catalog = catalog();
        let mut main = ninety_nine(&catalog, &[cards::MINSC_BOO_TIMELESS_HEROES, cards::FOREST]);
        main[0] = cards::FOREST;
        main[1] = cards::FOREST;
        let deck = Deck {
            main,
            sideboard: Vec::new(),
        };

        deck.validate_as_commander_deck(&catalog, cards::MINSC_BOO_TIMELESS_HEROES)
            .expect("any number of basics is legal");
    }

    #[test]
    fn ninety_eight_cards_are_not_a_deck() {
        let catalog = catalog();
        let mut main = ninety_nine(&catalog, &[cards::MINSC_BOO_TIMELESS_HEROES]);
        main.pop();
        let deck = Deck {
            main,
            sideboard: Vec::new(),
        };

        let error = deck
            .validate_as_commander_deck(&catalog, cards::MINSC_BOO_TIMELESS_HEROES)
            .expect_err("a hundred counts the commander");

        assert!(matches!(
            error,
            DeckError::MainDeckTooSmall {
                actual: 98,
                minimum: 99
            }
        ));
    }
}
