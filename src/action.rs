use std::error::Error;
use std::fmt;

use crate::{CardInstanceId, PlayerId, StackObjectId};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Target {
    Player(PlayerId),
    Permanent(CardInstanceId),
    Spell(StackObjectId),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CombatDamageAssignment {
    pub recipient: Target,
    pub amount: u16,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Action {
    KeepHand,
    TakeMulligan,
    BottomCards {
        cards: Vec<CardInstanceId>,
    },
    DiscardCards {
        cards: Vec<CardInstanceId>,
    },
    ChooseTriggeredAbility {
        pay: bool,
        new_targets: Vec<Target>,
    },
    ChooseCopyTargets {
        targets: Vec<Target>,
    },
    ChooseUntap {
        permanents: Vec<CardInstanceId>,
    },
    PassPriority,
    PlayLand {
        card: CardInstanceId,
    },
    ActivateManaAbility {
        source: CardInstanceId,
    },
    CastSpell {
        card: CardInstanceId,
        targets: Vec<Target>,
        x: u16,
    },
    ActivateAbility {
        source: CardInstanceId,
        target: Option<Target>,
        sacrifice: Option<CardInstanceId>,
    },
    DeclareAttacker {
        attacker: CardInstanceId,
    },
    FinishDeclaringAttackers,
    DeclareBlocker {
        blocker: CardInstanceId,
        attacker: CardInstanceId,
    },
    FinishDeclaringBlockers,
    AssignCombatDamage {
        attacker: CardInstanceId,
        assignments: Vec<CombatDamageAssignment>,
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
