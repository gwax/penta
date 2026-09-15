//! Shared counting semantics across entries, departures, and combat declarations.
use super::*;
use crate::card::{SimultaneousTriggerDef, TriggerAggregationDef};

const CREATURE_ENTERS: TriggerEventDef = TriggerEventDef::zone_changed(
    ObjectPredicateDef::HasType(CardType::Creature),
    None,
    Some(ZoneKind::Battlefield),
);
const GREEN_ATTACKS: TriggerEventDef = TriggerEventDef::attacks(ObjectPredicateDef::All(&[
    ObjectPredicateDef::ControlledBy(PlayerRelation::You),
    ObjectPredicateDef::Color(ManaColor::Green),
]));
const CARD_EXILED: TriggerEventDef =
    TriggerEventDef::zone_changed(ObjectPredicateDef::Any, None, Some(ZoneKind::Exile));

fn watcher(game: &mut Game, event: TriggerEventDef) -> GameObjectId {
    let abilities = Box::leak(Box::new([AbilityDef::triggered(
        "Count matching events.",
        event,
        EffectDef::GainLife {
            recipient: EffectRecipientDef::Controller,
            amount: ValueDef::Constant(1),
        },
    )]));
    game.create_token_from(
        PlayerId::One,
        crate::card::TokenCharacteristics::creature(&[], &[], 1, 1).with_abilities(abilities),
        None,
    )
}

fn counted(
    event: &'static TriggerEventDef,
    aggregation: TriggerAggregationDef,
    minimum: u16,
) -> TriggerEventDef {
    TriggerEventDef::Simultaneous(SimultaneousTriggerDef::new(event, aggregation).at_least(minimum))
}

fn counts(game: &Game, source: GameObjectId) -> usize {
    game.pending_triggers
        .iter()
        .filter(|trigger| trigger.source.object == source)
        .count()
}

fn watches(game: &mut Game, event: &'static TriggerEventDef) -> [GameObjectId; 4] {
    let sources = [
        watcher(game, *event),
        watcher(game, counted(event, TriggerAggregationDef::Each, 2)),
        watcher(game, counted(event, TriggerAggregationDef::Once, 2)),
        watcher(game, counted(event, TriggerAggregationDef::Once, 3)),
    ];
    game.pending_triggers.clear();
    sources
}

fn assert_two_matches(game: &Game, sources: [GameObjectId; 4]) {
    assert_eq!(sources.map(|source| counts(game, source)), [2, 2, 1, 0]);
    let grouped = game
        .pending_triggers
        .iter()
        .find(|t| t.source.object == sources[2])
        .unwrap();
    assert_eq!(grouped.context.trigger.amount, Some(2));
    assert_eq!(grouped.context.trigger.object, None);
    let individual = game
        .pending_triggers
        .iter()
        .filter(|t| t.source.object == sources[1])
        .map(|t| t.context.trigger.object.unwrap())
        .collect::<Vec<_>>();
    assert_ne!(individual[0], individual[1]);
}

#[test]
fn simultaneous_triggers_count_matching_entries_and_preserve_individual_objects() {
    for prepared in [false, true] {
        let mut game = ready_game();
        game.set_prepared_engine_enabled(prepared);
        let sources = watches(&mut game, &CREATURE_ENTERS);
        game.entering_together(|game| {
            game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
                .unwrap();
            game.put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
                .unwrap();
            game.put_onto_battlefield(PlayerId::One, cards::FOREST)
                .unwrap();
        });
        assert_two_matches(&game, sources);
        game.pending_triggers.clear();
        game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
            .unwrap();
        game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
            .unwrap();
        assert_eq!(
            sources.map(|source| counts(&game, source)),
            [2, 0, 0, 0],
            "separate entries do not meet a simultaneous threshold"
        );
    }
}

#[test]
fn simultaneous_triggers_count_filtered_attackers_in_one_declaration() {
    let mut game = ready_game();
    let sources = watches(&mut game, &GREEN_ATTACKS);
    for definition in [
        cards::GRIZZLY_BEARS,
        cards::LLANOWAR_ELVES,
        cards::SAVANNAH_LIONS,
    ] {
        let creature = game
            .put_onto_battlefield(PlayerId::One, definition)
            .unwrap();
        game.declare_attacker(creature, AttackDefender::Player(PlayerId::Two));
    }
    game.finish_declaring_attackers();
    assert_two_matches(&game, sources);
}

