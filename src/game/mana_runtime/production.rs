// Mana production and the replacement of a tapped source's output.

impl Game {
    pub(super) fn replaces_tapped_mana(
        &self,
        controller: PlayerId,
        source_types: crate::card::CardTypeSet,
        tapped: bool,
        amount: usize,
    ) -> bool {
        if !tapped || amount == 0 {
            return false;
        }
        let mut replace = false;
        self.visit_player_static_rules(controller, |rule| {
            if let crate::card::AppliedRuleDef::TappedManaBecomesColorless { types, minimum } = rule
            {
                replace |= source_types.intersects(types) && amount >= usize::from(minimum);
            }
        });
        replace
    }

    pub(super) fn replace_tapped_mana(
        &self,
        activation: &ManaAbilityActivation,
        mut produced: Vec<Mana>,
    ) -> Vec<Mana> {
        if self.replaces_tapped_mana(
            activation.controller,
            activation.source_types,
            activation.costs.contains(&CostDef::TapSource),
            produced.len(),
        ) {
            let mut mana = produced[0];
            mana.color = ManaColor::Colorless;
            produced.clear();
            produced.push(mana);
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
