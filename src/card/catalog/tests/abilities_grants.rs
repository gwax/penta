use super::*;
use crate::card::{EffectPaymentDef, PayOrDef, abilities};

#[test]
fn catalog_rejects_effect_operations_in_the_wrong_execution_context() {
    static STATIC_PUMP: EffectDef = EffectDef::StaticApply {
        recipient: EffectRecipientDef::Source,
        effect: AppliedEffectDef::modify_power_toughness(
            ValueDef::Constant(1),
            ValueDef::Constant(1),
        ),
    };
    static RESOLVING_STATIC: [EffectDef; 1] = [STATIC_PUMP];
    static ATTACK_QUERY: ObjectQueryDef =
        ObjectQueryDef::new(ObjectPredicateDef::Any, &[ZoneKind::Battlefield]);

    let cases = [
        (
            AbilityDef::static_ability(
                "At static-effect time, draw a card.",
                EffectDef::DrawCards {
                    recipient: EffectRecipientDef::Controller,
                    amount: ValueDef::Constant(1),
                },
            ),
            "static",
            "DrawCards",
        ),
        (
            AbilityDef::static_ability(
                "At static-effect time, store a resolved pump.",
                EffectDef::Apply {
                    recipient: EffectRecipientDef::Source,
                    effect: AppliedEffectDef::modify_power_toughness(
                        ValueDef::Constant(1),
                        ValueDef::Constant(1),
                    ),
                    duration: ResolvedEffectDurationDef::UntilEndOfTurn,
                },
            ),
            "static",
            "Apply",
        ),
        (
            AbilityDef::spell(
                "Resolve a live static effect.",
                EffectDef::Sequence(&RESOLVING_STATIC),
            ),
            "resolving",
            "StaticApply",
        ),
        (
            AbilityDef::activated(
                "Use a declaration-only attack restriction.",
                &[],
                EffectDef::CannotAttackUnless(&ATTACK_QUERY),
            ),
            "resolving",
            "CannotAttackUnless",
        ),
    ];

    for (ability, context, operation) in cases {
        assert_eq!(
            error(definition_with_ability(ability)),
            CatalogError::UnsupportedAbilityEffectProgramContext {
                definition: CardDefinitionId(1),
                part: CardPartId::PRIMARY,
                ability: AbilityId::PRIMARY,
                context,
                operation,
            },
        );
    }
}

#[test]
fn catalog_accepts_each_supported_static_program_lane() {
    static GRAVEYARD_CREATURES: ObjectQueryDef = ObjectQueryDef::matching(
        ObjectPredicateDef::HasType(CardType::Creature),
        &[ZoneKind::Graveyard],
        PlayerRelation::You,
    );
    let abilities = [
        AbilityDef::static_ability(
            "This creature gets +1/+1.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(1),
                    ValueDef::Constant(1),
                ),
            },
        ),
        AbilityDef::static_ability(
            "Players can't cast noncreature spells.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::EachPlayer,
                effect: AppliedEffectDef::Rule(AppliedRuleDef::CannotPlay(
                    PlayRestrictionDef::new(
                        PlayActionMatcherDef::CastSpell,
                        ObjectPredicateDef::NoncreatureSpell,
                    ),
                )),
            },
        ),
        AbilityDef::static_ability(
            "This spell can't be countered.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::Rule(AppliedRuleDef::CannotBeCountered),
            },
        )
        .with_source_zones(&[ZoneKind::Stack]),
        AbilityDef::static_ability(
            "This spell costs {1} less for each creature card in your graveyard.",
            EffectDef::ReduceGenericCostBy(ValueDef::CountMatchingObjects(&GRAVEYARD_CREATURES)),
        )
        .with_source_zones(&[ZoneKind::Hand]),
        AbilityDef::enforced_when_cast(
            "This spell has an externally enforced casting restriction.",
            "The casting action generator enforces this clause.",
        ),
    ];

    for ability in abilities {
        CardCatalog::new([definition_with_ability(ability)])
            .expect("the live runtime consumes this static program lane");
    }
}

#[test]
fn static_apply_rejects_shapes_its_live_reader_would_ignore() {
    let cases = [
        (
            EffectRecipientDef::EachPlayer,
            AppliedEffectDef::Rule(AppliedRuleDef::CannotBlock),
            "StaticApply with an unsupported player-facing effect",
        ),
        (
            EffectRecipientDef::Source,
            AppliedEffectDef::Rule(AppliedRuleDef::CannotPlay(PlayRestrictionDef::new(
                PlayActionMatcherDef::CastSpell,
                ObjectPredicateDef::Any,
            ))),
            "StaticApply with an unsupported object-facing effect",
        ),
        (
            EffectRecipientDef::Source,
            AppliedEffectDef::Composite(&[]),
            "StaticApply with an unsupported object-facing effect",
        ),
        (
            EffectRecipientDef::Source,
            AppliedEffectDef::Rule(AppliedRuleDef::PreventDamage(DamageEventMatcherDef {
                recipient: DamageRecipientMatcherDef::PlayerAndCreaturesControlledBy(
                    PlayerRefDef::EffectController,
                ),
                ..DamageEventMatcherDef::ANY
            })),
            "StaticApply with an unsupported object-facing effect",
        ),
        (
            EffectRecipientDef::Source,
            AppliedEffectDef::Rule(AppliedRuleDef::PreventDamage(DamageEventMatcherDef {
                source: DamageSourceMatcherDef::Object(ObjectRefDef::ResolvingObject),
                ..DamageEventMatcherDef::ANY
            })),
            "StaticApply with an unsupported object-facing effect",
        ),
    ];

    for (recipient, effect, operation) in cases {
        let ability = AbilityDef::static_ability(
            "Apply a live static effect.",
            EffectDef::StaticApply { recipient, effect },
        );
        assert_eq!(
            error(definition_with_ability(ability)),
            CatalogError::UnsupportedAbilityEffectProgramContext {
                definition: CardDefinitionId(1),
                part: CardPartId::PRIMARY,
                ability: AbilityId::PRIMARY,
                context: "static",
                operation,
            },
        );
    }
}

#[test]
fn resolving_apply_rejects_shapes_that_cannot_be_stored() {
    for effect in [
        AppliedEffectDef::Composite(&[]),
        AppliedEffectDef::Rule(AppliedRuleDef::CannotBeCountered),
        AppliedEffectDef::Rule(AppliedRuleDef::PreventDamage(DamageEventMatcherDef::ANY)),
    ] {
        assert_eq!(
            validate_ability_targets(
                &[],
                EffectDef::Apply {
                    recipient: EffectRecipientDef::Source,
                    effect,
                    duration: ResolvedEffectDurationDef::UntilEndOfTurn,
                },
            ),
            Err(GrantedAbilityValidationError::UnsupportedResolvingAppliedEffect),
        );
    }
    assert_eq!(
        validate_ability_targets(
            &[],
            EffectDef::Apply {
                recipient: EffectRecipientDef::Controller,
                effect: AppliedEffectDef::Rule(AppliedRuleDef::CannotBlock),
                duration: ResolvedEffectDurationDef::UntilEndOfTurn,
            },
        ),
        Err(GrantedAbilityValidationError::UnsupportedResolvingAppliedEffect),
    );
}

