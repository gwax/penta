//! Resolving payment windows use shared object selection and commit rules.
//! Tentative choices never consume resources.

use super::{CostDef, CostPaymentWindow};
use crate::card::{CostQuantityDef, EffectDef, PayOrDef, ZoneKind};
use crate::game::{
    BattlefieldExitCompletion, CardPartId, CommittedTriggerEvent, DecisionContinuation,
    DecisionOption, DecisionPreference, DecisionVisibility, DecisionZone, Game,
    ObjectCharacteristics, PlayerId,
};
use crate::ids::GameObjectId;

pub(in crate::game) struct CostPaymentOffer {
    pub options: Vec<DecisionOption>,
    pub count: usize,
    pub cancellable: bool,
}

impl Game {
    fn can_pay_action_cost(&self, player: PlayerId, source: GameObjectId, cost: CostDef) -> bool {
        match cost {
            CostDef::Named { cost, .. } => self.can_pay_action_cost(player, source, *cost),
            CostDef::Choice(costs) => costs
                .iter()
                .any(|cost| self.can_pay_action_cost(player, source, *cost)),
            cost => cost.object_selection().is_some_and(|(_, _, quantity)| {
                self.object_selection_is_payable(
                    &self.object_cost_candidates(player, source, cost),
                    quantity,
                )
            }),
        }
    }

    pub(in crate::game) fn cost_payment_offer(
        &self,
        window: &CostPaymentWindow,
    ) -> Option<CostPaymentOffer> {
        let (cost, _) = window.selected_cost()?;
        let source = window.object.source.unwrap_or(window.object.id);
        if !self.can_pay_action_cost(window.player, source, cost) {
            return None;
        }
        if let CostDef::Choice(choices) = cost {
            if !window.chosen.is_empty() {
                return None;
            }
            let mut options = vec![plain_option(0, "Decline".into())];
            for (index, choice) in choices.iter().enumerate() {
                if self.can_pay_action_cost(window.player, source, *choice) {
                    options.push(plain_option(
                        u32::try_from(index + 1).ok()?,
                        action_cost_label(*choice),
                    ));
                }
            }
            return Some(CostPaymentOffer {
                options,
                count: 1,
                cancellable: false,
            });
        }
        let (_, from, quantity) = cost.object_selection()?;
        let candidates = self.object_cost_candidates(window.player, source, cost);
        if !window
            .chosen
            .iter()
            .enumerate()
            .all(|(index, id)| candidates.contains(id) && !window.chosen[..index].contains(id))
        {
            return None;
        }
        let threshold = matches!(quantity, CostQuantityDef::ObjectSetValueAtLeast(_));
        if !threshold && !window.chosen.is_empty() {
            return None;
        }
        let mut options = Vec::new();
        if threshold && self.object_selection_is_valid(&candidates, &window.chosen, quantity) {
            options.push(plain_option(0, "Pay selected objects".into()));
        }
        options.extend(
            self.object_cost_options(&candidates, from)
                .into_iter()
                .filter(|option| {
                    option
                        .card
                        .is_none_or(|(id, _)| !window.chosen.contains(&id))
                }),
        );
        Some(CostPaymentOffer {
            options,
            count: if threshold {
                1
            } else {
                usize::from(quantity.fixed_value()?)
            },
            cancellable: true,
        })
    }