#[test]
fn simultaneous_triggers_enforce_upper_bounds_and_do_not_count_anyof_twice() {
    const EITHER: TriggerEventDef = TriggerEventDef::AnyOf(&[CREATURE_ENTERS, CREATURE_ENTERS]);
    let mut game = ready_game();
    let once = watcher(
        &mut game,
        TriggerEventDef::Simultaneous(
            SimultaneousTriggerDef::new(&EITHER, TriggerAggregationDef::Once)
                .at_least(2)
                .at_most(2),
        ),
    );
    game.pending_triggers.clear();
    for number in [1, 2, 3] {
        game.entering_together(|game| {
            for _ in 0..number {
                game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
                    .unwrap();
            }
        });
        assert_eq!(counts(&game, once), usize::from(number == 2));
        game.pending_triggers.clear();
    }
}

fn move_hand(game: &mut Game, zone: ZoneKind) {
    let object = spell(100_000, cards::TOME_SCOUR, PlayerId::One, 0);
    game.resolve_effect_def(
        ScopedEffect::primary(EffectDef::MoveObjects(crate::card::MoveObjectsDef {
            input: ObjectSetDef::Query(crate::card::ObjectQueryDef::matching(
                ObjectPredicateDef::Any,
                &[ZoneKind::Hand],
                PlayerRelation::You,
            )),
            from: Some(ZoneKind::Hand),
            zone,
            placement: ZonePlacement::Top,
            moved: None,
            then: &EffectDef::None,
        })),
        &object,
        EffectResolutionContext::empty(),
    );
}

#[test]
fn simultaneous_triggers_receive_collection_entries_as_one_batch() {
    let mut game = ready_game();
    let sources = watches(&mut game, &CREATURE_ENTERS);
    game.set_hand(
        PlayerId::One,
        &[cards::GRIZZLY_BEARS, cards::LLANOWAR_ELVES, cards::FOREST],
    )
    .unwrap();
    move_hand(&mut game, ZoneKind::Battlefield);
    assert_two_matches(&game, sources);
}

#[test]
fn simultaneous_triggers_receive_exile_moves_as_one_batch() {
    let mut game = ready_game();
    let sources = watches(&mut game, &CARD_EXILED);
    let laelia = game
        .put_onto_battlefield(PlayerId::One, cards::LAELIA_THE_BLADE_REFORGED)
        .unwrap();
    game.set_hand(PlayerId::One, &[cards::GRIZZLY_BEARS, cards::FOREST])
        .unwrap();
    move_hand(&mut game, ZoneKind::Exile);
    assert_two_matches(&game, sources);
    assert_eq!(counts(&game, laelia), 0, "Laelia does not watch the hand");
    game.pending_triggers.clear();
    game.players[0].graveyard = game
        .build_zone(PlayerId::One, &[cards::GRIZZLY_BEARS, cards::FOREST])
        .unwrap();
    let ids = game.players[0]
        .graveyard
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();
    let mut events = Vec::new();
    for id in ids {
        game.move_card_target_to_zone_collecting(
            id,
            ZoneKind::Exile,
            ZoneMoveCause::Rules,
            None,
            ZonePlacement::Top,
            &mut events,
        );
    }
    game.capture_zone_move_events(&events);
    assert_two_matches(&game, sources);
    assert_eq!(
        counts(&game, laelia),
        1,
        "existing grouped exile clauses also receive the whole move"
    );
}

