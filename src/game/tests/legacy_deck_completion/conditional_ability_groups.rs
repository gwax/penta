use super::*;
use crate::card::{CardRules, abilities};
use std::sync::OnceLock;

static SEVEN_CARDS: TriggerConditionDef = TriggerConditionDef::ObjectCount {
    query: ObjectQueryDef::owned_by(
        ObjectPredicateDef::Any,
        &[ZoneKind::Graveyard],
        PlayerSetDef::Related(PlayerRelation::You),
    ),
    comparison: ComparisonDef::GreaterOrEqual,
    amount: 7,
};
static HAS_HAND: TriggerConditionDef = TriggerConditionDef::ObjectCount {
    query: ObjectQueryDef::owned_by(
        ObjectPredicateDef::Any,
        &[ZoneKind::Hand],
        PlayerSetDef::Related(PlayerRelation::You),
    ),
    comparison: ComparisonDef::GreaterOrEqual,
    amount: 1,
};
const MEMBERS: [AbilityDef; 4] = abilities::conditional(
    &SEVEN_CARDS,
    &[
        abilities::flying(),
        AbilityDef::static_ability(
            "Gets +2/+3.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(2),
                    ValueDef::Constant(3),
                ),
            },
        ),
        AbilityDef::activated(
            "Gain 2 life while untapped.",
            &[],
            EffectDef::GainLife {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(2),
            },
        )
        .with_activation_condition(&TriggerConditionDef::SourceIsUntapped),
        AbilityDef::triggered(
            "At your upkeep, gain 3 life.",
            TriggerEventDef::StepBegins {
                step: crate::card::TurnStepDef::Upkeep,
                player: PlayerRelation::You,
            },
            EffectDef::GainLife {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(3),
            },
        ),
    ],
);
const STAT_GRANTS: [AbilityDef; 2] = abilities::conditional(
    &HAS_HAND,
    &[
        AbilityDef::static_ability(
            "Gets +2/+0.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(2),
                    ValueDef::Constant(0),
                ),
            },
        ),
        AbilityDef::static_ability(
            "Gets +0/+3.",
            EffectDef::StaticApply {
                recipient: EffectRecipientDef::Source,
                effect: AppliedEffectDef::modify_power_toughness(
                    ValueDef::Constant(0),
                    ValueDef::Constant(3),
                ),
            },
        ),
    ],
);
static NESTED_RULES: CardRules = CardRules::new_creature(mana_cost!("{1}"), &[], 2, 2)
    .with_abilities(&abilities::conditional(
        &TriggerConditionDef::SourceIsUntapped,
        &abilities::conditional(&HAS_HAND, &[abilities::flying(), abilities::vigilance()]),
    ));

static GRANTOR_RULES: CardRules =
    CardRules::new_enchantment(mana_cost!("{1}")).with_ability(AbilityDef::static_ability(
        "While untapped, creatures have the conditional stat abilities.",
        EffectDef::IfCondition {
            condition: &TriggerConditionDef::SourceIsUntapped,
            then: &EffectDef::StaticApply {
                recipient: EffectRecipientDef::matching_objects(
                    ObjectPredicateDef::HasType(CardType::Creature),
                    &[ZoneKind::Battlefield],
                    PlayerRelation::Any,
                ),
                effect: AppliedEffectDef::Composite(&abilities::grants(&STAT_GRANTS)),
            },
        },
    ));

