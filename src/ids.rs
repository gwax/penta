use std::fmt;

/// Stable identity of a card in the card catalog.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CardDefinitionId(pub u16);

/// Identity of one physical card for the duration of a game.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CardInstanceId(pub u32);

/// One of the two players in a game.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PlayerId {
    One,
    Two,
}

impl PlayerId {
    #[must_use]
    pub const fn opponent(self) -> Self {
        match self {
            Self::One => Self::Two,
            Self::Two => Self::One,
        }
    }

    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Two => 1,
        }
    }
}

impl fmt::Display for PlayerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::One => formatter.write_str("player one"),
            Self::Two => formatter.write_str("player two"),
        }
    }
}
