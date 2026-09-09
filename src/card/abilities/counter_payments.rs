// Shared counter-unless-paid programs. Included into card::abilities.

static COUNTER_PRIMARY_TARGET: EffectDef = EffectDef::Counter {
    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
    zone: ZoneKind::Graveyard,
    placement: ZonePlacement::Top,
};
static COUNTER_PRIMARY_TARGET_TO_EXILE: EffectDef = EffectDef::Counter {
    object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
    zone: ZoneKind::Exile,
    placement: ZonePlacement::Top,
};
static COUNTER_TRIGGERING_SPELL: EffectDef = EffectDef::Counter {
    object: EffectRecipientDef::TriggeringObject,
    zone: ZoneKind::Graveyard,
    placement: ZonePlacement::Top,
};

const fn pay_or_counter(
    payer: PlayerRefDef,
    costs: &'static [CostDef],
    otherwise: &'static EffectDef,
) -> EffectDef {
    EffectDef::PayOr(
        PayOrDef::unless(costs, otherwise)
            .with_payer(PlayerSetDef::One(payer))
            .with_visibility(ChoiceVisibilityDef::Public),
    )
}

/// Counter the primary targeted spell unless its controller pays the supplied costs.
#[must_use]
pub const fn counter_target_unless_paid(costs: &'static [CostDef]) -> EffectDef {
    pay_or_counter(
        PlayerRefDef::ControllerOf(ObjectRefDef::Target(TargetIndex::PRIMARY)),
        costs,
        &COUNTER_PRIMARY_TARGET,
    )
}

/// Counter the primary targeted spell into exile unless its controller pays.
#[must_use]
pub const fn counter_target_to_exile_unless_paid(costs: &'static [CostDef]) -> EffectDef {
    pay_or_counter(
        PlayerRefDef::ControllerOf(ObjectRefDef::Target(TargetIndex::PRIMARY)),
        costs,
        &COUNTER_PRIMARY_TARGET_TO_EXILE,
    )
}

/// Counter the spell that caused a trigger unless its controller pays.
#[must_use]
pub const fn counter_triggering_spell_unless_paid(costs: &'static [CostDef]) -> EffectDef {
    pay_or_counter(
        PlayerRefDef::ControllerOf(ObjectRefDef::TriggeringObject),
        costs,
        &COUNTER_TRIGGERING_SPELL,
    )
}