#[test]
fn nonbattlefield_ability_grants_are_executable_flashback_until_cleanup() {
    static FLYING: AbilityDef = abilities::flying();
    static FLASHBACK: AbilityDef = abilities::flashback_for_card_mana_cost();
    static MIRACLE: AbilityDef = abilities::miracle(ManaCost::new(0, 0));
    static INCOMPLETE_FLASHBACK: AbilityDef = abilities::flashback_for_card_mana_cost()
        .with_coverage(AbilityCoverageDef::metadata_only(
            "This fixture verifies that non-executable grants are rejected.",
        ));
    static GRAVEYARD_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
        AbilityTargetPredicate::Object {
            object: ObjectPredicateDef::Any,
            zones: &[ZoneKind::Graveyard],
            controller: None,
            owner: None,
        },
    )];
    static UNSUPPORTED_ZONE_TARGETS: [AbilityTargetDef; 2] = [
        AbilityTargetDef::exactly_one(AbilityTargetPredicate::Object {
            object: ObjectPredicateDef::Any,
            zones: &[ZoneKind::Hand],
            controller: None,
            owner: None,
        }),
        AbilityTargetDef::exactly_one(AbilityTargetPredicate::Object {
            object: ObjectPredicateDef::Any,
            zones: &[ZoneKind::Stack],
            controller: None,
            owner: None,
        }),
    ];
    static GRAVEYARD_CARDS: ObjectQueryDef =
        ObjectQueryDef::new(ObjectPredicateDef::Any, &[ZoneKind::Graveyard]);

    let targeted_grant = |ability, duration| EffectDef::Apply {
        recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
        effect: AppliedEffectDef::add_ability(ability),
        duration,
    };

    validate_ability_targets(
        &GRAVEYARD_TARGET,
        targeted_grant(&FLASHBACK, ResolvedEffectDurationDef::UntilEndOfTurn),
    )
    .expect("the hidden-zone runtime reads executable flashback grants until cleanup");
    validate_ability_targets(
        &[],
        EffectDef::Apply {
            recipient: EffectRecipientDef::objects(ObjectSetDef::Query(GRAVEYARD_CARDS)),
            effect: AppliedEffectDef::add_ability(&FLASHBACK),
            duration: ResolvedEffectDurationDef::UntilEndOfTurn,
        },
    )
    .expect("mass flashback grants use the same supported hidden-zone reader");

    for ability in [&FLYING, &MIRACLE, &INCOMPLETE_FLASHBACK] {
        assert_eq!(
            validate_ability_targets(
                &GRAVEYARD_TARGET,
                targeted_grant(ability, ResolvedEffectDurationDef::UntilEndOfTurn),
            ),
            Err(GrantedAbilityValidationError::UnsupportedResolvingAppliedEffect),
            "a hidden-zone grant must be an executable Flashback ability",
        );
    }
    for target in UNSUPPORTED_ZONE_TARGETS {
        assert_eq!(
            validate_ability_targets(
                &[target],
                targeted_grant(&FLASHBACK, ResolvedEffectDurationDef::UntilEndOfTurn),
            ),
            Err(GrantedAbilityValidationError::UnsupportedResolvingAppliedEffect),
            "the temporary Flashback reader only consumes graveyard-card grants",
        );
    }
    assert_eq!(
        validate_ability_targets(
            &GRAVEYARD_TARGET,
            targeted_grant(&FLASHBACK, ResolvedEffectDurationDef::Permanent),
        ),
        Err(GrantedAbilityValidationError::UnsupportedResolvingAppliedEffect),
        "the runtime only stores nonbattlefield card grants until cleanup",
    );

    for duration in [
        ResolvedEffectDurationDef::UntilEndOfTurn,
        ResolvedEffectDurationDef::Permanent,
    ] {
        let spell = AbilityDef::spell(
            "This spell grants itself an ability.",
            EffectDef::Apply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::add_ability(&FLYING),
                duration,
            },
        );
        assert_eq!(
            error(definition_with_ability(spell)),
            CatalogError::UnsupportedAbilityEffectProgramContext {
                definition: CardDefinitionId(1),
                part: CardPartId::PRIMARY,
                ability: AbilityId::PRIMARY,
                context: "resolving",
                operation: "Apply grants an ability to a nonbattlefield source",
            },
        );
    }
}

#[test]
fn triggering_object_grants_use_the_declared_event_zone() {
    static HASTE: AbilityDef = abilities::haste();

    let grant = |event| {
        AbilityDef::triggered(
            "The triggering object gains haste until end of turn.",
            event,
            EffectDef::Apply {
                recipient: EffectRecipientDef::TriggeringObject,
                effect: AppliedEffectDef::add_ability(&HASTE),
                duration: ResolvedEffectDurationDef::UntilEndOfTurn,
            },
        )
    };

    CardCatalog::new([definition_with_ability(grant(
        TriggerEventDef::zone_changed(ObjectPredicateDef::Any, None, Some(ZoneKind::Battlefield)),
    ))])
    .expect("an ETB trigger provably names a battlefield object");

    assert_eq!(
        error(definition_with_ability(grant(
            TriggerEventDef::zone_changed(
                ObjectPredicateDef::Any,
                Some(ZoneKind::Battlefield),
                Some(ZoneKind::Graveyard),
            )
        ))),
        CatalogError::UnsupportedResolvingAppliedEffect {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
        },
        "a departure trigger names a nonbattlefield card and cannot grant haste",
    );
}

#[test]
fn payment_target_sets_must_resolve_to_one_player() {
    static PLAYER_TARGETS: [AbilityTargetDef; 1] = [AbilityTargetDef::up_to(
        AbilityTargetPredicate::Player(PlayerRelation::Any),
        2,
    )];
    static NONE: EffectDef = EffectDef::None;

    assert_eq!(
        validate_ability_targets(
            &PLAYER_TARGETS,
            EffectDef::PayOr(PayOrDef::optional(
                EffectPaymentDef::mana(
                    PlayerSetDef::LegalTargets(TargetIndex::PRIMARY),
                    ManaCost::new(1, 0),
                ),
                &NONE,
            )),
        ),
        Err(
            GrantedAbilityValidationError::TargetReferenceRequiresSingular {
                target: TargetIndex::PRIMARY,
                maximum: 2,
            },
        ),
    );
}

#[test]
fn ability_ids_follow_clause_order_within_each_card_part() {
    static ABILITIES: [AbilityDef; 2] = [
        AbilityDef::spell("first", EffectDef::None),
        AbilityDef::not_implemented("second", "Only positional identity matters here."),
    ];
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_abilities(&ABILITIES);
    set_primary_rules(&mut card, &rules);

    let attached = card.parts[0].rules.indexed_abilities().collect::<Vec<_>>();
    assert_eq!(attached[0].id, AbilityId(0));
    assert_eq!(attached[1].id, AbilityId(1));
    CardCatalog::new(vec![card]).expect("ordered clauses receive distinct positional IDs");
}

#[test]
fn one_card_part_cannot_define_multiple_spell_abilities() {
    static ABILITIES: [AbilityDef; 2] = [
        AbilityDef::spell("first", EffectDef::None),
        AbilityDef::spell("second", EffectDef::None),
    ];
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_abilities(&ABILITIES);
    set_primary_rules(&mut card, &rules);

    assert_eq!(
        error(card),
        CatalogError::MultipleSpellAbilities {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            count: 2,
        }
    );
}

#[test]
fn positional_ability_ids_reject_more_than_their_address_space() {
    let abilities = Box::leak(
        vec![AbilityDef::spell("A spell ability.", EffectDef::None); 257].into_boxed_slice(),
    );
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_abilities(abilities);
    set_primary_rules(&mut card, &rules);

    assert_eq!(
        error(card),
        CatalogError::TooManyAbilities {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            count: 257,
        }
    );
}