#[test]
fn simultaneous_triggers_wait_for_all_entry_choices_and_all_arrivals() {
    const ENTERS: TriggerEventDef =
        TriggerEventDef::zone_changed(ObjectPredicateDef::Any, None, Some(ZoneKind::Battlefield));
    let mut game = ready_game();
    let grouped = watcher(&mut game, counted(&ENTERS, TriggerAggregationDef::Once, 2));
    game.pending_triggers.clear();
    game.set_hand(
        PlayerId::One,
        &[cards::GRIZZLY_BEARS, cards::CAVERN_OF_SOULS],
    )
    .unwrap();
    move_hand(&mut game, ZoneKind::Battlefield);
    assert_eq!(counts(&game, grouped), 0);
    assert!(
        game.ready_entry_batch
            .as_ref()
            .is_some_and(|ready| ready.len() == 1)
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|p| p.card.definition.card_definition() == Some(cards::GRIZZLY_BEARS))
    );
    choose_decision_by_label(&mut game, PlayerId::One, "Elf");
    assert!(game.ready_entry_batch.is_none());
    assert_eq!(
        game.stack
            .iter()
            .filter(|s| s.source == Some(grouped))
            .count()
            + counts(&game, grouped),
        1
    );
}

#[test]
fn simultaneous_entry_choices_round_trip_with_ready_entrants() {
    let mut game = ready_game();
    let token = game
        .catalog
        .get(cards::STAFF_OF_THE_STORYTELLER)
        .unwrap()
        .parts
        .iter()
        .flat_map(|part| part.rules.indexed_abilities())
        .find_map(|ability| match ability.definition.declarative_effect() {
            Some(EffectDef::CreateToken(crate::card::CreateTokenDef {
                token: crate::card::TokenDef::Literal(token),
                ..
            })) => Some(token),
            _ => None,
        })
        .expect("Staff defines its Spirit token");
    game.put_onto_battlefield(PlayerId::One, cards::SOUL_WARDEN)
        .unwrap();
    game.put_onto_battlefield(PlayerId::One, cards::CARETAKER_S_TALENT)
        .unwrap();
    let staff = game
        .put_onto_battlefield(PlayerId::One, cards::STAFF_OF_THE_STORYTELLER)
        .unwrap();
    game.pending_triggers.clear();
    game.set_hand(
        PlayerId::One,
        &[cards::GRIZZLY_BEARS, cards::CAVERN_OF_SOULS],
    )
    .unwrap();
    let cards = game.players[0]
        .hand
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();
    let mut tokens = Vec::new();
    game.entering_together(|game| {
        tokens.push(game.create_token_from(PlayerId::One, token, Some(staff)));
        for card in cards {
            game.move_target_to_zone(
                Target::Card(card),
                ZoneKind::Battlefield,
                ZoneMoveCause::Rules,
                None,
                ZonePlacement::Top,
            );
        }
    });
    game.capture_tokens_created(PlayerId::One, &tokens);
    for viewer in [PlayerId::One, PlayerId::Two] {
        let (wire, hidden) = checkpoint_fixture(&game, viewer);
        if viewer == PlayerId::Two {
            assert_eq!(
                wire["checkpoint"]["hasDeferredState"], true,
                "the other seat cannot reconstruct a private naming decision"
            );
            continue;
        }
        let mut restored = Game::from_observation_checkpoint(
            game.catalog.clone(),
            game.format,
            &wire,
            &hidden,
            42,
        )
        .unwrap();
        assert_eq!(restored.deferred_token_creations.len(), 1);
        choose_decision_by_label(&mut restored, PlayerId::One, "Elf");
        drain_pending(&mut restored);
        assert!(restored.ready_entry_batch.is_none());
        assert_eq!(restored.players[0].life, 22);
        assert_eq!(
            restored.players[0].hand.len(),
            1,
            "Caretaker's Talent sees the token arrival"
        );
        assert!(restored.deferred_token_creations.is_empty());
        assert_eq!(
            restored
                .battlefield
                .iter()
                .find(|p| p.card.id == staff)
                .unwrap()
                .counters(CounterKind::named("story")),
            1,
            "Staff sees the deferred creation event"
        );
        assert!(
            restored
                .battlefield
                .iter()
                .any(|p| p.card.definition.card_definition() == Some(cards::GRIZZLY_BEARS))
        );
        assert!(
            restored
                .battlefield
                .iter()
                .any(|p| p.card.definition.card_definition() == Some(cards::CAVERN_OF_SOULS))
        );
    }
}

