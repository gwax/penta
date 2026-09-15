//! Pregame deck requirements use card characteristics, never live object predicates.
use super::CardType;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CompanionDef {
    pub requirement: DeckRequirementDef,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeckCards {
    All,
    Permanents,
    Nonlands,
    OfType(CardType),
}

impl DeckCards {
    #[must_use]
    pub const fn permanents() -> Self {
        Self::Permanents
    }
    #[must_use]
    pub const fn nonlands() -> Self {
        Self::Nonlands
    }
    #[must_use]
    pub const fn creatures() -> Self {
        Self::OfType(CardType::Creature)
    }
    #[must_use]
    pub const fn all(self, requirement: CardRequirement) -> DeckRequirementDef {
        DeckRequirementDef::Each {
            cards: self,
            requirement,
        }
    }
    #[must_use]
    pub const fn distinct_by(self, property: CardProperty) -> DeckRequirementDef {
        DeckRequirementDef::Distinct {
            cards: self,
            property,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CardRequirement {
    ManaValueAtMost(u16),
    ManaValueAtLeast(u16),
    HasActivatedAbility,
    HasAnySubtype(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CardProperty {
    Name,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeckRequirementDef {
    Each {
        cards: DeckCards,
        requirement: CardRequirement,
    },
    Distinct {
        cards: DeckCards,
        property: CardProperty,
    },
    /// The format minimum includes commanders in a Commander game.
    MinimumSizeAboveFormat(usize),
}
