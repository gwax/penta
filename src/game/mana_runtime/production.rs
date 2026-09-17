// Mana production and the replacement of a tapped source's output.

impl Game {
    /// Apply the shared replacement program to a prospective production shape.
    /// The color query, planner, and commit path all use this same transform.
    fn replaced_mana_shape(
        &self,
        source_types: crate::card::CardTypeSet,
        tapped: bool,
        mut types: Vec<ManaColor>,
        mut amount: usize,
    ) -> (Vec<ManaColor>, usize) {
        use crate::card::{ReplacementEffectDef, ReplacementEventDef};
        fn apply(effect: ReplacementEffectDef, types: &mut Vec<ManaColor>, amount: &mut usize) {
            match effect {
                ReplacementEffectDef::Sequence(effects) => {
                    for effect in effects {
                        apply(*effect, types, amount);
                    }
                }
                ReplacementEffectDef::SetManaType(color) => {
                    *types = vec![color];
                }
                ReplacementEffectDef::SetEventAmount(value) => {
                    *amount = usize::from(value);
                }
                _ => unreachable!("catalog validation checks mana replacement programs"),
            }
        }
        if tapped && amount > 0 {
            for permanent in &self.battlefield {
                self.for_each_effective_ability(permanent, |effective| {
                    let ability = effective.ability;
                    let DeclarativeAbilityDef::Replacement(definition) = ability.definition else {
                        return;
                    };
                    let ReplacementEventDef::TappedForMana {
                        source_types: required,
                        minimum_amount,
                    } = definition.event
                    else {
                        return;
                    };
                    if source_types.intersects(required)
                        && amount >= usize::from(minimum_amount)
                        && definition.source_zones == [ZoneKind::Battlefield]
                        && !definition.optional
                        && !definition.once
                        && definition.condition.is_none()
                        && let Some(effect) = ability.declarative_replacement()
                    {
                        apply(effect, &mut types, &mut amount);
                    }
                });
            }
        }
        (types, amount)
    }

    pub(super) fn replace_tapped_mana(
        &self,
        activation: &ManaAbilityActivation,
        mut produced: Vec<Mana>,
    ) -> Vec<Mana> {
        let (types, amount) = self.replaced_mana_shape(
            activation.source_types,
            activation.costs.contains(&CostDef::TapSource),
            produced.iter().map(|mana| mana.color).collect(),
            produced.len(),
        );
        if let Some(prototype) = produced.first().copied() {
            produced.resize(amount, prototype);
            produced.truncate(amount);
            for (index, mana) in produced.iter_mut().enumerate() {
                mana.color = types
                    .get(index)
                    .or_else(|| types.first())
                    .copied()
                    .unwrap_or(mana.color);
            }
        }
        produced
    }

    pub(super) fn mana_production(&self, activation: &ManaAbilityActivation) -> PaymentPool {
        let mut pool = PaymentPool::default();
        for mana in self.mana_for_activation(activation) {
            pool.add_unit(
                mana,
                mana.restrictions
                    .contains(&ManaRestrictionDef::CannotPayGeneric),
            );
        }
        // Triggered mana belongs to a separate event, even when its trigger watched this tap.
        if let Some(triggered) = &activation.triggered_mana {
            for split in triggered {
                for (color, amount) in split.iter() {
                    pool.add_color(color, amount);
                }
            }
        }
        pool
    }

    pub(super) fn mana_production_for(
        &self,
        activation: &ManaAbilityActivation,
        purpose: &ManaPaymentPurpose,
        cost: ManaCost,
    ) -> PaymentPool {
        let mut pool = self.mana_production(activation);
        pool.non_generic = ManaPool::default();
        for mana in self.mana_for_activation(activation) {
            if self.mana_requires_nongeneric(mana, purpose, cost) {
                pool.non_generic.add_color(mana.color, 1);
            }
        }
        pool
    }

    pub(super) fn mana_for_activation(&self, activation: &ManaAbilityActivation) -> Vec<Mana> {
        self.replace_tapped_mana(activation, Self::raw_mana_for_activation(activation))
    }
}
