//! Which combinations of costs a cast can be paid with.
//!
//! A cast is enumerated once per way of paying for it, so this is where the
//! additional costs a spell names, the alternative costs it offers, and the
//! mana each configuration ends up owing are turned into concrete
//! configurations for the caller to walk.

use super::super::{
    AdditionalCostId, AlternativeCastKindDef, AlternativeCostId, CardDefinition, CardInstance,
    CastSourceZone, ControlFlow, CostConfiguration, DeclarativeAbilityDef, ExilePlayCost, Game,
    GameObjectId, ManaCost, PlayOptionDef, PlayerId, TriggerContext, ZoneKind, add_mana_cost,
    configured_mana_cost,
};
use crate::card::SpellAdditionalCostDef;

/// The chosen quantities a cost can be counted from: the X the spell is cast
/// for, and how many modes it was cast with.
#[derive(Clone, Copy)]
pub(in crate::game) struct CastScale {
    pub(in crate::game) x: u16,
    pub(in crate::game) modes: usize,
}

impl Game {
    /// Every way to pay a spell's declarative additional cost. A spell with
    /// none has exactly one way to pay it: spend nothing. A spell with one it
    /// cannot afford has none at all, which is what stops it being offered.
    pub(in crate::game) fn additional_cost_choices(
        &self,
        definition: &CardDefinition,
        option: &PlayOptionDef,
        costs: &CostConfiguration,
        card: &CardInstance,
        player: PlayerId,
        scale: CastScale,
    ) -> Vec<Vec<GameObjectId>> {
        // A cost paid instead of the mana cost replaces the spell's own
        // additional cost rather than stacking with it: "rather than pay this
        // spell's mana cost" is the whole payment.
        let selected = costs
            .alternative()
            .and_then(|selected| Self::alternative_cast_ability(definition, option, selected))
            .and_then(|(_, ability, _)| match ability.definition {
                DeclarativeAbilityDef::AlternativeCast(alternative) => alternative.additional_cost,
                _ => None,
            });
        let cost = selected.or_else(|| {
            definition
                .rules
                .ability_clauses()
                .iter()
                .find_map(|ability| match ability.definition {
                    DeclarativeAbilityDef::Spell(spell) if ability.is_executable() => {
                        spell.additional_cost()
                    }
                    _ => None,
                })
        });
        let Some(cost) = cost else {
            return vec![Vec::new()];
        };
        // "Sacrifice a creature or discard a card" is one cost with two ways
        // to pay it, so the ways of paying are the union: each half is
        // enumerated over its own zone, and a half nothing can pay simply
        // contributes nothing.
        let mut payments = Vec::new();
        for alternative in cost.alternatives() {
            for payment in self.additional_cost_payments(alternative, card, player, scale) {
                if !payments.contains(&payment) {
                    payments.push(payment);
                }
            }
        }
        payments
    }

    /// Every way to pay one half of a spell's additional cost.
    fn additional_cost_payments(
        &self,
        cost: SpellAdditionalCostDef,
        card: &CardInstance,
        player: PlayerId,
        scale: CastScale,
    ) -> Vec<Vec<GameObjectId>> {
        let candidates: Vec<GameObjectId> = match cost.zone {
            ZoneKind::Battlefield => self
                .battlefield
                .iter()
                .filter(|permanent| {
                    permanent.controller == player
                        && self.trigger_object_matches(
                            cost.object,
                            &self.trigger_event_object(permanent),
                            permanent.card.id,
                            false,
                        )
                })
                .map(|permanent| permanent.card.id)
                .collect(),
            // The same exclusion as hand below, for the same reason: escape
            // and flashback are cast from the graveyard, so by the time the
            // cost is paid the card is on the stack and not there to spend.
            // This is what "exile five other cards" means.
            ZoneKind::Graveyard => self.players[player.index()]
                .graveyard
                .iter()
                .filter(|held| {
                    held.id != card.id
                        && self.card_object_matches(cost.object, held, ZoneKind::Graveyard, held.id)
                })
                .map(|held| held.id)
                .collect(),
            // The card paying the cost cannot be the spell itself: it has
            // already left hand by the time the cost is paid.
            ZoneKind::Hand => self.players[player.index()]
                .hand
                .iter()
                .filter(|held| {
                    held.id != card.id
                        && self.card_object_matches(cost.object, held, ZoneKind::Hand, held.id)
                })
                .map(|held| held.id)
                .collect(),
            _ => Vec::new(),
        };
        // One configuration per way of paying, so a cost naming more than one
        // object enumerates combinations rather than candidates. Order does
        // not matter -- exiling A then B is the same payment as B then A --
        // so each combination appears once, in candidate order.
        let required = match cost.counted {
            crate::card::SpellAdditionalCostCountDef::Printed => usize::from(cost.count),
            crate::card::SpellAdditionalCostCountDef::ChosenX => usize::from(scale.x),
            // Escalate: a spell with one mode pays nothing extra, and every
            // mode past the first costs another one of these.
            crate::card::SpellAdditionalCostCountDef::ModesBeyondFirst => {
                usize::from(cost.count).saturating_mul(scale.modes.saturating_sub(1))
            }
        };
        Self::object_combinations(&candidates, required)
    }

