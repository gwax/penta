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
