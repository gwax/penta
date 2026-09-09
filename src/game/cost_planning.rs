//! Complete scalar obligations shared by casting and resolving payments.
use super::{ManaCost, add_mana_cost};
use crate::card::CostDef;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ScalarCostPlan {
    pub mana: ManaCost,
    pub life: u16,
    pub includes_mana_payment: bool,
}

impl ScalarCostPlan {
    fn free() -> Self {
        Self {
            mana: ManaCost::default(),
            life: 0,
            includes_mana_payment: false,
        }
    }
    fn combine(self, other: Self) -> Self {
        Self {
            mana: add_mana_cost(self.mana, other.mana),
            life: self.life.saturating_add(other.life),
            includes_mana_payment: self.includes_mana_payment || other.includes_mana_payment,
        }
    }
}

/// `None` means the program needs a non-scalar planner. An empty vector means
/// there is no complete way to pay. Each repetition makes its own choices.
pub(super) fn scalar_cost_plans(cost: CostDef, repetitions: u16) -> Option<Vec<ScalarCostPlan>> {
    fn combine(left: &[ScalarCostPlan], right: &[ScalarCostPlan]) -> Vec<ScalarCostPlan> {
        let mut result = Vec::new();
        for left in left {
            for right in right {
                let plan = left.combine(*right);
                if !result.contains(&plan) {
                    result.push(plan);
                }
            }
        }
        result
    }
    let ways = match cost {
        CostDef::Mana(mana) if !mana.variable_x => vec![ScalarCostPlan {
            mana,
            life: 0,
            includes_mana_payment: true,
        }],
        CostDef::PayLife(life) => vec![ScalarCostPlan {
            mana: ManaCost::default(),
            life,
            includes_mana_payment: false,
        }],
        CostDef::Choice(costs) => {
            let mut ways = Vec::new();
            for cost in costs {
                ways.extend(scalar_cost_plans(*cost, 1)?);
            }
            ways
        }
        CostDef::All(costs) => {
            let mut ways = vec![ScalarCostPlan::free()];
            for cost in costs {
                ways = combine(&ways, &scalar_cost_plans(*cost, 1)?);
            }
            ways
        }
        _ => return None,
    };
    let mut plans = vec![ScalarCostPlan::free()];
    for _ in 0..repetitions {
        plans = combine(&plans, &ways);
    }
    Some(plans)
}