    /// Every `size`-element combination of `candidates`, in candidate order.
    /// An empty requirement has exactly one payment: the empty one.
    pub(in crate::game) fn object_combinations(
        candidates: &[GameObjectId],
        size: usize,
    ) -> Vec<Vec<GameObjectId>> {
        if size == 0 {
            return vec![Vec::new()];
        }
        if candidates.len() < size {
            return Vec::new();
        }
        let mut combinations = Vec::new();
        for (index, candidate) in candidates.iter().enumerate() {
            for mut rest in Self::object_combinations(&candidates[index + 1..], size - 1) {
                let mut combination = vec![*candidate];
                combination.append(&mut rest);
                combinations.push(combination);
            }
        }
        combinations
    }

    /// Whether an alternative of this kind may be used on a card being cast
    /// from this zone. What separates them is where the permission lets the
    /// card be cast from: flashback and escape are permissions to cast it
    /// where it lies, and everything else is an ordinary cast from hand paid
    /// for differently.
    fn alternative_is_castable_from(
        &self,
        source_zone: CastSourceZone,
        kind: Option<AlternativeCastKindDef>,
        card: GameObjectId,
    ) -> bool {
        match (source_zone, kind) {
            // Both are permission to cast the card where it lies,
            // which is nowhere else.
            (
                CastSourceZone::Hand,
                Some(AlternativeCastKindDef::Flashback | AlternativeCastKindDef::Escape),
            )
            | (
                CastSourceZone::Graveyard,
                Some(
            AlternativeCastKindDef::Overload
            | AlternativeCastKindDef::Miracle
            | AlternativeCastKindDef::Kicked
            | AlternativeCastKindDef::Buyback
            | AlternativeCastKindDef::AlternativeCost
            | AlternativeCastKindDef::Impending
            | AlternativeCastKindDef::FaceDown,
                )
                | None,
            )
            // A card coming back from an adventure is cast for what
            // its creature half prints, which is the permission the
            // adventure gave. Nothing else about it changes.
            | (CastSourceZone::Exile, _) => false,
            // A kicked spell, and one paid for some other way, are both
            // cast from hand like any other; only what they cost and what
            // they do are different.
            (
                CastSourceZone::Hand,
                Some(
            AlternativeCastKindDef::Overload
            | AlternativeCastKindDef::Kicked
            | AlternativeCastKindDef::Buyback
            | AlternativeCastKindDef::AlternativeCost
            | AlternativeCastKindDef::Impending
            // Face down is a way of casting the card from
            // hand, not a permission to cast it elsewhere.
            | AlternativeCastKindDef::FaceDown,
                )
                | None,
            )
            | (
                CastSourceZone::Graveyard,
                Some(AlternativeCastKindDef::Flashback | AlternativeCastKindDef::Escape),
            ) => true,
            // Only in the window the draw opened, and only for the card
            // that was drawn.
            (CastSourceZone::Hand, Some(AlternativeCastKindDef::Miracle)) => {
                self.miracle_window == Some(card)
            }
        }
    }