#[test]
fn grant_ids_reject_more_than_their_structural_address_space() {
    static GRANTED: AbilityDef = AbilityDef::not_implemented(
        "A granted ability.",
        "The test only needs a reusable definition.",
    );
    let effects = Box::leak(
        vec![
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::add_ability(&GRANTED),
            };
            257
        ]
        .into_boxed_slice(),
    );
    let abilities = Box::leak(
        vec![AbilityDef::static_ability(
            "This object receives many abilities.",
            EffectDef::Sequence(effects),
        )]
        .into_boxed_slice(),
    );
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_abilities(abilities);
    set_primary_rules(&mut card, &rules);

    assert_eq!(
        error(card),
        CatalogError::TooManyAbilityGrantSites {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            count: 257,
        }
    );
}

#[test]
fn delayed_grants_count_toward_the_structural_address_space() {
    static GRANTED: AbilityDef = AbilityDef::not_implemented(
        "A granted ability.",
        "The test only needs a reusable definition.",
    );
    static GRANT: EffectDef = EffectDef::StaticApply {
        recipient: EffectRecipientDef::Source,
        effect: AppliedEffectDef::add_ability(&GRANTED),
    };
    static DELAYED_GRANT: EffectDef =
        EffectDef::InstallTrigger(InstalledTriggerDef::once(&AbilityDef::triggered(
            "At the beginning of your next end step, grant an ability.",
            TriggerEventDef::StepBegins {
                step: TurnStepDef::End,
                player: PlayerRelation::You,
            },
            GRANT,
        )));
    let effects = Box::leak(vec![DELAYED_GRANT; 257].into_boxed_slice());
    let abilities = Box::leak(
        vec![AbilityDef::static_ability(
            "This object schedules many granted abilities.",
            EffectDef::Sequence(effects),
        )]
        .into_boxed_slice(),
    );
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_abilities(abilities);
    set_primary_rules(&mut card, &rules);

    assert_eq!(
        error(card),
        CatalogError::TooManyAbilityGrantSites {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            count: 257,
        }
    );
}

#[test]
fn replacement_program_grants_count_toward_the_structural_address_space() {
    static GRANTED: AbilityDef = AbilityDef::not_implemented(
        "A granted ability.",
        "The test only needs a reusable definition.",
    );
    static GRANT: EffectDef = EffectDef::StaticApply {
        recipient: EffectRecipientDef::Source,
        effect: AppliedEffectDef::add_ability(&GRANTED),
    };
    let replacement_effects =
        Box::leak(vec![ReplacementEffectDef::Perform(&GRANT); 257].into_boxed_slice());
    let abilities = Box::leak(
        vec![
            AbilityDef::replacement(
                "This replacement performs many ability grants.",
                ReplacementEffectDef::Sequence(replacement_effects),
            )
            .with_effect_execution(EffectExecutionDef::Custom(CardBehavior::Unsupported))
            .with_coverage(AbilityCoverageDef::explained_complete(
                "This structural-capacity test does not execute the replacement program.",
            )),
        ]
        .into_boxed_slice(),
    );
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_abilities(abilities);
    set_primary_rules(&mut card, &rules);

    assert_eq!(
        error(card),
        CatalogError::TooManyAbilityGrantSites {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            count: 257,
        }
    );
}

#[test]
fn executable_granted_static_abilities_are_rejected_until_fixed_point_evaluation_exists() {
    static GRANTED: AbilityDef =
        AbilityDef::static_ability("This object gets +1/+1.", EffectDef::None);

    assert_eq!(
        error(definition_granting(&GRANTED)),
        CatalogError::InvalidGrantedAbility {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            grant_path: vec![GrantId::PRIMARY],
            problem: GrantedAbilityValidationError::ExecutableStaticAbility,
        }
    );
}

#[test]
fn granted_ability_validation_reports_nested_structural_paths() {
    static INVALID: AbilityDef = AbilityDef::spell("", EffectDef::None);
    static CHILD: AbilityDef = AbilityDef::activated(
        "This ability grants another ability.",
        &[],
        EffectDef::Apply {
            recipient: EffectRecipientDef::Source,
            effect: AppliedEffectDef::add_ability(&INVALID),
            duration: ResolvedEffectDurationDef::UntilEndOfTurn,
        },
    );

    assert_eq!(
        error(definition_granting(&CHILD)),
        CatalogError::InvalidGrantedAbility {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            grant_path: vec![GrantId::PRIMARY, GrantId::PRIMARY],
            problem: GrantedAbilityValidationError::EmptyText,
        }
    );
}

#[test]
fn granted_ability_validation_follows_sacrifice_continuations() {
    static INVALID: AbilityDef = AbilityDef::spell("", EffectDef::None);
    static THEN: EffectDef = EffectDef::Apply {
        recipient: EffectRecipientDef::Source,
        effect: AppliedEffectDef::add_ability(&INVALID),
        duration: ResolvedEffectDurationDef::UntilEndOfTurn,
    };
    static CHILD: AbilityDef = AbilityDef::activated(
        "Sacrifice a permanent, then grant an ability.",
        &[],
        EffectDef::SacrificeOfChoice {
            player: EffectRecipientDef::Controller,
            object: ObjectPredicateDef::Any,
            then: Some(&THEN),
            optional: false,
        },
    );

    assert_eq!(
        error(definition_granting(&CHILD)),
        CatalogError::InvalidGrantedAbility {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            grant_path: vec![GrantId::PRIMARY, GrantId::PRIMARY],
            problem: GrantedAbilityValidationError::EmptyText,
        }
    );
}

#[test]
fn granted_ability_validation_follows_replacement_programs() {
    static INVALID: AbilityDef = AbilityDef::spell("", EffectDef::None);
    static GRANT: EffectDef = EffectDef::Apply {
        recipient: EffectRecipientDef::Source,
        effect: AppliedEffectDef::add_ability(&INVALID),
        duration: ResolvedEffectDurationDef::UntilEndOfTurn,
    };
    static PROGRAM: [ReplacementEffectDef; 2] = [
        ReplacementEffectDef::MoveToZone(ZoneKind::Exile),
        ReplacementEffectDef::Perform(&GRANT),
    ];
    static ABILITIES: [AbilityDef; 1] = [AbilityDef::replacement(
        "Replace an event, then grant an ability.",
        ReplacementEffectDef::Sequence(&PROGRAM),
    )
    .with_effect_execution(EffectExecutionDef::Custom(CardBehavior::Unsupported))
    .with_coverage(AbilityCoverageDef::explained_complete(
        "This structural grant-validation test does not execute the replacement program.",
    ))];
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_abilities(&ABILITIES);
    set_primary_rules(&mut card, &rules);

    assert_eq!(
        error(card),
        CatalogError::InvalidGrantedAbility {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            grant_path: vec![GrantId::PRIMARY],
            problem: GrantedAbilityValidationError::EmptyText,
        }
    );
}

