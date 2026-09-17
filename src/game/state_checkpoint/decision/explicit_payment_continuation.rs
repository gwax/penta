// Restoring an explicit payment's selected units and funding operation.
#[allow(clippy::too_many_lines)]
fn parse_explicit_payment_continuation(
    value: &DecisionContinuationSnapshot,
    observation: &DecisionObservation,
    hidden: &Value,
    game: &Game,
) -> Result<DecisionContinuation, String> {
    Ok(match value {
        DecisionContinuationSnapshot::ExplicitFunding { draft } => {
            let draft = parse_payment_draft(game, draft, hidden)?;
            DecisionContinuation::Payment(PaymentDecision::Funding(draft))
        }
        DecisionContinuationSnapshot::ExplicitDraftMana {
            draft,
            action,
            units,
        } => {
            let draft = parse_payment_draft(game, draft, hidden)?;
            if draft.player != observation.player {
                return Err("payment draft has the wrong player".into());
            }
            let (preview, frame) = game
                .preview_funding(&draft)
                .ok_or("payment draft cannot be prepared")?;
            let (target, obligation) = if let Some(index) = action {
                let action = game
                    .funding_candidates(&draft)
                    .and_then(|actions| actions.get(*index).cloned())
                    .ok_or("funding ability is unavailable")?;
                let obligation = preview
                    .explicit_payment_obligation(draft.player, &action)
                    .ok_or("funding ability has no mana cost")?;
                (
                    PaymentTarget::Funding {
                        draft,
                        action: Box::new(action),
                    },
                    obligation,
                )
            } else {
                (PaymentTarget::Draft(draft), frame.obligation)
            };
            if !preview.mana_selection_can_complete(&obligation, units) {
                return Err("payment draft mana selection is invalid".into());
            }
            DecisionContinuation::Payment(PaymentDecision::Mana {
                target,
                obligation: Box::new(obligation),
                selected: units.clone(),
            })
        }

        DecisionContinuationSnapshot::ExplicitPayment {
            player: chooser,
            x,
            action,
            resume,
            effect_choice,
            units,
            allocations,
        } => {
            use crate::game::payment::state::{PaymentDecision, PaymentTarget};
            let player = player(*chooser)?;
            let resume = resume
                .as_ref()
                .map(|snapshot| {
                    parse_pending_decision(
                        &serde_json::json!({ "decision": snapshot.observation }),
                        Some(&snapshot.state),
                        hidden,
                        game,
                    )?
                    .map(Box::new)
                    .ok_or_else(|| "explicit payment has no enclosing decision".to_owned())
                })
                .transpose()?;
            DecisionContinuation::Payment(if let Some(answered) = effect_choice {
                if action.is_some() {
                    return Err("explicit effect payment also names an action".into());
                }
                let pending = resume.ok_or("explicit effect payment has no continuation")?;
                if answered.len() != 1
                    || pending.observation.player != player
                    || !game
                        .explicit_effect_choices(&pending)
                        .contains(&answered[0])
                {
                    return Err("explicit effect payment has an invalid answer".into());
                }
                let obligation = answered
                    .first()
                    .and_then(|chosen| game.effect_payment_obligations(&pending, *chosen))
                    .and_then(|bills| bills.get(allocations.len()).cloned())
                    .ok_or("explicit effect payment has no mana obligation")?;
                PaymentDecision::Mana {
                    target: PaymentTarget::Effect {
                        pending,
                        answered: answered.clone(),
                        allocations: allocations
                            .iter()
                            .map(|units| BoundManaPayment {
                                units: units.clone(),
                            })
                            .collect(),
                    },
                    obligation: Box::new(obligation),
                    selected: units.clone(),
                }
            } else if let Some(index) = action {
                let action = game
                    .manual_payment_actions_in(player, resume.as_deref())
                    .get(*index)
                    .cloned()
                    .ok_or("explicit payment operation is unavailable")?;
                let obligation = game
                    .explicit_payment_obligation(player, &action)
                    .ok_or("explicit payment operation has no cost")?;
                PaymentDecision::Mana {
                    target: PaymentTarget::Action {
                        action: Box::new(action),
                        resume,
                    },
                    obligation: Box::new(obligation),
                    selected: units.clone(),
                }
            } else {
                PaymentDecision::Operation {
                    player,
                    x: *x,
                    resume,
                }
            })
        }
        _ => unreachable!("payment continuation dispatcher"),
    })
}
