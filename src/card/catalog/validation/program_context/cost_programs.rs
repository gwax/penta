// A parameter is valid only beneath its lexical supplier. Keep this check
// separate from effect traversal so copied/granted programs get the same rules.
fn validate_cost_program(
    effect: EffectDef,
    parameter: Option<&'static [CostDef]>,
) -> Result<(), &'static str> {
    if let EffectDef::WithCosts { costs, effect } = effect {
        if costs.iter().any(|cost| contains_cost_parameter(*cost)) {
            return Err("WithCosts cannot bind an unresolved parameter");
        }
        return validate_cost_program(*effect, Some(costs));
    }
    if let EffectDef::PayOr(payment) = effect {
        for cost in payment.payment.costs {
            let costs = if matches!(cost, CostDef::Parameter) {
                parameter.ok_or("unbound cost parameter")?
            } else {
                if contains_cost_parameter(*cost) {
                    return Err("cost parameters must be whole list components");
                }
                std::slice::from_ref(cost)
            };
            if (payment.repeat.is_some() || payment.label.is_some())
                && !costs.iter().all(|cost| repeatable_payment_cost(*cost))
            {
                return Err("unsupported repeated or labeled payment cost");
            }
        }
    }
    for child in crate::card::child_effects(effect) {
        validate_cost_program(child, parameter)?;
    }
    Ok(())
}

fn contains_cost_parameter(cost: CostDef) -> bool {
    match cost {
        CostDef::Parameter => true,
        CostDef::All(costs) | CostDef::Choice(costs) => {
            costs.iter().any(|cost| contains_cost_parameter(*cost))
        }
        _ => false,
    }
}

fn repeatable_payment_cost(cost: CostDef) -> bool {
    match cost {
        CostDef::All(costs) => costs.iter().all(|cost| repeatable_payment_cost(*cost)),
        CostDef::Choice(_) => scalar_batch_cost(cost),
        CostDef::Mana(cost) => !cost.variable_x,
        CostDef::SnowMana(_)
        | CostDef::PayLife(_)
        | CostDef::Energy(_)
        | CostDef::MillCards(_)
        | CostDef::DrawCards(_)
        | CostDef::DiscardCards(_)
        | CostDef::ExileTopCards(_)
        | CostDef::FlipCoins(_)
        | CostDef::PutCountersOnSource { .. }
        | CostDef::GainControlPermanents { .. }
        | CostDef::SacrificePermanents {
            controller: PlayerRelation::You,
            ..
        }
        | CostDef::GainLife {
            player: PlayerRelation::Opponent,
            ..
        }
        | CostDef::CreateTokens {
            player: PlayerRelation::Opponent,
            ..
        } => true,
        CostDef::AddMana(effect) => {
            matches!(
                effect.mana,
                crate::card::ManaSelectionDef::One(crate::card::ManaTypeDef::Fixed(_))
            ) && effect.also.is_none()
                && effect.variable_amount.is_none()
                && effect.amount_override.is_none()
                && effect.damage_to_controller == 0
                && effect.sacrifice_source_when_out_of.is_none()
                && effect.restrictions.is_empty()
                && effect.spend_effects.is_empty()
        }
        _ => false,
    }
}

fn scalar_batch_cost(cost: CostDef) -> bool {
    match cost {
        CostDef::Mana(cost) => !cost.variable_x,
        CostDef::PayLife(_) => true,
        CostDef::All(costs) | CostDef::Choice(costs) => {
            costs.iter().all(|cost| scalar_batch_cost(*cost))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{AbilityLabel, PayOrDef};

    const SCALAR_CHOICES: &[CostDef] = &[
        CostDef::PayLife(1),
        CostDef::Mana(crate::ManaCost::new(1, 0)),
    ];

    static PAY_PARAMETER: EffectDef = EffectDef::PayOr(
        PayOrDef::optional(&[CostDef::Parameter], &EffectDef::None)
            .labeled(AbilityLabel("test purpose"))
            .repeated(&ValueDef::Constant(2))
            .with_visibility(crate::card::ChoiceVisibilityDef::Public),
    );

    #[test]
    fn composed_mechanic_programs_reject_unbound_and_unpayable_programs() {
        assert!(validate_cost_program(PAY_PARAMETER, None).is_err());
        assert!(
            validate_cost_program(
                EffectDef::WithCosts {
                    costs: &[CostDef::Choice(&[
                        CostDef::PayLife(1),
                        CostDef::DrawCards(1)
                    ])],
                    effect: &PAY_PARAMETER,
                },
                None
            )
            .is_err()
        );
        assert!(
            validate_cost_program(
                EffectDef::WithCosts {
                    costs: &[CostDef::Choice(SCALAR_CHOICES)],
                    effect: &PAY_PARAMETER,
                },
                None
            )
            .is_ok()
        );
        assert!(
            validate_cost_program(
                EffectDef::WithCosts {
                    costs: &[CostDef::PayLife(1)],
                    effect: &PAY_PARAMETER
                },
                None
            )
            .is_ok()
        );
        assert!(
            validate_cost_program(
                EffectDef::WithCosts {
                    costs: &[CostDef::Special("unsupported")],
                    effect: &PAY_PARAMETER
                },
                None
            )
            .is_err()
        );
    }
}