#[test]
fn granted_modal_branches_validate_nested_grants_in_printed_order() {
    static VALID: AbilityDef = AbilityDef::not_implemented(
        "A valid granted ability.",
        "Only nested validation matters in this fixture.",
    );
    static INVALID: AbilityDef = AbilityDef::spell("", EffectDef::None);
    static MODES: [AbilityDef; 2] = [
        AbilityDef::spell(
            "The first mode grants a valid ability.",
            EffectDef::Apply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::add_ability(&VALID),
                duration: ResolvedEffectDurationDef::UntilEndOfTurn,
            },
        ),
        AbilityDef::spell(
            "The second mode grants an invalid ability.",
            EffectDef::Apply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::add_ability(&INVALID),
                duration: ResolvedEffectDurationDef::UntilEndOfTurn,
            },
        ),
    ];
    static GRANTED_MODAL: AbilityDef = AbilityDef::choose_one_spell("Choose one.", &MODES);

    assert_eq!(
        error(definition_granting(&GRANTED_MODAL)),
        CatalogError::InvalidGrantedAbility {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            grant_path: vec![GrantId::PRIMARY, GrantId(1)],
            problem: GrantedAbilityValidationError::EmptyText,
        }
    );
}

#[test]
fn granted_modal_capacity_counts_grants_across_all_modes() {
    static TERMINAL: AbilityDef = AbilityDef::not_implemented(
        "A terminal granted ability.",
        "The terminal ability is intentionally not executable.",
    );
    let grants = |count| {
        Box::leak(
            vec![
                EffectDef::Apply {
                    recipient: EffectRecipientDef::Source,
                    effect: AppliedEffectDef::add_ability(&TERMINAL),
                    duration: ResolvedEffectDurationDef::UntilEndOfTurn,
                };
                count
            ]
            .into_boxed_slice(),
        )
    };
    let modes = Box::leak(
        vec![
            AbilityDef::spell("First mode.", EffectDef::Sequence(grants(128))),
            AbilityDef::spell("Second mode.", EffectDef::Sequence(grants(129))),
        ]
        .into_boxed_slice(),
    );
    let granted_modal = Box::leak(Box::new(AbilityDef::choose_one_spell("Choose one.", modes)));

    assert_eq!(
        error(definition_granting(granted_modal)),
        CatalogError::InvalidGrantedAbility {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            grant_path: vec![GrantId::PRIMARY],
            problem: GrantedAbilityValidationError::TooManyGrantSites { count: 257 },
        }
    );
}

#[test]
fn granted_ability_validation_checks_zones_mana_targets_and_target_slots() {
    static MANA_TARGETS: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
        AbilityTargetPredicate::Player(PlayerRelation::Any),
    )];
    static NO_ZONES: AbilityDef =
        AbilityDef::activated("An activated ability.", &[], EffectDef::None).with_source_zones(&[]);
    static TARGETED_MANA: AbilityDef = AbilityDef::defined(
        "A targeted mana ability.",
        DeclarativeAbilityDef::ActivatedMana(
            ActivatedAbilityDef::new(&[AbilityCostDef::TapSource]).with_targets(&MANA_TARGETS),
        ),
        EffectDef::None,
    );
    static OUT_OF_RANGE_TARGET: AbilityDef = AbilityDef::activated_with_targets(
        "An activated ability.",
        &[],
        &MANA_TARGETS,
        EffectDef::DealDamage {
            recipient: EffectRecipientDef::Target(TargetIndex(1)),
            amount: crate::ValueDef::Constant(1),
        },
    );

    let cases = [
        (&NO_ZONES, GrantedAbilityValidationError::HasNoSourceZone),
        (
            &TARGETED_MANA,
            GrantedAbilityValidationError::ManaAbilityHasTargets,
        ),
        (
            &OUT_OF_RANGE_TARGET,
            GrantedAbilityValidationError::TargetReferenceOutOfBounds {
                target: TargetIndex(1),
                target_count: 1,
            },
        ),
    ];
    for (granted, problem) in cases {
        assert_eq!(
            error(definition_granting(granted)),
            CatalogError::InvalidGrantedAbility {
                definition: CardDefinitionId(1),
                part: CardPartId::PRIMARY,
                ability: AbilityId::PRIMARY,
                grant_path: vec![GrantId::PRIMARY],
                problem,
            }
        );
    }
}

#[test]
fn target_references_are_validated_through_nested_values() {
    static TARGETS: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
        AbilityTargetPredicate::Player(PlayerRelation::Any),
    )];
    static CONDITION: TargetConditionDef = TargetConditionDef {
        slot: TargetIndex(1),
        object: crate::ObjectPredicateDef::Any,
        then: ValueDef::Constant(1),
        otherwise: ValueDef::Constant(0),
    };
    static ABILITIES: [AbilityDef; 1] = [AbilityDef::spell_with_targets(
        "Use a nested value from the chosen target.",
        &TARGETS,
        EffectDef::GainLife {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::IfTargetMatches(&CONDITION),
        },
    )];
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_abilities(&ABILITIES);
    set_primary_rules(&mut card, &rules);

    assert_eq!(
        error(card),
        CatalogError::AbilityTargetReferenceOutOfBounds {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            target: TargetIndex(1),
            target_count: 1,
        }
    );
}

