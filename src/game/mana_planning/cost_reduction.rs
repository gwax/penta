// What a spell's cost is reduced by, and what the reductions may read.
//
// Two shapes share this file because they answer the same question from
// opposite sides: a card in hand discounting itself, and a permanent on the
// battlefield discounting other spells. Included textually into
// `mana_planning.rs`, so the imports here are the parent module's.

impl Game {
    /// How much generic mana this card's own static clauses take off its
    /// cost. Read from the hand, which is where casting reads it.
    pub(super) fn spell_cost_reduction(
        &self,
        definition: CardDefinitionId,
        player: PlayerId,
        source: GameObjectId,
    ) -> u16 {
        let Some(card) = self.catalog.get(definition) else {
            return 0;
        };
        card.rules
            .ability_clauses()
            .iter()
            .filter(|ability| ability.is_executable())
            .filter_map(|ability| match ability.declarative_effect()? {
                EffectDef::ReduceGenericCostBy(value) => Some(value),
                _ => None,
            })
            .map(|value| self.cost_reduction_value(value, player, source))
            .fold(0, u16::saturating_add)
            .saturating_add(self.battlefield_spell_cost_reduction(player, source))
    }

    /// What permanents on the battlefield take off this spell's cost.
    ///
    /// Read from the card being cast rather than from its definition, so a
    /// continuous effect that changed its types is what the predicate sees.
    fn battlefield_spell_cost_reduction(&self, player: PlayerId, source: GameObjectId) -> u16 {
        let Some((zone, card)) = self.card_in_nonbattlefield_zone(source) else {
            return 0;
        };
        let mut reduction = 0_u16;
        for permanent in &self.battlefield {
            let Some(rules) = self.effective_rules(permanent) else {
                continue;
            };
            for ability in rules.ability_clauses() {
                if !ability.is_executable() {
                    continue;
                }
                let Some(EffectDef::ReduceMatchingSpellCostBy {
                    spell,
                    caster,
                    amount,
                }) = ability.declarative_effect()
                else {
                    continue;
                };
                if !self.player_relation_matches(
                    player,
                    caster,
                    permanent.controller,
                    TriggerContext::empty(),
                ) {
                    continue;
                }
                if !self.card_object_matches(spell, card, zone, permanent.card.id) {
                    continue;
                }
                reduction =
                    reduction.saturating_add(self.cost_reduction_value(amount, player, source));
            }
        }
        reduction
    }

    /// What permanents on the battlefield add to this spell's cost. Read the
    /// same way as the discount beside it, and kept separate from it because
    /// the two are not opposite numbers: a discount is generic-only, while an
    /// increase can name a colour.
    pub(super) fn spell_cost_increase(&self, player: PlayerId, source: GameObjectId) -> ManaCost {
        let Some((zone, card)) = self.card_in_nonbattlefield_zone(source) else {
            return ManaCost::default();
        };
        let mut increase = ManaCost::default();
        for permanent in &self.battlefield {
            let Some(rules) = self.effective_rules(permanent) else {
                continue;
            };
            for ability in rules.ability_clauses() {
                if !ability.is_executable() {
                    continue;
                }
                let Some(EffectDef::IncreaseMatchingSpellCostBy {
                    spell,
                    caster,
                    amount,
                }) = ability.declarative_effect()
                else {
                    continue;
                };
                if !self.player_relation_matches(
                    player,
                    caster,
                    permanent.controller,
                    TriggerContext::empty(),
                ) {
                    continue;
                }
                if !self.card_object_matches(spell, card, zone, permanent.card.id) {
                    continue;
                }
                increase = add_mana_cost(increase, amount);
            }
        }
        increase
    }

    /// What this permanent's activated abilities actually cost in mana,
    /// with every increase on the battlefield folded in. Abilities have no
    /// discount vocabulary, so unlike a spell this only ever goes up.
    pub(super) fn ability_mana_cost(&self, permanent: &Permanent, cost: ManaCost) -> ManaCost {
        let mut total = cost;
        for other in &self.battlefield {
            let Some(rules) = self.effective_rules(other) else {
                continue;
            };
            for ability in rules.ability_clauses() {
                if !ability.is_executable() {
                    continue;
                }
                let Some(EffectDef::IncreaseMatchingAbilityCostBy {
                    permanent: matcher,
                    amount,
                }) = ability.declarative_effect()
                else {
                    continue;
                };
                if self.trigger_object_matches(
                    matcher,
                    &self.trigger_event_object(permanent),
                    other.card.id,
                    false,
                ) {
                    total = add_mana_cost(total, amount);
                }
            }
        }
        total
    }

    /// The values a cost reduction can read. There is no resolving object
    /// while a cost is being worked out, but static zone queries can still
    /// use the card being cast as their source.
    /// A mana ability's amount, read off the permanent offering it. Only
    /// board-readable values belong here: the number has to be known before
    /// the ability is activated, not while it resolves.
    pub(super) fn mana_ability_value(&self, value: ValueDef, permanent: &Permanent) -> u16 {
        match value {
            ValueDef::CountersOnSource(kind) => permanent.counters(kind),
            other => self.cost_reduction_value(other, permanent.controller, permanent.card.id),
        }
        // `cost_reduction_value` already answers constants and battlefield
        // counts; anything it does not know reads as zero, which is why the
        // boundary rule admits only the forms listed there.
    }

    pub(super) fn cost_reduction_value(
        &self,
        value: ValueDef,
        player: PlayerId,
        source: GameObjectId,
    ) -> u16 {
        match value {
            ValueDef::Constant(amount) => u16::try_from(amount.max(0)).unwrap_or(u16::MAX),
            ValueDef::CountMatchingObjects(query) => u16::try_from(
                self.objects_matching_query(*query, player, source, TriggerContext::empty())
                    .len(),
            )
            .unwrap_or(u16::MAX),
            _ => 0,
        }
    }
}
