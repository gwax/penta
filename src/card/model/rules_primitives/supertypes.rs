// Supertypes: the printed words that sit ahead of a card's types.
//
// Split out of `rules_primitives.rs` for the source-size budget; the file
// next door is about counters, which is a different vocabulary entirely.
// Included textually, so the imports here are that module's.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CardSupertype {
    Basic,
    Legendary,
    Snow,
    World,
}

/// A const-friendly set of supertypes for layer-4 characteristic changes.
///
/// Printed card rules keep their compact flag array, while continuous effects
/// need an ordinary value so add, remove, and set operations compose in the
/// same form as card types and colors.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct CardSupertypeSet(u8);

impl CardSupertypeSet {
    pub const EMPTY: Self = Self(0);

    #[must_use]
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    #[must_use]
    pub const fn single(supertype: CardSupertype) -> Self {
        Self(1 << supertype.index())
    }

    #[must_use]
    pub const fn with(mut self, supertype: CardSupertype) -> Self {
        self.0 |= 1 << supertype.index();
        self
    }

    #[must_use]
    pub const fn contains(self, supertype: CardSupertype) -> bool {
        self.0 & (1 << supertype.index()) != 0
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl CardSupertype {
    pub const COUNT: usize = 4;

    pub const ALL: [Self; Self::COUNT] = [Self::Basic, Self::Legendary, Self::Snow, Self::World];

    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Basic => 0,
            Self::Legendary => 1,
            Self::Snow => 2,
            Self::World => 3,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Basic => "Basic",
            Self::Legendary => "Legendary",
            Self::Snow => "Snow",
            Self::World => "World",
        }
    }
}