#[test]
fn simultaneous_trigger_thresholds_exclude_suppressed_causes() {
    const ENTERS: TriggerEventDef =
        TriggerEventDef::zone_changed(ObjectPredicateDef::Any, None, Some(ZoneKind::Battlefield));
    let mut game = ready_game();
    game.put_onto_battlefield(PlayerId::One, cards::TORPOR_ORB)
        .unwrap();
    let grouped = watcher(&mut game, counted(&ENTERS, TriggerAggregationDef::Once, 2));
    game.pending_triggers.clear();
    for forests in [1, 2] {
        game.entering_together(|game| {
            game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
                .unwrap();
            for _ in 0..forests {
                game.put_onto_battlefield(PlayerId::One, cards::FOREST)
                    .unwrap();
            }
        });
        assert_eq!(counts(&game, grouped), usize::from(forests == 2));
        if forests == 2 {
            assert_eq!(
                game.pending_triggers
                    .iter()
                    .find(|t| t.source.object == grouped)
                    .unwrap()
                    .context
                    .trigger
                    .amount,
                Some(2)
            );
        }
        game.pending_triggers.clear();
    }
}

#[test]
fn simultaneous_trigger_observers_and_modifiers_inspect_later_members() {
    const ENTERS: TriggerEventDef =
        TriggerEventDef::zone_changed(ObjectPredicateDef::Any, None, Some(ZoneKind::Battlefield));
    const ARTIFACT_ENTERS: TriggerEventDef = TriggerEventDef::zone_changed(
        ObjectPredicateDef::HasType(CardType::Artifact),
        None,
        Some(ZoneKind::Battlefield),
    );
    let mut game = ready_game();
    game.put_onto_battlefield(PlayerId::One, cards::GANDALF_THE_WHITE)
        .unwrap();
    let grouped = watcher(&mut game, counted(&ENTERS, TriggerAggregationDef::Once, 2));
    let observer = watcher(
        &mut game,
        TriggerEventDef::AbilityTriggeredBy(&ARTIFACT_ENTERS),
    );
    game.pending_triggers.clear();
    game.entering_together(|game| {
        game.put_onto_battlefield(PlayerId::One, cards::FOREST)
            .unwrap();
        game.put_onto_battlefield(PlayerId::One, cards::SOL_RING)
            .unwrap();
    });
    assert_eq!(
        counts(&game, grouped),
        2,
        "Gandalf adds one occurrence for the artifact later in the batch"
    );
    assert_eq!(
        counts(&game, observer),
        2,
        "each occurrence retains all contributing causes"
    );
}

#[test]
fn simultaneous_attack_threshold_is_used_by_armasaur_guide() {
    for amount in [2, 3] {
        let mut game = ready_game();
        let source = game
            .put_onto_battlefield(PlayerId::One, cards::ARMASAUR_GUIDE)
            .unwrap();
        for _ in 0..amount {
            let attacker = game
                .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
                .unwrap();
            game.declare_attacker(attacker, AttackDefender::Player(PlayerId::Two));
        }
        game.finish_declaring_attackers();
        assert_eq!(counts(&game, source), usize::from(amount >= 3));
    }
}

#[test]
fn simultaneous_token_creation_waits_for_an_entry_choice() {
    let mut game = ready_game();
    let source = watcher(
        &mut game,
        TriggerEventDef::TokensCreated {
            player: PlayerRelation::You,
            token: ObjectPredicateDef::Any,
        },
    );
    let mut minted = Vec::new();
    game.entering_together(|game| {
        minted.push(game.create_token_from(
            PlayerId::One,
            crate::card::TokenCharacteristics::creature(&["Elf"], &[], 1, 1),
            None,
        ));
        game.put_onto_battlefield(PlayerId::One, cards::CAVERN_OF_SOULS)
            .unwrap();
    });
    game.capture_tokens_created(PlayerId::One, &minted);
    assert_eq!(counts(&game, source), 0);
    assert_eq!(game.deferred_token_creations.len(), 1);
    choose_decision_by_label(&mut game, PlayerId::One, "Elf");
    assert!(game.deferred_token_creations.is_empty());
    assert_eq!(
        game.stack
            .iter()
            .filter(|s| s.source == Some(source))
            .count()
            + counts(&game, source),
        1
    );
}