#[test]
fn non_targeting_choice_references_are_lexically_scoped() {
    let binding = ObjectBindingIndex::PRIMARY;
    let chosen = EffectRecipientDef::object(ObjectRefDef::Binding(binding));
    let destroy_chosen: &'static EffectDef = Box::leak(Box::new(EffectDef::Destroy {
        object: chosen,
        can_regenerate: true,
    }));

    assert_eq!(
        super::validate_ability_targets(&[], *destroy_chosen,),
        Err(GrantedAbilityValidationError::ObjectBindingReferenceOutOfScope { binding }),
    );

    let rebound: &'static EffectDef = Box::leak(Box::new(EffectDef::Choose(ChooseDef {
        binding: ObjectChoiceBindingDef::Object(binding),
        chooser: PlayerRefDef::EffectController,
        candidates: ObjectSetDef::Query(ObjectQueryDef::new(
            ObjectPredicateDef::Any,
            &[ZoneKind::Battlefield],
        )),
        exclude: None,
        minimum: 1,
        maximum: 1,
        visibility: ChoiceVisibilityDef::Public,
        then: destroy_chosen,
    })));
    let nested_rebinding = EffectDef::Choose(ChooseDef {
        binding: ObjectChoiceBindingDef::Object(binding),
        chooser: PlayerRefDef::EffectController,
        candidates: ObjectSetDef::Query(ObjectQueryDef::new(
            ObjectPredicateDef::Any,
            &[ZoneKind::Battlefield],
        )),
        exclude: None,
        minimum: 1,
        maximum: 1,
        visibility: ChoiceVisibilityDef::Public,
        then: rebound,
    });
    assert_eq!(
        super::validate_ability_targets(&[], nested_rebinding),
        Err(GrantedAbilityValidationError::ObjectBindingAlreadyInScope { binding }),
    );

    super::validate_ability_targets(
        &[],
        EffectDef::Choose(ChooseDef {
            binding: ObjectChoiceBindingDef::Object(binding),
            chooser: PlayerRefDef::EffectController,
            candidates: ObjectSetDef::Query(ObjectQueryDef::new(
                ObjectPredicateDef::Any,
                &[ZoneKind::Battlefield],
            )),
            exclude: None,
            minimum: 1,
            maximum: 1,
            visibility: ChoiceVisibilityDef::Public,
            then: destroy_chosen,
        }),
    )
    .expect("the binding is visible only inside its continuation");

    static CHOSEN_CONTROLLER_QUERY: ObjectQueryDef = ObjectQueryDef::controlled_by(
        ObjectPredicateDef::HasType(CardType::Creature),
        &[ZoneKind::Battlefield],
        PlayerSetDef::One(PlayerRefDef::ControllerOf(ObjectRefDef::Binding(
            ObjectBindingIndex::PRIMARY,
        ))),
    );
    static COUNT_CHOSEN_CONTROLLERS_CREATURES: EffectDef = EffectDef::GainLife {
        recipient: EffectRecipientDef::Controller,
        amount: ValueDef::CountMatchingObjects(&CHOSEN_CONTROLLER_QUERY),
    };

    assert_eq!(
        super::validate_ability_targets(&[], COUNT_CHOSEN_CONTROLLERS_CREATURES),
        Err(GrantedAbilityValidationError::ObjectBindingReferenceOutOfScope { binding }),
        "queries embedded in values participate in lexical binding validation",
    );
    super::validate_ability_targets(
        &[],
        EffectDef::Choose(ChooseDef {
            binding: ObjectChoiceBindingDef::Object(binding),
            chooser: PlayerRefDef::EffectController,
            candidates: ObjectSetDef::Query(ObjectQueryDef::new(
                ObjectPredicateDef::Any,
                &[ZoneKind::Battlefield],
            )),
            exclude: None,
            minimum: 1,
            maximum: 1,
            visibility: ChoiceVisibilityDef::Public,
            then: &COUNT_CHOSEN_CONTROLLERS_CREATURES,
        }),
    )
    .expect("a value query can consume a choice inside its continuation");

    let set_binding = ObjectSetBindingIndex::PRIMARY;
    let sacrifice_chosen: &'static EffectDef = Box::leak(Box::new(EffectDef::Sacrifice {
        object: EffectRecipientDef::objects(ObjectSetDef::Binding(set_binding)),
    }));
    assert_eq!(
        super::validate_ability_targets(&[], *sacrifice_chosen),
        Err(
            GrantedAbilityValidationError::ObjectSetBindingReferenceOutOfScope {
                binding: set_binding,
            }
        ),
    );

    let choose_set = |then: &'static EffectDef| {
        EffectDef::Choose(ChooseDef {
            binding: ObjectChoiceBindingDef::Objects(set_binding),
            chooser: PlayerRefDef::EffectController,
            candidates: ObjectSetDef::Query(ObjectQueryDef::new(
                ObjectPredicateDef::Any,
                &[ZoneKind::Battlefield],
            )),
            exclude: None,
            minimum: 0,
            maximum: 2,
            visibility: ChoiceVisibilityDef::Public,
            then,
        })
    };
    let rebound_set: &'static EffectDef = Box::leak(Box::new(choose_set(sacrifice_chosen)));
    assert_eq!(
        super::validate_ability_targets(&[], choose_set(rebound_set)),
        Err(
            GrantedAbilityValidationError::ObjectSetBindingAlreadyInScope {
                binding: set_binding,
            }
        ),
    );
    super::validate_ability_targets(&[], choose_set(sacrifice_chosen))
        .expect("the object-set binding is visible only inside its continuation");
}

#[test]
fn generic_object_choices_validate_their_cardinality() {
    let cases = [
        (
            ObjectChoiceBindingDef::Objects(ObjectSetBindingIndex::PRIMARY),
            2,
            1,
        ),
        (
            ObjectChoiceBindingDef::Object(ObjectBindingIndex::PRIMARY),
            0,
            2,
        ),
    ];

    for (binding, minimum, maximum) in cases {
        let effect = EffectDef::Choose(ChooseDef {
            binding,
            chooser: PlayerRefDef::EffectController,
            candidates: ObjectSetDef::Query(ObjectQueryDef::new(
                ObjectPredicateDef::Any,
                &[ZoneKind::Battlefield],
            )),
            exclude: None,
            minimum,
            maximum,
            visibility: ChoiceVisibilityDef::Public,
            then: &EffectDef::None,
        });
        assert_eq!(
            super::validate_ability_targets(&[], effect),
            Err(GrantedAbilityValidationError::InvalidObjectChoiceBounds {
                binding,
                minimum,
                maximum,
            }),
        );
    }
}

#[test]
fn target_references_are_validated_through_replacement_programs() {
    static TARGETS: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
        AbilityTargetPredicate::Player(PlayerRelation::Any),
    )];
    static TARGET_EFFECT: EffectDef = EffectDef::Untap {
        object: EffectRecipientDef::Target(TargetIndex(1)),
    };
    static PROGRAM: [ReplacementEffectDef; 1] = [ReplacementEffectDef::Perform(&TARGET_EFFECT)];

    assert_eq!(
        super::validate_replacement_ability_targets(
            &TARGETS,
            ReplacementEffectDef::Sequence(&PROGRAM),
        ),
        Err(GrantedAbilityValidationError::TargetReferenceOutOfBounds {
            target: TargetIndex(1),
            target_count: 1,
        })
    );
}

#[test]
fn ability_and_program_kinds_must_agree() {
    let mut replacement = AbilityDef::replacement(
        "This permanent enters tapped.",
        ReplacementEffectDef::ModifyBattlefieldEntry(BattlefieldEntryModificationDef::Tapped),
    );
    replacement.effect = AbilityEffectDef::declarative(EffectDef::None);
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_ability(replacement);
    set_primary_rules(&mut card, &rules);
    assert_eq!(
        error(card),
        CatalogError::ReplacementAbilityRequiresReplacementProgram {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
        },
    );

    let mut spell = AbilityDef::spell("Do nothing.", EffectDef::None);
    spell.effect =
        AbilityEffectDef::replacement_program(ReplacementEffectDef::ReplaceEventWithNothing);
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_ability(spell);
    set_primary_rules(&mut card, &rules);
    assert_eq!(
        error(card),
        CatalogError::ReplacementProgramRequiresReplacementAbility {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
        },
    );
}

#[test]
fn replacement_events_reject_programs_their_runtime_would_ignore() {
    static NO_EFFECT: EffectDef = EffectDef::None;
    static NESTED_INVALID: [ReplacementEffectDef; 1] =
        [ReplacementEffectDef::MultiplyEventAmount(2)];
    static INVALID_SEQUENCE: [ReplacementEffectDef; 1] =
        [ReplacementEffectDef::Sequence(&NESTED_INVALID)];

    let cases = [
        (
            ReplacementEventDef::SourceEntersBattlefield,
            ReplacementEffectDef::ReplaceEventWithNothing,
            "ReplaceEventWithNothing",
        ),
        (
            ReplacementEventDef::SourceEntersBattlefield,
            ReplacementEffectDef::Sequence(&INVALID_SEQUENCE),
            "MultiplyEventAmount",
        ),
        (
            ReplacementEventDef::WouldGainLife(PlayerRelation::You),
            ReplacementEffectDef::ModifyBattlefieldEntry(BattlefieldEntryModificationDef::Tapped),
            "ModifyBattlefieldEntry",
        ),
        (
            ReplacementEventDef::WouldBeginTurn {
                player: PlayerRelation::You,
                kind: TurnKindDef::Any,
            },
            ReplacementEffectDef::MoveToZone(ZoneKind::Exile),
            "MoveToZone",
        ),
        (
            ReplacementEventDef::WouldMove {
                from: ZoneKind::Hand,
                to: ZoneKind::Graveyard,
                cause: ZoneMoveCauseDef::Any,
            },
            ReplacementEffectDef::Perform(&NO_EFFECT),
            "Perform",
        ),
        (
            ReplacementEventDef::AnyObjectWouldMove {
                to: ZoneKind::Graveyard,
            },
            ReplacementEffectDef::MultiplyEventAmount(2),
            "MultiplyEventAmount",
        ),
    ];

    for (event, effect, operation) in cases {
        let ability = AbilityDef::defined_replacement(
            "Replace an event.",
            ReplacementAbilityDef::new().with_event(event),
            effect,
        );
        let mut card = definition(1, "Test Card", CardSet::Alpha);
        let rules = card.rules.with_ability(ability);
        set_primary_rules(&mut card, &rules);
        assert_eq!(
            error(card),
            CatalogError::UnsupportedReplacementProgram {
                definition: CardDefinitionId(1),
                part: CardPartId::PRIMARY,
                ability: AbilityId::PRIMARY,
                event,
                operation,
            },
        );
    }
}

