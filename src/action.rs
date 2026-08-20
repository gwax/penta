use std::error::Error;
use std::fmt;

use crate::card::{BasicLandType, ManaSplit};
use crate::casting::{CastChoices, TargetSelection};
use crate::{
    AbilityId, CardDefinitionId, CardPartId, GameObjectId, GrantId, PlayOptionId, PlayerId,
};

pub use crate::card::ManaColor;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Target {
    Player(PlayerId),
    /// A card object in a non-battlefield, non-stack zone. The object ID is
    /// the current zone incarnation, so moving the card makes this target
    /// illegal without conflating it with the new object created there.
    Card(GameObjectId),
    Permanent(GameObjectId),
    Spell(GameObjectId),
}

/// The stable origin of an effective ability on a game object.
///
/// Printed IDs are local to one card part, so copied abilities freeze their
/// effective card definition as well as the part and clause ID. Intrinsic land
/// abilities are identified by the subtype that grants them. A granted origin
/// records the granting object, the effective card definition and part that
/// supplied its positional source clause, and the grant site inside that
/// clause; it is provenance, not an executable definition. Stack objects
/// separately freeze the effective text, target declarations, and resolver
/// they received at creation. Pair this with the affected object's
/// [`GameObjectId`] to identify one ability in a game.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AbilityOrigin {
    Printed {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
    },
    IntrinsicBasicLand(BasicLandType),
    Granted {
        source: GameObjectId,
        source_definition: CardDefinitionId,
        source_part: CardPartId,
        source_ability: AbilityId,
        grant: GrantId,
    },
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CombatDamageAssignment {
    pub recipient: Target,
    pub amount: u16,
}

/// The player or planeswalker a creature is attacking.
///
/// Declaring an attack does not target, so this is deliberately distinct from
/// [`Target`]. Keeping the defender on the attacker also prevents a
/// planeswalker that leaves combat from silently redirecting that attack to
/// its controller.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AttackDefender {
    Player(PlayerId),
    Planeswalker(GameObjectId),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Action {
    KeepHand,
    TakeMulligan,
    BottomCards {
        cards: Vec<GameObjectId>,
    },
    DiscardCards {
        cards: Vec<GameObjectId>,
    },
    ChooseDecision {
        decision: u32,
        options: Vec<u32>,
    },
    CancelDecision {
        decision: u32,
    },
    ChooseUntap {
        permanents: Vec<GameObjectId>,
    },
    PassPriority,
    PlayLand {
        card: GameObjectId,
        option: PlayOptionId,
    },
    ActivateManaAbility {
        source: GameObjectId,
        ability: AbilityOrigin,
        color: ManaColor,
        /// How many counters an open-ended removal cost takes, when the
        /// ability has one. Source, ability, and colour do not distinguish
        /// "remove one storage counter" from "remove three", so the size is
        /// part of the action rather than something chosen afterwards.
        /// `None` for every ability whose cost has only one size.
        counters_removed: Option<u16>,
        /// The permanent a "Sacrifice a <thing>" cost consumes. Source,
        /// ability, and colour name one Goblin-sacrificing ability once per
        /// Goblin, so which one is part of the action: a mana ability
        /// resolves without ever holding priority, so there is no window in
        /// which to ask afterwards. `None` for every ability that sacrifices
        /// nothing but itself.
        cost_object: Option<GameObjectId>,
        /// How the amount is divided, for an ability that adds mana "in any
        /// combination of" more than one type. Source, ability, and colour
        /// name one such ability once per division, so the division is part
        /// of the action: like the two choices above, a mana ability resolves
        /// without ever holding priority. `None` for every ability that
        /// produces one type at a time.
        combination: Option<ManaSplit>,
    },
    PayLifeForMana,
    CastSpell {
        card: GameObjectId,
        choices: CastChoices,
        sacrifices: Vec<GameObjectId>,
    },
    ActivateAbility {
        source: GameObjectId,
        ability: AbilityOrigin,
        targets: Vec<TargetSelection>,
        /// The objects chosen to pay a nonmana cost: the permanent a
        /// sacrifice cost takes, or the cards an exile cost lifts from a
        /// graveyard. Most costs name one or none; a cost that spends several
        /// names them all, because an activation has no window in which to
        /// ask afterwards. Empty when the cost spends nothing chosen.
        cost_objects: Vec<GameObjectId>,
        /// The value chosen for X in the activation cost, zero when the cost
        /// has no X.
        x: u16,
    },
    /// Turn a face-down permanent face up by paying its morph cost. A
    /// special action rather than an ability: it uses no stack, nothing can
    /// respond to it, and the permanent it names has no abilities to
    /// activate while it is face down (CR 702.37b).
    TurnFaceUp {
        permanent: GameObjectId,
    },
    DeclareAttacker {
        attacker: GameObjectId,
        defender: AttackDefender,
    },
    /// Puts two declared attackers, and everything already banded with
    /// either of them, into one attacking band. Bands are built a pair at a
    /// time rather than named all at once so that the legal ones can be
    /// enumerated the way every other declaration is.
    BandAttackers {
        first: GameObjectId,
        second: GameObjectId,
    },
    FinishDeclaringAttackers,
    DeclareBlocker {
        blocker: GameObjectId,
        attacker: GameObjectId,
    },
    FinishDeclaringBlockers,
    AssignCombatDamage {
        attacker: GameObjectId,
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
