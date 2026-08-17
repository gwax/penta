use crate::action::{AbilityOrigin, ManaColor};
use crate::card::{AbilityCostList, AddManaEffectDef, AppliedEffectDef, SpellForm};
use crate::ids::{CardDefinitionId, GameObjectId, PlayerId};

use super::{ManaPool, ManaSource};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AppliedStackEffect {
    pub(super) source: Option<ManaSource>,
    pub(super) effect: AppliedEffectDef,
}

/// The object or procedure a mana payment is paying for. Restrictions are
/// evaluated against this frozen purpose both while planning mana abilities
/// and when selecting the exact mana units to spend.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ManaPaymentPurpose {
    Spell {
        object: GameObjectId,
        definition: CardDefinitionId,
        controller: PlayerId,
        form: SpellForm,
    },
    Ability {
        source: GameObjectId,
        /// Whether the ability taps its source to pay for itself. When it
        /// does, that source cannot also be tapped for mana, so it is barred
        /// from the payment rather than merely deprioritised.
        taps_source: bool,
        /// Whether the source must still be on the battlefield after mana is
        /// raised so it can be sacrificed or exiled for the main ability.
        /// Mana abilities of that same source which leave the battlefield are
        /// not legal ways to pay this cost.
        leaves_source: bool,
    },
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ManaAbilityActivation {
    pub(super) source: GameObjectId,
    pub(super) ability: AbilityOrigin,
    pub(super) color: ManaColor,
    pub(super) costs: AbilityCostList,
    pub(super) effect: AddManaEffectDef,
    /// How many counters this activation takes, for the abilities whose
    /// removal cost is open-ended and therefore offered once per size.
    /// `None` whenever the cost has only one size, which is every other
    /// mana ability.
    pub(super) counters_removed: Option<u16>,
    /// The permanent a "Sacrifice a <thing>" cost consumes. Like the counter
    /// size above, source and colour do not distinguish one Goblin from
    /// another, so the choice is enumerated into the activation rather than
    /// asked afterwards -- a mana ability has no window in which to ask.
    /// `None` for every ability that sacrifices nothing but itself.
    pub(super) cost_object: Option<GameObjectId>,
}

/// One enumerated output of a mana source: the ability, the colour, what it
/// produces, whether that mana helps the payment at hand, and the two choices
/// that distinguish otherwise identical activations -- the counter size and
/// the sacrificed permanent.
pub(super) type ManaSourceOutputs = Vec<(
    AbilityOrigin,
    ManaColor,
    ManaPool,
    bool,
    Option<u16>,
    Option<GameObjectId>,
)>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PlannedManaActivation {
    pub(super) source: GameObjectId,
    pub(super) ability: AbilityOrigin,
    pub(super) color: ManaColor,
    pub(super) production: ManaPool,
    pub(super) benefits_payment: bool,
    pub(super) flexibility: usize,
    pub(super) order: usize,
    /// Which sized activation this plan means, for a source that offers
    /// several. `None` for every ability with only one size.
    pub(super) counters_removed: Option<u16>,
    /// Which permanent this plan sacrifices, for a source that offers
    /// several. `None` for every ability that sacrifices nothing but itself.
    pub(super) cost_object: Option<GameObjectId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FlexibleManaSource {
    pub(super) source: GameObjectId,
    /// Ability, colour, what it makes, whether it benefits the payment,
    /// and how many counters its sized removal takes when the ability
    /// offers more than one size.
    pub(super) outputs: ManaSourceOutputs,
    pub(super) order: usize,
}