#[test]
fn replacement_event_validation_accepts_each_supported_program_family() {
    static UNTAP_SOURCE: EffectDef = EffectDef::Untap {
        object: EffectRecipientDef::Source,
    };
    static BEGIN_TURN: [ReplacementEffectDef; 2] = [
        ReplacementEffectDef::ReplaceEventWithNothing,
        ReplacementEffectDef::Perform(&UNTAP_SOURCE),
    ];
    static TAKE_EXTRA_TURN: EffectDef = EffectDef::TakeExtraTurn {
        player: EffectRecipientDef::Controller,
    };
    static BATTLEFIELD_EXIT: [ReplacementEffectDef; 2] = [
        ReplacementEffectDef::MoveToZone(ZoneKind::Exile),
        ReplacementEffectDef::Perform(&TAKE_EXTRA_TURN),
    ];

    let cases = [
        (
            ReplacementEventDef::SourceEntersBattlefield,
            ReplacementEffectDef::ModifyBattlefieldEntry(BattlefieldEntryModificationDef::Tapped),
        ),
        (
            ReplacementEventDef::WouldGainLife(PlayerRelation::You),
            ReplacementEffectDef::MultiplyEventAmount(2),
        ),
        (
            ReplacementEventDef::WouldBeginTurn {
                player: PlayerRelation::You,
                kind: TurnKindDef::Any,
            },
            ReplacementEffectDef::Sequence(&BEGIN_TURN),
        ),
        (
            ReplacementEventDef::WouldMove {
                from: ZoneKind::Hand,
                to: ZoneKind::Graveyard,
                cause: ZoneMoveCauseDef::Any,
            },
            ReplacementEffectDef::MoveToZone(ZoneKind::Battlefield),
        ),
        (
            ReplacementEventDef::WouldMove {
                from: ZoneKind::Battlefield,
                to: ZoneKind::Graveyard,
                cause: ZoneMoveCauseDef::Any,
            },
            ReplacementEffectDef::Sequence(&BATTLEFIELD_EXIT),
        ),
        (
            ReplacementEventDef::AnyObjectWouldMove {
                to: ZoneKind::Graveyard,
            },
            ReplacementEffectDef::MoveToZone(ZoneKind::Exile),
        ),
    ];

    for (event, effect) in cases {
        let ability = AbilityDef::defined_replacement(
            "Replace an event.",
            ReplacementAbilityDef::new().with_event(event),
            effect,
        );
        let mut card = definition(1, "Test Card", CardSet::Alpha);
        let rules = card.rules.with_ability(ability);
        set_primary_rules(&mut card, &rules);
        CardCatalog::new([card]).expect("the event's shared runtime supports this program");
    }
}

#[test]
fn installed_triggers_retain_installer_targets_and_reject_fresh_target_scopes() {
    static INSTALLER_TARGETS: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
        AbilityTargetPredicate::Object {
            object: ObjectPredicateDef::Any,
            zones: &[ZoneKind::Battlefield],
            controller: None,
            owner: None,
        },
    )];
    static FRESH_TARGETS: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
        AbilityTargetPredicate::Object {
            object: ObjectPredicateDef::Any,
            zones: &[ZoneKind::Battlefield],
            controller: None,
            owner: None,
        },
    )];
    static LEXICAL_EFFECT: EffectDef = EffectDef::Destroy {
        object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
        can_regenerate: true,
    };
    static LEXICAL_TRIGGER: AbilityDef = AbilityDef::triggered(
        "At the beginning of the next end step, destroy that permanent.",
        TriggerEventDef::StepBegins {
            step: TurnStepDef::End,
            player: PlayerRelation::Any,
        },
        LEXICAL_EFFECT,
    );
    static FRESH_TARGET_TRIGGER: AbilityDef = AbilityDef::triggered_with_targets(
        "At the beginning of the next end step, destroy target permanent.",
        TriggerEventDef::StepBegins {
            step: TurnStepDef::End,
            player: PlayerRelation::Any,
        },
        &FRESH_TARGETS,
        LEXICAL_EFFECT,
    );
    static LEGACY_TRIGGER: AbilityDef = AbilityDef::triggered(
        "At the beginning of the next end step, destroy that permanent.",
        TriggerEventDef::StepBegins {
            step: TurnStepDef::End,
            player: PlayerRelation::Any,
        },
        LEXICAL_EFFECT,
    )
    .with_legacy_procedure();

    super::validate_ability_targets(
        &INSTALLER_TARGETS,
        EffectDef::InstallTrigger(InstalledTriggerDef::once(&LEXICAL_TRIGGER)),
    )
    .expect("a targetless installed trigger may retain its installer's target slots");

    for ability in [&FRESH_TARGET_TRIGGER, &LEGACY_TRIGGER] {
        assert_eq!(
            super::validate_ability_targets(
                &INSTALLER_TARGETS,
                EffectDef::InstallTrigger(InstalledTriggerDef::once(ability)),
            ),
            Err(GrantedAbilityValidationError::UnsupportedInstalledTriggerAbility),
        );
    }

    static CONDITIONLESS_STATE_TRIGGER: AbilityDef = AbilityDef::triggered(
        "Whenever an unspecified state exists, trigger.",
        TriggerEventDef::StateCondition,
        EffectDef::None,
    );
    assert_eq!(
        super::validate_ability_targets(
            &[],
            EffectDef::InstallTrigger(InstalledTriggerDef::once(&CONDITIONLESS_STATE_TRIGGER,)),
        ),
        Err(GrantedAbilityValidationError::UnsupportedTriggerEvent {
            event: TriggerEventDef::StateCondition,
        }),
    );
    static STATE_CONDITION: TriggerConditionDef = TriggerConditionDef::SourceOnBattlefield;
    static CONDITIONAL_STATE_TRIGGER: AbilityDef = AbilityDef::triggered_if(
        "Whenever this remains on the battlefield, trigger.",
        TriggerEventDef::StateCondition,
        &STATE_CONDITION,
        EffectDef::None,
    );
    assert_eq!(
        super::validate_ability_targets(
            &[],
            EffectDef::InstallTrigger(InstalledTriggerDef::once(&CONDITIONAL_STATE_TRIGGER,)),
        ),
        Err(GrantedAbilityValidationError::UnsupportedTriggerEvent {
            event: TriggerEventDef::StateCondition,
        }),
        "installed state triggers stay rejected until Once consumption joins state capture",
    );
    static WRONG_ZONE_TRIGGER: AbilityDef = AbilityDef::triggered(
        "At the beginning of the next end step, trigger.",
        TriggerEventDef::StepBegins {
            step: TurnStepDef::End,
            player: PlayerRelation::Any,
        },
        EffectDef::None,
    )
    .with_source_zones(&[ZoneKind::Graveyard]);
    assert_eq!(
        super::validate_ability_targets(
            &[],
            EffectDef::InstallTrigger(InstalledTriggerDef::once(&WRONG_ZONE_TRIGGER)),
        ),
        Err(GrantedAbilityValidationError::UnsupportedInstalledTriggerAbility),
    );

    static INVALID_SPELL: AbilityDef = AbilityDef::spell_with_targets(
        "Install an unsupported delayed trigger.",
        &INSTALLER_TARGETS,
        EffectDef::InstallTrigger(InstalledTriggerDef::once(&FRESH_TARGET_TRIGGER)),
    );
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_ability(INVALID_SPELL);
    set_primary_rules(&mut card, &rules);
    assert_eq!(
        error(card),
        CatalogError::UnsupportedInstalledTriggerAbility {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
        },
    );

    static INVALID_STATE_INSTALL_SPELL: AbilityDef = AbilityDef::spell(
        "Install an unsupported state trigger.",
        EffectDef::InstallTrigger(InstalledTriggerDef::once(&CONDITIONLESS_STATE_TRIGGER)),
    );
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_ability(INVALID_STATE_INSTALL_SPELL);
    set_primary_rules(&mut card, &rules);
    assert_eq!(
        error(card),
        CatalogError::UnsupportedTriggerEvent {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            event: TriggerEventDef::StateCondition,
        },
    );
}

