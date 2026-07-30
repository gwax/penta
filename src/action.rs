use std::error::Error;
use std::fmt;

use crate::{CardInstanceId, PlayerId};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Target {
    Player(PlayerId),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Action {
    PassPriority,
    PlayLand {
        card: CardInstanceId,
    },
    ActivateManaAbility {
        source: CardInstanceId,
    },
    CastSpell {
        card: CardInstanceId,
        target: Target,
    },
    Concede,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionError {
    GameAlreadyFinished,
    NotLegal { player: PlayerId, action: Action },
}

impl fmt::Display for ActionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GameAlreadyFinished => formatter.write_str("the game is already finished"),
            Self::NotLegal { player, action } => {
                write!(formatter, "{action:?} is not legal for {player}")
            }
        }
    }
}

impl Error for ActionError {}
