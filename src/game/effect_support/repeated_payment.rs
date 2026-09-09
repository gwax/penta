impl Game {
    pub(in crate::game) fn resolved_repeated_payment(
        cost: crate::card::CostDef,
        source: GameObjectId,
        times: u16,
        label: Option<crate::card::AbilityLabel>,
    ) -> crate::game::ResolvedEffectPayment {
        use crate::card::CostDef as Cost;
        use crate::game::ResolvedEffectPayment as Resolved;

        let repeated = |amount: u16| amount.saturating_mul(times);
        match cost {
            Cost::All(costs) => Resolved::all(
                costs
                    .iter()
                    .map(|cost| Self::resolved_repeated_payment(*cost, source, times, label))
                    .collect(),
            ),
            Cost::Choice(_) => Self::resolved_repeated_scalar_choices(cost, source, times, label),
            Cost::Mana(cost) => match label {
                Some(label) => Resolved::LabeledMana {
                    source,
                    label,
                    cost: repeat_mana_cost(cost, times),
                },
                None => Resolved::Mana(repeat_mana_cost(cost, times)),
            },
            Cost::SnowMana(amount) => Resolved::SnowMana {
                label,
                source,
                amount: repeated(amount),
            },
            Cost::Energy(amount) => Resolved::Energy(repeated(amount)),
            Cost::MillCards(amount) => Resolved::Mill(repeated(amount)),
            Cost::PayLife(amount) => Resolved::Life(repeated(amount)),
            Cost::DrawCards(amount) => Resolved::DrawCards(repeated(amount)),
            Cost::DiscardCards(amount) => Resolved::DiscardCards(repeated(amount)),
            Cost::PutCountersOnSource { kind, amount } => Resolved::PutCounters {
                object: source,
                kind,
                amount,
                times,
            },
            Cost::SacrificePermanents {
                object,
                controller: crate::card::PlayerRelation::You,
                count,
            } => Resolved::SacrificePermanents {
                object,
                amount: repeated(u16::from(count)),
            },
            Cost::ExileTopCards(amount) => Resolved::ExileTopCards(repeated(amount)),
            Cost::AddMana(effect) => {
                let crate::card::ManaSelectionDef::One(crate::card::ManaTypeDef::Fixed(color)) =
                    effect.mana
                else {
                    panic!("unsupported repeated payment mana output")
                };
                assert!(
                    effect.also.is_none()
                        && effect.variable_amount.is_none()
                        && effect.amount_override.is_none()
                        && effect.damage_to_controller == 0
                        && effect.sacrifice_source_when_out_of.is_none()
                        && effect.restrictions.is_empty()
                        && effect.spend_effects.is_empty(),
                    "unsupported repeated payment mana output",
                );
                Resolved::AddMana {
                    color,
                    amount: repeated(effect.amount),
                }
            }
            Cost::GainLife {
                player: crate::card::PlayerRelation::Opponent,
                amount,
            } => Resolved::OpponentGainsLife(repeated(amount)),
            Cost::CreateTokens {
                player: crate::card::PlayerRelation::Opponent,
                token,
                amount,
            } => Resolved::OpponentCreatesTokens {
                token: *token,
                amount: repeated(amount),
            },
            Cost::GainControlPermanents { object, amount } => Resolved::GainControlPermanents {
                source,
                object,
                amount: repeated(amount),
            },
            Cost::FlipCoins(amount) => Resolved::FlipCoins(repeated(amount)),
            _ => panic!("unsupported repeated payment cost"),
        }
    }
}

fn repeat_mana_cost(mut cost: crate::ManaCost, count: u16) -> crate::ManaCost {
    cost.generic = cost.generic.saturating_mul(count);
    cost.white = cost.white.saturating_mul(count);
    cost.blue = cost.blue.saturating_mul(count);
    cost.black = cost.black.saturating_mul(count);
    cost.red = cost.red.saturating_mul(count);
    cost.green = cost.green.saturating_mul(count);
    cost.colorless = cost.colorless.saturating_mul(count);
    for amount in &mut cost.hybrid {
        *amount = amount.saturating_mul(count);
    }
    for amount in &mut cost.additional_flexible {
        *amount = amount.saturating_mul(count);
    }
    cost.x_multiplier = cost.x_multiplier.saturating_mul(count);
    cost
}

impl Game {
    pub(in crate::game) fn resolve_payment_offer(
        &self,
        definition: crate::card::PayOrDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) -> (
        super::ResolvedEffectPayment,
        Option<super::PaymentProvenance>,
    ) {
        let costs = definition
            .payment
            .costs
            .iter()
            .flat_map(|cost| {
                if matches!(cost, crate::card::CostDef::Parameter) {
                    scoped
                        .cost_parameter
                        .expect("validated lexical cost parameter")
                        .to_vec()
                } else {
                    vec![*cost]
                }
            })
            .collect::<Vec<_>>();
        let repetitions = definition.repeat.map_or(1, |value| {
            u16::try_from(self.effect_value(*value, object, context, scoped).max(0))
                .unwrap_or(u16::MAX)
        });
        let provenance = definition
            .label
            .map(|label| super::PaymentProvenance { label, repetitions });
        let payment = if definition.repeat.is_some() || definition.label.is_some() {
            super::ResolvedEffectPayment::all(
                costs
                    .iter()
                    .map(|cost| {
                        Self::resolved_repeated_payment(
                            *cost,
                            object.source.unwrap_or(object.id),
                            repetitions,
                            definition.label,
                        )
                    })
                    .collect(),
            )
        } else {
            self.resolved_effect_costs(&costs, object, context, scoped)
        };
        (payment, provenance)
    }
}

impl Game {
    fn resolved_repeated_scalar_choices(
        cost: crate::card::CostDef,
        source: GameObjectId,
        times: u16,
        label: Option<crate::card::AbilityLabel>,
    ) -> crate::game::ResolvedEffectPayment {
        use crate::card::CostDef as Cost;
        use crate::game::ResolvedEffectPayment as Resolved;
        Resolved::Choice(
            crate::game::cost_planning::scalar_cost_plans(cost, times)
                .expect("validated scalar alternatives")
                .into_iter()
                .map(|plan| {
                    let mut payments = Vec::new();
                    if plan.includes_mana_payment {
                        payments.push(Self::resolved_repeated_payment(
                            Cost::Mana(plan.mana),
                            source,
                            1,
                            label,
                        ));
                    }
                    if plan.life > 0 {
                        payments.push(Resolved::Life(plan.life));
                    }
                    Resolved::all(payments)
                })
                .collect(),
        )
    }
}