fn definition_with_ability(ability: AbilityDef) -> CardDefinition {
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_ability(ability);
    set_primary_rules(&mut card, &rules);
    card
}

#[test]
fn shared_trigger_catalog_rejects_undiscoverable_or_incomplete_listeners() {
    static CONDITION: TriggerConditionDef = TriggerConditionDef::SourceOnBattlefield;
    let upkeep = TriggerEventDef::StepBegins {
        step: TurnStepDef::Upkeep,
        player: PlayerRelation::You,
    };
    let outside_battlefield = AbilityDef::triggered("At upkeep, trigger.", upkeep, EffectDef::None)
        .with_source_zones(&[ZoneKind::Graveyard]);
    let mixed_zones = AbilityDef::triggered("At upkeep, trigger.", upkeep, EffectDef::None)
        .with_source_zones(&[ZoneKind::Battlefield, ZoneKind::Graveyard]);
    let state_without_condition = AbilityDef::triggered(
        "Trigger whenever a state exists.",
        TriggerEventDef::StateCondition,
        EffectDef::None,
    );
    let conditional_mana = AbilityDef::defined(
        "Whenever this is tapped for mana, if it remains on the battlefield, add {B}.",
        DeclarativeAbilityDef::TriggeredMana(
            match AbilityDef::triggered_mana(
                "placeholder",
                TriggerEventDef::tapped_for_mana(ObjectPredicateDef::Source),
                EffectDef::None,
            )
            .definition
            {
                DeclarativeAbilityDef::TriggeredMana(definition) => {
                    definition.with_condition(&CONDITION)
                }
                _ => unreachable!(),
            },
        ),
        EffectDef::AddMana(crate::card::AddManaEffectDef::one(
            crate::card::ManaColor::Black,
        )),
    );
    let ordinary_tap_mana = AbilityDef::triggered_mana(
        "Whenever this becomes tapped, add {B}.",
        TriggerEventDef::tapped(ObjectPredicateDef::Source),
        EffectDef::AddMana(crate::card::AddManaEffectDef::one(
            crate::card::ManaColor::Black,
        )),
    );
    let damage_mana = AbilityDef::triggered_mana(
        "Whenever this deals damage, add {B}.",
        TriggerEventDef::damage_dealt_by(ObjectPredicateDef::Source),
        EffectDef::AddMana(crate::card::AddManaEffectDef::one(
            crate::card::ManaColor::Black,
        )),
    );

    for (ability, event) in [
        (outside_battlefield, upkeep),
        (mixed_zones, upkeep),
        (state_without_condition, TriggerEventDef::StateCondition),
        (
            conditional_mana,
            TriggerEventDef::tapped_for_mana(ObjectPredicateDef::Source),
        ),
        (
            ordinary_tap_mana,
            TriggerEventDef::tapped(ObjectPredicateDef::Source),
        ),
        (
            damage_mana,
            TriggerEventDef::damage_dealt_by(ObjectPredicateDef::Source),
        ),
    ] {
        assert_eq!(
            error(definition_with_ability(ability)),
            CatalogError::UnsupportedTriggerEvent {
                definition: CardDefinitionId(1),
                part: CardPartId::PRIMARY,
                ability: AbilityId::PRIMARY,
                event,
            },
        );
    }
}

#[test]
fn triggered_mana_catalog_requires_a_nonempty_fixed_add_mana_program() {
    static MIXED_PROGRAM: [EffectDef; 2] = [
        EffectDef::AddMana(crate::card::AddManaEffectDef::one(
            crate::card::ManaColor::Black,
        )),
        EffectDef::DrawCards {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(1),
        },
    ];
    let event = TriggerEventDef::tapped_for_mana(ObjectPredicateDef::Source);
    for effect in [
        EffectDef::None,
        EffectDef::Sequence(&[]),
        EffectDef::Sequence(&MIXED_PROGRAM),
        EffectDef::AddMana(crate::card::AddManaEffectDef::choice(&[
            crate::card::ManaColor::Black,
            crate::card::ManaColor::Green,
        ])),
    ] {
        let ability =
            AbilityDef::triggered_mana("Whenever tapped for mana, add mana.", event, effect);
        assert_eq!(
            error(definition_with_ability(ability)),
            CatalogError::UnsupportedTriggeredManaProgram {
                definition: CardDefinitionId(1),
                part: CardPartId::PRIMARY,
                ability: AbilityId::PRIMARY,
            },
        );
    }
}

#[test]
fn trigger_catalog_rejects_static_only_affected_object_anchors() {
    for event in [
        TriggerEventDef::DamageDealt(DamageEventMatcherDef {
            source: DamageSourceMatcherDef::AffectedObject,
            ..DamageEventMatcherDef::ANY
        }),
        TriggerEventDef::DamageDealt(DamageEventMatcherDef {
            recipient: DamageRecipientMatcherDef::AffectedObject,
            ..DamageEventMatcherDef::ANY
        }),
        TriggerEventDef::SpellCast(ObjectPredicateDef::HasNonManaActivatedAbility),
        TriggerEventDef::DamageDealt(DamageEventMatcherDef {
            source: DamageSourceMatcherDef::Matching(
                ObjectPredicateDef::HasNonManaActivatedAbility,
            ),
            ..DamageEventMatcherDef::ANY
        }),
    ] {
        let ability =
            AbilityDef::triggered("Whenever damage is dealt, trigger.", event, EffectDef::None);
        assert_eq!(
            error(definition_with_ability(ability)),
            CatalogError::UnsupportedTriggerEvent {
                definition: CardDefinitionId(1),
                part: CardPartId::PRIMARY,
                ability: AbilityId::PRIMARY,
                event,
            },
        );
    }
}