    pub(in crate::game) fn object_cost_options(
        &self,
        candidates: &[GameObjectId],
        from: ZoneKind,
    ) -> Vec<DecisionOption> {
        let zone = match from {
            ZoneKind::Hand => DecisionZone::Hand,
            ZoneKind::Graveyard => DecisionZone::Graveyard,
            ZoneKind::Battlefield => DecisionZone::Battlefield,
            _ => unreachable!("unsupported object-payment zone"),
        };
        candidates
            .iter()
            .enumerate()
            .filter_map(|(index, id)| {
                let characteristics = self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == *id)
                    .map(Self::effective_rules_source)
                    .or_else(|| {
                        self.card_in_nonbattlefield_zone(*id).map(|(_, card)| {
                            ObjectCharacteristics::card(card.definition, CardPartId::PRIMARY)
                        })
                    })?;
                Some(DecisionOption {
                    id: u32::try_from(index + 1).ok()?,
                    label: self.characteristics_name(characteristics)?.into_owned(),
                    card: Some((*id, characteristics)),
                    members: Vec::new(),
                    ability_text: None,
                    zone,
                })
            })
            .collect()
    }

    pub(in crate::game) fn queue_cost_payment_window(&mut self, window: CostPaymentWindow) {
        let Some(offer) = self.cost_payment_offer(&window) else {
            self.finish_cost_payment_window(window, false);
            return;
        };
        let visibility = window.visibility();
        let source = window.object.source;
        self.queue_decision(
            window.player,
            window.object.ability_text().unwrap_or("Pay the cost?"),
            visibility,
            DecisionPreference::Neutral,
            offer.count..=offer.count,
            offer.cancellable,
            offer.options,
            DecisionContinuation::CostPayment(Box::new(window)),
        );
        if let Some(decision) = self.pending_decisions.last_mut() {
            decision.observation.source = source;
        }
    }

    pub(in crate::game) fn resolve_cost_payment_window(
        &mut self,
        mut window: CostPaymentWindow,
        selected: &[u32],
        options: &[DecisionOption],
    ) {
        let Some((cost, _)) = window.selected_cost() else {
            return;
        };
        let source = window.object.source.unwrap_or(window.object.id);
        if let CostDef::Choice(choices) = cost {
            if let [selected] = selected
                && let Some(index) = selected
                    .checked_sub(1)
                    .and_then(|index| usize::try_from(index).ok())
                && choices
                    .get(index)
                    .is_some_and(|cost| self.can_pay_action_cost(window.player, source, *cost))
            {
                window.path.push(index);
                self.queue_cost_payment_window(window);
            } else {
                self.finish_cost_payment_window(window, false);
            }
            return;
        }
        let (_, _, quantity) = cost.object_selection().expect("a supported object cost");
        let mut members = selected
            .iter()
            .filter_map(|selected| {
                options
                    .iter()
                    .find(|option| option.id == *selected)
                    .and_then(|option| option.card.map(|(id, _)| id))
            })
            .collect::<Vec<_>>();
        let candidates = self.object_cost_candidates(window.player, source, cost);
        if matches!(quantity, CostQuantityDef::ObjectSetValueAtLeast(_)) {
            if selected == [0] {
                members.clone_from(&window.chosen);
            } else if let [id] = members.as_slice()
                && selected.len() == 1
                && candidates.contains(id)
                && !window.chosen.contains(id)
            {
                window.chosen.push(*id);
                self.queue_cost_payment_window(window);
                return;
            } else {
                self.finish_cost_payment_window(window, false);
                return;
            }
        } else if selected.len() != members.len() {
            self.finish_cost_payment_window(window, false);
            return;
        }
        if !self.object_selection_is_valid(&candidates, &members, quantity) {
            self.finish_cost_payment_window(window, false);
            return;
        }
        match cost {
            CostDef::Sacrifice { .. } => {
                self.capture_sacrifices(&members);
                self.move_permanents_to_graveyard_then(
                    &members,
                    Some(BattlefieldExitCompletion::CompleteResolvingCost(Box::new(
                        window,
                    ))),
                );
            }
            CostDef::Discard { .. }
            | CostDef::Exile {
                from: ZoneKind::Hand | ZoneKind::Graveyard,
                ..
            } => {
                self.pay_object_card_cost(window.player, cost, &members);
                self.finish_cost_payment_window(window, true);
            }
            _ => unreachable!("unsupported action cost cannot open a payment window"),
        }
    }

    pub(in crate::game) fn finish_cost_payment_window(
        &mut self,
        window: CostPaymentWindow,
        paid: bool,
    ) {
        if let Some(age) = window.cumulative_upkeep_age {
            if paid {
                self.capture_optional_effect_taken(&window.object);
                self.capture_cumulative_upkeep_paid(&window.object, window.player, age, &[]);
            } else {
                self.capture_cumulative_upkeep_not_paid(&window.object, window.player, age);
                self.resolve_nested_effect_before_later(
                    window.definition.with_effect(EffectDef::Sacrifice {
                        object: crate::card::EffectRecipientDef::Source,
                    }),
                    &window.object,
                    window.context,
                );
            }
            return;
        }
        let EffectDef::PayOr(PayOrDef {
            if_paid, otherwise, ..
        }) = window.definition.effect
        else {
            unreachable!()
        };
        if paid {
            let (_, mechanics) = window
                .selected_cost()
                .expect("validated authored cost path");
            for mechanic in mechanics.into_iter().rev() {
                self.capture_battlefield_triggers(&CommittedTriggerEvent::MechanicPerformed {
                    mechanic,
                    player: window.player,
                    object: None,
                });
            }
            self.capture_optional_effect_taken(&window.object);
        }
        if let Some(effect) = if paid { if_paid } else { otherwise } {
            let mut context = window.context;
            context.paid_amount = paid.then_some(0);
            self.resolve_nested_effect_before_later(
                window.definition.with_effect(*effect),
                &window.object,
                context,
            );
        }
    }
}

impl CostPaymentWindow {
    pub(in crate::game) fn visibility(&self) -> DecisionVisibility {
        if self.cumulative_upkeep_age.is_some() {
            return DecisionVisibility::Private;
        }
        let EffectDef::PayOr(definition) = self.definition.effect else {
            unreachable!()
        };
        if cost_has_private_selection(definition.payment.cost) {
            DecisionVisibility::Private
        } else {
            crate::game::decision_offers::effect_choice_visibility(definition.visibility)
        }
    }
}

fn cost_has_private_selection(cost: CostDef) -> bool {
    match cost {
        CostDef::Named { cost, .. } => cost_has_private_selection(*cost),
        CostDef::Choice(costs) => costs.iter().any(|cost| cost_has_private_selection(*cost)),
        cost => cost
            .object_selection()
            .is_some_and(|(_, zone, _)| zone == ZoneKind::Hand),
    }
}

fn plain_option(id: u32, label: String) -> DecisionOption {
    DecisionOption {
        id,
        label,
        card: None,
        members: Vec::new(),
        ability_text: None,
        zone: DecisionZone::None,
    }
}

fn action_cost_label(cost: CostDef) -> String {
    match cost {
        CostDef::Named { cost, .. } => action_cost_label(*cost),
        CostDef::Sacrifice {
            quantity: CostQuantityDef::Fixed(count),
            ..
        } => format!("Sacrifice {count} permanent(s)"),
        CostDef::Exile {
            from: ZoneKind::Graveyard,
            quantity: CostQuantityDef::Fixed(count),
            ..
        } => format!("Exile {count} card(s) from your graveyard"),
        CostDef::Sacrifice { .. } => "Sacrifice matching permanents".into(),
        CostDef::Discard { .. } => "Discard matching cards".into(),
        CostDef::Exile {
            from: ZoneKind::Hand,
            ..
        } => "Exile cards from your hand".into(),
        CostDef::Exile { .. } => "Exile cards from your graveyard".into(),
        CostDef::Choice(_) => "Choose a payment".into(),
        _ => unreachable!("unsupported action cost"),
    }
}