    pub(in crate::game) fn visit_cost_configurations(
        &self,
        definition: &CardDefinition,
        card: GameObjectId,
        player: PlayerId,
        option: &PlayOptionDef,
        source_zone: CastSourceZone,
        mut visitor: impl FnMut(CostConfiguration) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        let mut selected_additional = Vec::with_capacity(option.additional_costs.len());
        if matches!(source_zone, CastSourceZone::Hand | CastSourceZone::Exile)
            && Self::visit_additional_cost_configurations(
                option,
                None,
                option.additional_costs.len(),
                &mut selected_additional,
                &mut visitor,
            )
            .is_break()
        {
            return ControlFlow::Break(());
        }
        for cost in &option.alternative_costs {
            let kind = match Self::alternative_cast_clause(definition, option, cost.id) {
                Some((_, ability, kind)) if ability.is_executable() => Some(kind),
                Some(_) => continue,
                None => None,
            };
            // A free cast gated on the board is not offered while its
            // condition is false, the same way an "activate only if" ability
            // is not offered.
            let gated = match Self::alternative_cast_clause(definition, option, cost.id) {
                Some((origin, ability, _)) => match ability.definition {
                    DeclarativeAbilityDef::AlternativeCast(alternative) => {
                        // CR 118.4: life can only be paid down to zero, so an
                        // alternative that costs more life than the player
                        // has is not on offer at all.
                        i16::try_from(alternative.life).unwrap_or(i16::MAX)
                            > self.players[player.index()].life
                            || alternative.condition.is_some_and(|condition| {
                                !self.trigger_condition_holds(
                                    condition,
                                    card,
                                    player,
                                    TriggerContext::empty(),
                                    Some(origin),
                                    None,
                                )
                            })
                    }
                    _ => false,
                },
                None => false,
            };
            let available = !gated && self.alternative_is_castable_from(source_zone, kind, card);
            if available
                && Self::visit_additional_cost_configurations(
                    option,
                    Some(cost.id),
                    option.additional_costs.len(),
                    &mut selected_additional,
                    &mut visitor,
                )
                .is_break()
            {
                return ControlFlow::Break(());
            }
        }
        if source_zone == CastSourceZone::Graveyard
            && self.granted_flashback(card, option).is_some()
            && let Some(granted) = Self::temporary_alternative_cost_id(option)
            && Self::visit_additional_cost_configurations(
                option,
                Some(granted),
                option.additional_costs.len(),
                &mut selected_additional,
                &mut visitor,
            )
            .is_break()
        {
            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
    }

    pub(in crate::game) fn visit_additional_cost_configurations(
        option: &PlayOptionDef,
        alternative: Option<AlternativeCostId>,
        remaining: usize,
        selected_reversed: &mut Vec<AdditionalCostId>,
        visitor: &mut impl FnMut(CostConfiguration) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        let Some(index) = remaining.checked_sub(1) else {
            let additional = selected_reversed.iter().rev().copied().collect();
            return visitor(CostConfiguration::new(alternative, additional));
        };

        if Self::visit_additional_cost_configurations(
            option,
            alternative,
            index,
            selected_reversed,
            visitor,
        )
        .is_break()
        {
            return ControlFlow::Break(());
        }
        selected_reversed.push(option.additional_costs[index].id);
        let result = Self::visit_additional_cost_configurations(
            option,
            alternative,
            index,
            selected_reversed,
            visitor,
        );
        selected_reversed.pop();
        result
    }

    pub(in crate::game) fn configured_cast_mana_cost(
        &self,
        card: GameObjectId,
        option: &PlayOptionDef,
        configuration: &CostConfiguration,
    ) -> Option<ManaCost> {
        let granted = Self::temporary_alternative_cost_id(option);
        let granted_flashback = (configuration.alternative().is_some()
            && configuration.alternative() == granted)
            .then(|| self.granted_flashback(card, option))
            .flatten();
        let mut cost = granted_flashback.map_or_else(
            || configured_mana_cost(option, configuration),
            |(_, mana_cost)| Some(mana_cost),
        )?;
        // `configured_mana_cost` already included additional costs for every
        // printed alternative and the normal cost. Runtime-granted
        // alternatives need them folded in here.
        if granted_flashback.is_some() {
            for selected in configuration.additional() {
                let additional = option
                    .additional_costs
                    .iter()
                    .find(|candidate| candidate.id == *selected)?;
                if let Some(mana) = additional.mana_cost {
                    cost = add_mana_cost(cost, mana);
                }
            }
        }
        // "Without paying its mana cost" and "rather than paying its mana
        // cost" are both permissions held over the card rather than
        // alternatives printed on it, so they are applied here, after
        // everything the card itself asks for. Additional costs still apply
        // (CR 601.2h); only the mana cost is replaced.
        if self.card_mana_cost_is_replaced(card) {
            cost = ManaCost {
                variable_x: cost.variable_x,
                x_multiplier: cost.x_multiplier,
                ..ManaCost::default()
            };
        }
        Some(cost)
    }

    /// Whether whoever is playing this card pays something other than its
    /// mana cost -- nothing at all, or energy. Read off the exile
    /// permissions, which is the only source today.
    fn card_mana_cost_is_replaced(&self, card: GameObjectId) -> bool {
        self.exile_play_permissions.iter().any(|permission| {
            permission.card == card
                && matches!(
                    permission.cost,
                    ExilePlayCost::Free | ExilePlayCost::EnergyEqualToManaValue
                )
        })
    }
}