fn identity(suffix: &str) -> CardDefinitionId {
    CardDefinitionId::from_uuid(&format!("00000000-0000-0000-0000-00000000{suffix}"))
}
fn definition(id: CardDefinitionId, name: &'static str, rules: &CardRules) -> CardDefinition {
    let mut card = CardDefinition::new(id, name, crate::card::sets::alpha::SET, *rules);
    synchronize_single_part_definition(&mut card);
    card
}
fn board() -> Game {
    static CATALOG: OnceLock<CardCatalog> = OnceLock::new();
    let mut game = ready_game();
    game.catalog = CATALOG
        .get_or_init(|| {
            let mut cards: Vec<_> = game.catalog.definitions().into_iter().cloned().collect();
            cards.extend([
                definition(
                    identity("cf20"),
                    "Conditional group",
                    &CardRules::new_creature(mana_cost!("{1}"), &[], 2, 2).with_abilities(&MEMBERS),
                ),
                definition(identity("cf21"), "Nested conditional group", &NESTED_RULES),
                definition(
                    identity("cf22"),
                    "Conditional group grantor",
                    &GRANTOR_RULES,
                ),
            ]);
            CardCatalog::new(cards).unwrap()
        })
        .clone();
    game
}
fn graveyard(game: &mut Game, count: usize) {
    game.players[0].graveyard = game
        .build_zone(PlayerId::One, &vec![cards::FOREST; count])
        .unwrap();
}
fn can_activate(game: &Game, source: GameObjectId) -> bool {
    game.legal_actions(PlayerId::One).iter().any(|action| matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source))
}
fn stats(game: &Game, source: GameObjectId) -> (Option<i16>, Option<i16>) {
    let permanent = game
        .battlefield
        .iter()
        .find(|p| p.card.id == source)
        .unwrap();
    (game.power(permanent), game.toughness(permanent))
}

#[test]
fn conditional_ability_groups_change_presence_without_rewriting_member_programs() {
    for prepared in [false, true] {
        let mut game = board();
        game.set_prepared_engine_enabled(prepared);
        let source = put_ready(&mut game, identity("cf20"));
        let expected = game
            .catalog
            .get(identity("cf20"))
            .unwrap()
            .rules
            .indexed_abilities()
            .map(|a| a.id)
            .collect::<Vec<_>>();
        for count in [6, 7, 6, 7] {
            graveyard(&mut game, count);
            let permanent = game
                .battlefield
                .iter()
                .find(|p| p.card.id == source)
                .unwrap();
            assert_eq!(game.has_flying(permanent), count >= 7);
            assert_eq!(
                stats(&game, source),
                if count >= 7 {
                    (Some(4), Some(5))
                } else {
                    (Some(2), Some(2))
                }
            );
            let effective = game.effective_abilities(permanent);
            assert_eq!(effective.len(), if count >= 7 { 4 } else { 0 });
            for (ability, expected) in effective.iter().zip(&expected) {
                assert!(
                    matches!(ability.origin, AbilityOrigin::Printed { ability, .. } if ability == *expected)
                );
            }
            assert_eq!(can_activate(&game, source), count >= 7);
        }
        game.battlefield
            .iter_mut()
            .find(|p| p.card.id == source)
            .unwrap()
            .tapped = true;
        assert!(
            !can_activate(&game, source),
            "the member retains its own activation condition"
        );
    }
}

#[test]
fn conditional_ability_groups_do_not_recheck_presence_on_the_stack() {
    let mut game = board();
    let source = put_ready(&mut game, identity("cf20"));
    graveyard(&mut game, 7);
    activate(&mut game, source);
    graveyard(&mut game, 6);
    let (wire, hidden) = checkpoint_fixture(&game, PlayerId::One);
    game = Game::from_observation_checkpoint(game.catalog.clone(), game.format, &wire, &hidden, 2)
        .unwrap();
    drain_pending(&mut game);
    assert_eq!(game.players[0].life, 22);
    assert!(!can_activate(&game, source));

    game.capture_battlefield_triggers(&CommittedTriggerEvent::StepBegins {
        step: crate::card::TurnStepDef::Upkeep,
        player: PlayerId::One,
    });
    drain_pending(&mut game);
    assert_eq!(
        game.players[0].life, 22,
        "the absent trigger does not trigger"
    );
    graveyard(&mut game, 7);
    game.capture_battlefield_triggers(&CommittedTriggerEvent::StepBegins {
        step: crate::card::TurnStepDef::Upkeep,
        player: PlayerId::One,
    });
    graveyard(&mut game, 6);
    drain_pending(&mut game);
    assert_eq!(
        game.players[0].life, 25,
        "a captured trigger survives the group's disappearance"
    );
}

