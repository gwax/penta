//! Semantic payment steps, including named-action completion boundaries.

use crate::card::CostDef;
use crate::ids::{GameObjectId, MechanicId};

use super::{EffectResolutionContext, PlayerId, ScopedEffect, StackObject};

mod activation;
mod selection;
mod window;

/// A choice path through the authored cost, not an executable serialized
/// callback. Choices and object selections do not mutate the game.
#[derive(Clone, Debug)]
pub(in crate::game) struct CostPaymentWindow {
    pub player: PlayerId,
    pub definition: ScopedEffect,
    pub object: Box<StackObject>,
    pub context: EffectResolutionContext,
    pub path: Vec<usize>,
    /// Tentative members of an aggregate-constrained selection. None have
    /// been consumed; cancellation discards only this selection state.
    pub chosen: Vec<GameObjectId>,
    pub cumulative_upkeep_age: Option<u16>,
}

pub(in crate::game) const fn uses_cost_payment_window(cost: CostDef) -> bool {
    matches!(
        cost,
        CostDef::Named { .. }
            | CostDef::Choice(_)
            | CostDef::Sacrifice { .. }
            | CostDef::Discard { .. }
            | CostDef::Exile { .. }
    )
}

impl CostPaymentWindow {
    pub(in crate::game) fn selected_cost(&self) -> Option<(CostDef, Vec<MechanicId>)> {
        let mut cost = match self.definition.effect {
            crate::card::EffectDef::PayOr(definition) if self.cumulative_upkeep_age.is_none() => {
                definition.payment.cost
            }
            crate::card::EffectDef::CumulativeUpkeep(cost)
                if self.cumulative_upkeep_age.is_some() =>
            {
                cost
            }
            _ => return None,
        };
        let mut path = self.path.iter();
        let mut mechanics = Vec::new();
        loop {
            match cost {
                CostDef::Named {
                    mechanic,
                    cost: inner,
                } => {
                    mechanics.push(mechanic);
                    cost = *inner;
                }
                CostDef::Choice(choices) => match path.next() {
                    Some(index) => cost = *choices.get(*index)?,
                    None => break,
                },
                _ => break,
            }
        }
        if let Some(age) = self.cumulative_upkeep_age {
            cost = match cost {
                CostDef::Sacrifice {
                    object,
                    quantity: crate::card::CostQuantityDef::Fixed(count),
                } => CostDef::sacrifice(
                    object,
                    crate::card::CostQuantityDef::Fixed(count.saturating_mul(age)),
                ),
                CostDef::Discard {
                    object,
                    quantity: crate::card::CostQuantityDef::Fixed(count),
                } => CostDef::discard(
                    object,
                    crate::card::CostQuantityDef::Fixed(count.saturating_mul(age)),
                ),
                _ => return None,
            };
        }
        path.next().is_none().then_some((cost, mechanics))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(in crate::game) enum CostPaymentStep {
    Object(GameObjectId, CostDef),
    /// Separates two payment actions even when their primitive costs match.
    EndAction,
    CompleteMechanic(MechanicId),
}

impl CostPaymentStep {
    pub(in crate::game) const fn object(self) -> Option<GameObjectId> {
        match self {
            Self::Object(object, _) => Some(object),
            Self::CompleteMechanic(_) | Self::EndAction => None,
        }
    }
}
