//! Bounded resolving action-cost choices. This first lane accepts named
//! choices of fixed public-zone object payments. Bundles and hidden/random
//! action costs remain outside this lane until their joint planning exists.

use super::{CostDef, CostPaymentWindow};
use crate::card::{CostQuantityDef, EffectDef, PayOrDef, ZoneKind};
use crate::game::{
    BattlefieldExitCompletion, CardPartId, CommittedTriggerEvent, DecisionContinuation,
    DecisionOption, DecisionPreference, DecisionZone, Game, ObjectCharacteristics, PlayerId,
};
use crate::ids::GameObjectId;

pub(in crate::game) struct CostPaymentOffer {
    pub options: Vec<DecisionOption>,
    pub count: usize,
    pub cancellable: bool,
}

impl Game {
    fn action_cost_candidates(&self, player: PlayerId, cost: CostDef) -> Vec<GameObjectId> {
        match cost {
            CostDef::Sacrifice { object, .. } => {
                self.matching_permanents_controlled(player, object)
            }
            CostDef::Exile {
                object,
                from: ZoneKind::Graveyard,
                ..
            } => self.players[player.index()]
                .graveyard
                .iter()
                .filter(|card| self.card_object_matches(object, card, ZoneKind::Graveyard, card.id))
                .map(|card| card.id)
                .collect(),
            _ => Vec::new(),
        }
    }

    fn can_pay_action_cost(&self, player: PlayerId, cost: CostDef) -> bool {
        match cost {
            CostDef::Named { cost, .. } => self.can_pay_action_cost(player, *cost),
            CostDef::Choice(costs) => costs
                .iter()
                .any(|cost| self.can_pay_action_cost(player, *cost)),
            CostDef::Sacrifice {
                quantity: CostQuantityDef::Fixed(count),
                ..
            }
            | CostDef::Exile {
                quantity: CostQuantityDef::Fixed(count),
                ..
            } => self.action_cost_candidates(player, cost).len() >= usize::from(count),
            _ => false,
        }
    }

    pub(in crate::game) fn cost_payment_offer(
        &self,
        window: &CostPaymentWindow,
    ) -> Option<CostPaymentOffer> {
        let (cost, _) = window.selected_cost()?;
        if !self.can_pay_action_cost(window.player, cost) {
            return None;
        }
        if let CostDef::Choice(choices) = cost {
            let mut options = vec![DecisionOption {
                id: 0,
                label: "Decline".into(),
                card: None,
                members: Vec::new(),
                ability_text: None,
                zone: DecisionZone::None,
            }];
            for (index, choice) in choices.iter().enumerate() {
                if self.can_pay_action_cost(window.player, *choice) {
                    options.push(DecisionOption {
                        id: u32::try_from(index + 1).ok()?,
                        label: action_cost_label(*choice),
                        card: None,
                        members: Vec::new(),
                        ability_text: None,
                        zone: DecisionZone::None,
                    });
                }
            }
            return Some(CostPaymentOffer {
                options,
                count: 1,
                cancellable: false,
            });
        }
        let (count, zone) = match cost {
            CostDef::Sacrifice {
                quantity: CostQuantityDef::Fixed(count),
                ..
            } => (count, DecisionZone::Battlefield),
            CostDef::Exile {
                quantity: CostQuantityDef::Fixed(count),
                from: ZoneKind::Graveyard,
                ..
            } => (count, DecisionZone::Graveyard),
            _ => return None,
        };
        let options = self
            .action_cost_candidates(window.player, cost)
            .into_iter()
            .enumerate()
            .filter_map(|(index, id)| {
                let characteristics = self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == id)
                    .map(Self::effective_rules_source)
                    .or_else(|| {
                        self.card_in_nonbattlefield_zone(id).map(|(_, card)| {
                            ObjectCharacteristics::card(card.definition, CardPartId::PRIMARY)
                        })
                    })?;
                Some(DecisionOption {
                    id: u32::try_from(index + 1).ok()?,
                    label: self.characteristics_name(characteristics)?.into_owned(),
                    card: Some((id, characteristics)),
                    members: Vec::new(),
                    ability_text: None,
                    zone,
                })
            })
            .collect();
        Some(CostPaymentOffer {
            options,
            count: usize::from(count),
            cancellable: true,
        })
    }

    pub(in crate::game) fn queue_cost_payment_window(&mut self, window: CostPaymentWindow) {
        let Some(offer) = self.cost_payment_offer(&window) else {
            self.finish_cost_payment_window(window, false);
            return;
        };
        let EffectDef::PayOr(definition) = window.definition.effect else {
            unreachable!()
        };
        let source = window.object.source;
        self.queue_decision(
            window.player,
            window.object.ability_text().unwrap_or("Pay the cost?"),
            crate::game::decision_offers::effect_choice_visibility(definition.visibility),
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
        if let CostDef::Choice(choices) = cost {
            if let [selected] = selected
                && let Some(index) = selected
                    .checked_sub(1)
                    .and_then(|index| usize::try_from(index).ok())
                && choices
                    .get(index)
                    .is_some_and(|cost| self.can_pay_action_cost(window.player, *cost))
            {
                window.path.push(index);
                self.queue_cost_payment_window(window);
            } else {
                self.finish_cost_payment_window(window, false);
            }
            return;
        }
        let members = selected
            .iter()
            .filter_map(|selected| {
                options
                    .iter()
                    .find(|option| option.id == *selected)
                    .and_then(|option| option.card.map(|(id, _)| id))
            })
            .collect::<Vec<_>>();
        // Revalidate every selected resource before the first mutation.
        let valid = self.cost_payment_offer(&window).is_some_and(|offer| {
            selected.len() == offer.count
                && members.len() == offer.count
                && members.iter().enumerate().all(|(index, id)| {
                    !members[..index].contains(id)
                        && offer.options.iter().any(|option| {
                            option.card.is_some_and(|(candidate, _)| candidate == *id)
                        })
                })
        });
        if !valid {
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
            CostDef::Exile {
                from: ZoneKind::Graveyard,
                ..
            } => {
                let mut exiled = Vec::new();
                for id in members {
                    let card = crate::game::remove_card(
                        &mut self.players[window.player.index()].graveyard,
                        id,
                    )
                    .expect("the whole selection was validated before payment");
                    let (card, _) = self.zone_change_card(card);
                    self.players[window.player.index()].exile.push(card.clone());
                    exiled.push(card);
                }
                self.capture_cards_exiled(&exiled, ZoneKind::Graveyard);
                self.note_card_left_graveyard(window.player);
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

fn action_cost_label(cost: CostDef) -> String {
    match cost {
        CostDef::Named { cost, .. } => action_cost_label(*cost),
        CostDef::Sacrifice {
            quantity: CostQuantityDef::Fixed(count),
            ..
        } => format!("Sacrifice {count} permanent(s)"),
        CostDef::Exile {
            quantity: CostQuantityDef::Fixed(count),
            ..
        } => format!("Exile {count} card(s) from your graveyard"),
        CostDef::Choice(_) => "Choose a payment".into(),
        _ => unreachable!("unsupported action cost"),
    }
}