#[test]
fn conditional_ability_groups_nest_without_discarding_either_condition() {
    let mut game = board();
    let source = put_ready(&mut game, identity("cf21"));
    for (has_card, tapped) in [(false, false), (true, false), (true, true), (false, true)] {
        game.players[0].hand = game
            .build_zone(PlayerId::One, if has_card { &[cards::FOREST] } else { &[] })
            .unwrap();
        game.battlefield
            .iter_mut()
            .find(|p| p.card.id == source)
            .unwrap()
            .tapped = tapped;
        let permanent = game
            .battlefield
            .iter()
            .find(|p| p.card.id == source)
            .unwrap();
        assert_eq!(game.has_flying(permanent), has_card && !tapped);
        let mut prospective = permanent.clone();
        prospective.tapped = !tapped;
        assert_eq!(
            game.collect_effective_abilities(permanent, Some(&prospective))
                .len(),
            if has_card && tapped { 2 } else { 0 },
            "presence reads the prospective state rather than looking up the live source"
        );
        assert_eq!(
            game.permanent_has_executable_keyword(permanent, KeywordAbility::Vigilance),
            has_card && !tapped
        );
    }
}

#[test]
fn conditional_ability_groups_can_be_granted_as_multiple_independent_statics() {
    for prepared in [false, true] {
        let mut game = board();
        game.set_prepared_engine_enabled(prepared);
        let grantor = put_ready(&mut game, identity("cf22"));
        let first = put_ready(&mut game, cards::GRIZZLY_BEARS);
        let second = game
            .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
            .unwrap();
        game.players[0].hand = game.build_zone(PlayerId::One, &[cards::FOREST]).unwrap();
        assert_eq!(stats(&game, first), (Some(4), Some(5)));
        assert_eq!(
            stats(&game, second),
            (Some(2), Some(2)),
            "member presence reads its recipient's controller"
        );
        let permanent = game
            .battlefield
            .iter()
            .find(|p| p.card.id == first)
            .unwrap();
        let origins = game
            .effective_abilities(permanent)
            .iter()
            .filter_map(|a| matches!(a.origin, AbilityOrigin::Granted { .. }).then_some(a.origin))
            .collect::<Vec<_>>();
        assert_eq!(origins.len(), 2);
        assert_ne!(origins[0], origins[1]);
        let (wire, hidden) = checkpoint_fixture(&game, PlayerId::One);
        game =
            Game::from_observation_checkpoint(game.catalog.clone(), game.format, &wire, &hidden, 2)
                .unwrap();
        assert_eq!(stats(&game, first), (Some(4), Some(5)));
        game.battlefield
            .iter_mut()
            .find(|p| p.card.id == grantor)
            .unwrap()
            .tapped = true;
        assert_eq!(
            stats(&game, first),
            (Some(2), Some(2)),
            "the outer condition reads the granting source"
        );
    }
}

#[test]
fn conditional_ability_groups_reject_event_conditions_and_nonbattlefield_members() {
    static INVALID_RULES: [CardRules; 3] = [
        CardRules::new_creature(mana_cost!("{1}"), &[], 1, 1).with_abilities(
            &abilities::conditional(
                &TriggerConditionDef::SourceAbilityUsedThisTurn,
                &[abilities::flying()],
            ),
        ),
        CardRules::new_instant(mana_cost!("{1}")).with_abilities(&abilities::conditional(
            &SEVEN_CARDS,
            &[AbilityDef::spell(
                "Draw a card.",
                EffectDef::DrawCards {
                    recipient: EffectRecipientDef::Controller,
                    amount: ValueDef::Constant(1),
                },
            )],
        )),
        CardRules::new_creature(mana_cost!("{1}"), &[], 1, 1).with_abilities(
            &abilities::conditional(
                &SEVEN_CARDS,
                &[AbilityDef::static_ability(
                    "Defines power and toughness.",
                    EffectDef::StaticApply {
                        recipient: EffectRecipientDef::Source,
                        effect: AppliedEffectDef::Characteristic(
                            CharacteristicOperationDef::PowerToughness(
                                crate::card::PowerToughnessOperationDef::Define {
                                    power: Some(ValueDef::Constant(3)),
                                    toughness: Some(ValueDef::Constant(3)),
                                },
                            ),
                        ),
                    },
                )],
            ),
        ),
    ];
    for rules in INVALID_RULES {
        let invalid = definition(identity("cf23"), "Invalid conditional group", &rules);
        assert!(CardCatalog::new([invalid]).is_err());
    }
}
