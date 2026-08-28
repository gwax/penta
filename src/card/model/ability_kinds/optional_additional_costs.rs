//! Optional additional costs selected independently of how a spell is cast.
//!
//! Unlike an alternative cost, one of these adds to the spell's already
//! calculated cost and can therefore be combined with flashback, overload, or
//! any other legal way of casting it.

use crate::ids::{AbilityId, AdditionalCostId};

use super::super::{AdditionalCostDef, ManaCost};
use super::{SpellAdditionalCostDef, SpellResolutionDestinationDef};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OptionalAdditionalCostKindDef {
    /// Buyback (CR 702.27): if this cost was paid, a resolving spell card goes
    /// to its owner's hand instead of its graveyard.
    Buyback,
    /// Replicate (CR 702.55): the cost may be paid any number of times, and
    /// what it buys is a copy of the spell for each payment. The copies come
    /// from a cast trigger the card prints beside this, which reads how many
    /// times the cost was paid.
    Replicate,
    /// Multikicker (CR 702.33b): a kicker cost that may be paid any number
    /// of times as the spell is cast. Like replicate it is repeatable and
    /// changes nothing by itself; what it buys is read back off the count by
    /// whatever the card prints about having been kicked.
    Multikicker,
    /// Squad (CR 702.152): a cost that may be paid any number of times as
    /// the creature spell is cast, and an enters trigger that makes that
    /// many token copies of the creature. Repeatable like the two above;
    /// what it buys is printed beside it and reads the count back.
    Squad,
}

impl OptionalAdditionalCostKindDef {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Buyback => "Buyback",
            Self::Replicate => "Replicate",
            Self::Multikicker => "Multikicker",
            Self::Squad => "Squad",
        }
    }

    /// Whether one cast may pay this cost more than once. Buyback is the
    /// one that cannot: every repeatable cost buys a number rather than a
    /// yes, and something on the card reads that number back.
    #[must_use]
    pub const fn repeatable(self) -> bool {
        matches!(self, Self::Replicate | Self::Multikicker | Self::Squad)
    }
}

/// One named optional additional cost and the stack outcome it locks in.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OptionalAdditionalCostAbilityDef {
    pub kind: OptionalAdditionalCostKindDef,
    pub mana_cost: Option<ManaCost>,
    pub additional_cost: Option<SpellAdditionalCostDef>,
    pub resolution_destination: SpellResolutionDestinationDef,
}

impl OptionalAdditionalCostAbilityDef {
    #[must_use]
    pub fn rules_text(self) -> String {
        match (self.kind, self.mana_cost) {
            (OptionalAdditionalCostKindDef::Buyback, Some(cost)) => format!(
                "Buyback {cost} (You may pay an additional {cost} as you cast this spell. If you \
                 do, put this card into your hand as it resolves.)"
            ),
            (OptionalAdditionalCostKindDef::Buyback, None) => "Buyback".into(),
            (OptionalAdditionalCostKindDef::Replicate, Some(cost)) => format!(
                "Replicate {cost} (When you cast this spell, copy it for each time you paid its \
                 replicate cost. You may choose new targets for the copies.)"
            ),
            (OptionalAdditionalCostKindDef::Replicate, None) => "Replicate".into(),
            (OptionalAdditionalCostKindDef::Multikicker, Some(cost)) => format!(
                "Multikicker {cost} (You may pay an additional {cost} any number of times as you \
                 cast this spell.)"
            ),
            (OptionalAdditionalCostKindDef::Multikicker, None) => "Multikicker".into(),
            (OptionalAdditionalCostKindDef::Squad, Some(cost)) => format!(
                "Squad {cost} (As an additional cost to cast this spell, you may pay {cost} any \
                 number of times. When this creature enters, create that many tokens that are \
                 copies of it.)"
            ),
            (OptionalAdditionalCostKindDef::Squad, None) => "Squad".into(),
        }
    }

    #[must_use]
    pub fn additional_cost(self, ability: AbilityId) -> AdditionalCostDef {
        AdditionalCostDef {
            id: AdditionalCostId(ability.0),
            label: self.kind.label().into(),
            mana_cost: self.mana_cost,
            repeatable: self.kind.repeatable(),
        }
    }
}