#[test]
fn merged_effect_vocabulary_preserves_local_target_bounds() {
    static TARGETS: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
        AbilityTargetPredicate::Player(PlayerRelation::Any),
    )];
    let out_of_range = TargetIndex(1);
    let recipient = EffectRecipientDef::ControllerOfTarget(out_of_range);
    let effects = [
        EffectDef::Tap {
            object: EffectRecipientDef::objects_controlled_by_target(
                ObjectPredicateDef::Any,
                out_of_range,
            ),
        },
        EffectDef::SplitIntoPiles(SplitIntoPilesDef {
            items: PartitionItemsDef::Objects(ObjectSetDef::Query(ObjectQueryDef::new(
                ObjectPredicateDef::Any,
                &[ZoneKind::Battlefield],
            ))),
            divider: PlayerSetDef::One(PlayerRefDef::ControllerOf(ObjectRefDef::Target(
                out_of_range,
            ))),
            chooser: PlayerSetDef::One(PlayerRefDef::EffectController),
            chosen: ObjectSetBindingIndex::PRIMARY,
            unchosen: ObjectSetBindingIndex::new(1),
            then: &EffectDef::None,
        }),
        EffectDef::Mill {
            player: recipient,
            amount: ValueDef::DividedAmongTargets,
        },
        EffectDef::Apply {
            recipient,
            effect: AppliedEffectDef::Rule(AppliedRuleDef::CannotPlay(PlayRestrictionDef::new(
                PlayActionMatcherDef::CastSpell,
                ObjectPredicateDef::NoncreatureSpell,
            ))),
            duration: ResolvedEffectDurationDef::UntilEndOfTurn,
        },
    ];

    for effect in effects {
        assert_eq!(
            super::validate_ability_targets(&TARGETS, effect),
            Err(GrantedAbilityValidationError::TargetReferenceOutOfBounds {
                target: out_of_range,
                target_count: 1,
            })
        );
    }

    static VALID_SEQUENCE: [EffectDef; 2] = [
        EffectDef::DealDamage {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::DividedAmongTargets,
        },
        EffectDef::ScheduleTurnPhases(&[crate::card::TurnPhaseDef::Combat]),
    ];
    super::validate_ability_targets(&TARGETS, EffectDef::Sequence(&VALID_SEQUENCE))
        .expect("implicit divided values and target-free combat effects add no slot reference");
}

#[test]
fn pile_roles_reject_player_sets_that_can_resolve_to_multiple_players() {
    let partition = |divider, chooser| {
        EffectDef::SplitIntoPiles(SplitIntoPilesDef {
            items: PartitionItemsDef::Objects(ObjectSetDef::Query(ObjectQueryDef::new(
                ObjectPredicateDef::Any,
                &[ZoneKind::Battlefield],
            ))),
            divider,
            chooser,
            chosen: ObjectSetBindingIndex::PRIMARY,
            unchosen: ObjectSetBindingIndex::new(1),
            then: &EffectDef::None,
        })
    };

    assert_eq!(
        super::validate_ability_targets(
            &[],
            partition(
                PlayerSetDef::All,
                PlayerSetDef::One(PlayerRefDef::EffectController),
            ),
        ),
        Err(GrantedAbilityValidationError::InvalidPileRole {
            role: "divider",
            players: PlayerSetDef::All,
        })
    );
    assert_eq!(
        super::validate_ability_targets(
            &[],
            partition(
                PlayerSetDef::One(PlayerRefDef::EffectController),
                PlayerSetDef::Related(PlayerRelation::Any),
            ),
        ),
        Err(GrantedAbilityValidationError::InvalidPileRole {
            role: "chooser",
            players: PlayerSetDef::Related(PlayerRelation::Any),
        })
    );
    super::validate_ability_targets(
        &[],
        partition(
            PlayerSetDef::Related(PlayerRelation::Opponent),
            PlayerSetDef::One(PlayerRefDef::EffectController),
        ),
    )
    .expect("an opponent relation and a single player reference are singleton roles");
}

#[test]
fn authored_target_count_fits_the_positional_index_space() {
    let targets = Box::leak(
        vec![
            AbilityTargetDef::exactly_one(AbilityTargetPredicate::Player(PlayerRelation::Any),);
            257
        ]
        .into_boxed_slice(),
    );
    let abilities = Box::leak(
        vec![AbilityDef::activated_with_targets(
            "An ability with too many targets.",
            &[],
            targets,
            EffectDef::None,
        )]
        .into_boxed_slice(),
    );
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_abilities(abilities);
    set_primary_rules(&mut card, &rules);

    assert_eq!(
        error(card),
        CatalogError::TooManyAbilityTargets {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            count: 257,
        }
    );
}

#[test]
fn nested_grant_capacity_is_validated_per_granted_definition() {
    static TERMINAL: AbilityDef = AbilityDef::not_implemented(
        "A terminal granted ability.",
        "The terminal ability is intentionally not executable.",
    );
    let effects = Box::leak(
        vec![
            EffectDef::Apply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::add_ability(&TERMINAL),
                duration: ResolvedEffectDurationDef::UntilEndOfTurn,
            };
            257
        ]
        .into_boxed_slice(),
    );
    let child = Box::leak(Box::new(AbilityDef::activated(
        "This ability contains too many nested grant sites.",
        &[],
        EffectDef::Sequence(effects),
    )));

    assert_eq!(
        error(definition_granting(child)),
        CatalogError::InvalidGrantedAbility {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            grant_path: vec![GrantId::PRIMARY],
            problem: GrantedAbilityValidationError::TooManyGrantSites { count: 257 },
        }
    );
}

#[test]
fn granted_non_declarative_implementations_require_an_explanation() {
    static GRANTED: AbilityDef =
        AbilityDef::activated("An incompletely implemented ability.", &[], EffectDef::None)
            .with_coverage(AbilityCoverageDef::metadata_only(""));

    assert_eq!(
        error(definition_granting(&GRANTED)),
        CatalogError::InvalidGrantedAbility {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            grant_path: vec![GrantId::PRIMARY],
            problem: GrantedAbilityValidationError::MissingImplementationExplanation,
        }
    );
}

#[test]
fn executable_legacy_procedures_require_custom_effect_execution() {
    static LEGACY: AbilityDef = AbilityDef::activated(
        "An ability routed through the legacy procedure.",
        &[],
        EffectDef::None,
    )
    .with_coverage(AbilityCoverageDef::explained_complete(
        "The test supplies the required legacy-procedure explanation.",
    ))
    .with_legacy_procedure();

    let mut top_level = definition(1, "Test Card", CardSet::Alpha);
    let rules = top_level.rules.with_ability(LEGACY);
    set_primary_rules(&mut top_level, &rules);
    assert_eq!(
        error(top_level),
        CatalogError::LegacyProcedureRequiresCustomExecution {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
        }
    );

    assert_eq!(
        error(definition_granting(&LEGACY)),
        CatalogError::InvalidGrantedAbility {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
            grant_path: vec![GrantId::PRIMARY],
            problem: GrantedAbilityValidationError::LegacyProcedureRequiresCustomExecution,
        }
    );
}

#[test]
fn explicitly_tagged_mana_abilities_cannot_declare_targets() {
    static COSTS: [AbilityCostDef; 1] = [AbilityCostDef::TapSource];
    static TARGETS: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
        AbilityTargetPredicate::Player(PlayerRelation::Any),
    )];
    static ABILITIES: [AbilityDef; 1] = [AbilityDef::defined(
        "Target player adds mana.",
        DeclarativeAbilityDef::ActivatedMana(
            ActivatedAbilityDef::new(&COSTS).with_targets(&TARGETS),
        ),
        EffectDef::None,
    )];
    let mut card = definition(1, "Test Card", CardSet::Alpha);
    let rules = card.rules.with_abilities(&ABILITIES);
    set_primary_rules(&mut card, &rules);

    assert_eq!(
        error(card),
        CatalogError::ManaAbilityHasTargets {
            definition: CardDefinitionId(1),
            part: CardPartId::PRIMARY,
            ability: AbilityId::PRIMARY,
        }
    );
}
