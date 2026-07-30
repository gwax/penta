use std::error::Error;
use std::fmt;

use crate::PlayerId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
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
