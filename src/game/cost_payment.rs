//! Semantic payment steps, including named-action completion boundaries.

use crate::card::CostDef;
use crate::ids::{GameObjectId, MechanicId};

use super::{EffectResolutionContext, PlayerId, ScopedEffect, StackObject};

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
}

pub(in crate::game) const fn uses_cost_payment_window(cost: CostDef) -> bool {
    matches!(
        cost,
        CostDef::Named { .. }
            | CostDef::Choice(_)
            | CostDef::Sacrifice { .. }
            | CostDef::Exile { .. }
    )
}

impl CostPaymentWindow {
    pub(in crate::game) fn selected_cost(&self) -> Option<(CostDef, Vec<MechanicId>)> {
        let crate::card::EffectDef::PayOr(definition) = self.definition.effect else {
            return None;
        };
        let mut cost = definition.payment.cost;
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
