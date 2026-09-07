use super::super::{EffectResolutionContext, Game, ScopedEffect, StackObject};
use crate::card::{ChoiceVisibilityDef, CostDef, EffectDef, EffectRecipientDef};

impl Game {
    pub(super) fn resolve_cumulative_upkeep(
        &mut self,
        cost: CostDef,
        scoped: ScopedEffect,
        object: &StackObject,
        context: EffectResolutionContext,
    ) {
        let Some(source) = object.source else {
            return;
        };
        if !self
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == source)
        {
            return;
        }
        let age_kind = crate::CounterKind::named("age");
        self.add_counters_to_permanent(source, age_kind, 1);
        let age = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == source)
            .map_or(0, |permanent| permanent.counters(age_kind));
        if matches!(cost, CostDef::Sacrifice { .. } | CostDef::Discard { .. }) {
            self.queue_cost_payment_window(crate::game::cost_payment::CostPaymentWindow {
                player: object.controller,
                definition: scoped,
                object: Box::new(object.clone()),
                context,
                path: Vec::new(),
                chosen: Vec::new(),
                cumulative_upkeep_age: Some(age),
            });
            return;
        }
        let payment = Self::resolved_cumulative_upkeep_payment(cost, source, age);
        self.queue_pay_or(
            object.controller,
            payment,
            Some(age),
            ChoiceVisibilityDef::Private,
            scoped,
            object,
            context,
            None,
            Some(scoped.with_effect(EffectDef::Sacrifice {
                object: EffectRecipientDef::Source,
            })),
        );
    }
}
