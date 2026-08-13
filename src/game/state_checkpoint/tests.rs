use super::*;
use crate::ManaColor;
use crate::card::KeywordAbility;
use crate::game::DecisionContinuation;
use crate::policy::Policy;
use serde_json::json;

#[test]
fn catalog_semantics_rehydrate_an_animation_without_a_card_name_switch() {
    let catalog = crate::poc::catalog().expect("catalog builds");
    let key = animation_snapshot(&crate::card::abilities::MISHRAS_FACTORY_ANIMATION);
    let rebuilt = catalog_animation(&catalog, &key).expect("animation is cataloged");
    assert_eq!(*rebuilt, crate::card::abilities::MISHRAS_FACTORY_ANIMATION);
}

#[test]
fn catalog_semantics_rehydrate_top_level_and_nested_abilities() {
    let catalog = crate::poc::catalog().expect("catalog builds");
    let top_level = catalog
        .definitions()
        .into_iter()
        .flat_map(|definition| &definition.parts)
        .flat_map(|part| part.rules.indexed_abilities())
        .next()
        .expect("catalog has an ability")
        .definition;
    let locator = ability_locator(&catalog, |candidate| *candidate == top_level)
        .expect("top-level ability has a locator");
    assert_eq!(catalog_ability(&catalog, &locator), Some(top_level));

    let granted_text =
        "At the beginning of your upkeep, sacrifice this artifact unless you pay {2}.";
    let locator = ability_locator(&catalog, |candidate| candidate.text == granted_text)
        .expect("nested granted ability has a locator");
    let rebuilt = catalog_ability(&catalog, &locator).expect("nested locator resolves");
    assert_eq!(rebuilt.text, granted_text);
    assert!(
        !locator.nested.is_empty(),
        "the granted clause is addressed beneath its printed source"
    );
}

#[test]
fn every_runtime_keyword_has_a_stable_checkpoint_round_trip() {
    let mut keywords = vec![
        KeywordAbility::Flying,
        KeywordAbility::Trample,
        KeywordAbility::Haste,
        KeywordAbility::FirstStrike,
        KeywordAbility::DoubleStrike,
        KeywordAbility::Banding,
        KeywordAbility::Vigilance,
        KeywordAbility::Defender,
        KeywordAbility::Deathtouch,
        KeywordAbility::Lifelink,
        KeywordAbility::Reach,
        KeywordAbility::Flash,
        KeywordAbility::Hexproof,
        KeywordAbility::Shroud,
        KeywordAbility::Intimidate,
        KeywordAbility::Undying,
        KeywordAbility::Indestructible,
        KeywordAbility::AttacksEachCombatIfAble,
        KeywordAbility::Mountainwalk,
        KeywordAbility::Forestwalk,
    ];
    keywords.extend(
        [
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
            ManaColor::Colorless,
        ]
        .map(KeywordAbility::ProtectionFrom),
    );
    for keyword in keywords {
        assert_eq!(parse_keyword(keyword_snapshot(keyword)), keyword);
    }
}

#[test]
fn checkpoint_redacts_opposing_hidden_object_ids() {
    let catalog = crate::poc::catalog().expect("catalog builds");
    let deck = crate::Deck {
        main: vec![crate::card::cards::MOUNTAIN; 60],
        sideboard: Vec::new(),
    };
    let mut game = Game::new(catalog, [deck.clone(), deck], 41).expect("game starts");
    let hidden = game.players[PlayerId::Two.index()].hand[0].id;
    game.drawn_this_turn[PlayerId::Two.index()] = vec![hidden];
    game.miracle_window = Some(hidden);

    let checkpoint = game.checkpoint_json(PlayerId::One);
    assert_eq!(checkpoint["drawnThisTurn"][1], json!([]));
    assert!(checkpoint["miracleWindow"].is_null());

    let own = game.players[PlayerId::One.index()].hand[0].id;
    game.drawn_this_turn[PlayerId::One.index()] = vec![own];
    game.miracle_window = Some(own);
    let checkpoint = game.checkpoint_json(PlayerId::One);
    assert_eq!(checkpoint["drawnThisTurn"][0], json!([own.0]));
    assert_eq!(checkpoint["miracleWindow"], own.0);
}

#[test]
fn checkpoint_json_is_a_projection_of_one_typed_snapshot_schema() {
    let catalog = crate::poc::catalog().expect("catalog builds");
    let deck = crate::Deck {
        main: vec![crate::card::cards::MOUNTAIN; 60],
        sideboard: Vec::new(),
    };
    let game = Game::new(catalog, [deck.clone(), deck], 42).expect("game starts");
    let json = game.checkpoint_json(PlayerId::One);
    let snapshot: GameSnapshot =
        serde_json::from_value(json.clone()).expect("checkpoint matches GameSnapshot");
    assert_eq!(
        serde_json::to_value(snapshot).expect("snapshot serializes"),
        json
    );

    let mut malformed = json;
    malformed
        .as_object_mut()
        .expect("snapshot object")
        .remove("turnsStarted");
    assert!(
        serde_json::from_value::<GameSnapshot>(malformed)
            .expect_err("missing required state must fail")
            .to_string()
            .contains("turnsStarted")
    );
}

#[test]
fn a_supported_pending_decision_rebuilds_and_resumes() {
    let catalog = crate::poc::catalog().expect("catalog builds");
    let deck = crate::Deck {
        main: vec![crate::card::cards::MOUNTAIN; 60],
        sideboard: Vec::new(),
    };
    let mut game = Game::new(catalog.clone(), [deck.clone(), deck], 43).expect("game starts");
    let player = PlayerId::One;
    let card = game.players[player.index()].hand[0].id;
    game.queue_miracle_reveal(player, card);

    let observation = game.observe(player);
    let actions = crate::protocol::protocol_actions(&observation);
    let observation_json = crate::protocol::observation_json_for_format(
        &catalog,
        game.format,
        &observation,
        true,
        &actions,
    );
    let definitions = |cards: &[CardInstance]| {
        cards
            .iter()
            .map(|card| card.definition.0)
            .collect::<Vec<_>>()
    };
    let hidden = json!({
        "hands": {
            "p2": definitions(&game.players[PlayerId::Two.index()].hand),
        },
        "libraries": {
            "p1": definitions(&game.players[PlayerId::One.index()].library),
            "p2": definitions(&game.players[PlayerId::Two.index()].library),
        },
    });

    assert_eq!(observation_json["checkpoint"]["hasDeferredState"], false);
    let mut rebuilt =
        Game::from_observation_checkpoint(catalog, game.format, &observation_json, &hidden, 1_007)
            .expect("supported decision reconstructs");
    assert_eq!(rebuilt.pending_decisions.len(), 1);
    let rebuilt_observation = rebuilt.observe(player);
    assert_eq!(
        crate::protocol::protocol_actions(&rebuilt_observation),
        actions,
        "the rebuilt decision offers the same indexed actions"
    );
    assert!(matches!(
        rebuilt.pending_decisions[0].continuation,
        DecisionContinuation::MiracleReveal { card: rebuilt_card } if rebuilt_card == card
    ));
    let decision = rebuilt.pending_decisions[0].observation.id;
    rebuilt.choose_decision(player, decision, &[1]);
    assert_eq!(rebuilt.miracle_window, Some(card));
}

#[test]
fn uncataloged_executable_state_fails_closed() {
    let catalog = crate::poc::catalog().expect("catalog builds");
    let deck = crate::Deck {
        main: vec![crate::card::cards::MOUNTAIN; 60],
        sideboard: Vec::new(),
    };
    let mut game = Game::new(catalog, [deck.clone(), deck], 47).expect("game starts");
    let player = PlayerId::One;
    let card = game.players[player.index()].hand[0].id;
    game.temporary_ability_grants.push(TemporaryAbilityGrant {
        object: card,
        ability: crate::card::AbilityDef::static_ability(
            "An intentionally uncataloged test ability.",
            crate::card::EffectDef::None,
        ),
    });

    let checkpoint = game.checkpoint_json(player);
    assert_eq!(checkpoint["hasDeferredState"], true);
}

#[test]
fn an_emblem_rebuilds_with_identity_and_source_provenance() {
    let catalog = crate::poc::catalog().expect("catalog builds");
    let deck = crate::Deck {
        main: vec![crate::card::cards::MOUNTAIN; 60],
        sideboard: Vec::new(),
    };
    let mut game = Game::new(catalog.clone(), [deck.clone(), deck], 53).expect("game starts");
    let controller = PlayerId::One;
    let definition = crate::card::cards::DOMRI_RADE_EMBLEM;
    let card = game.unbacked_object(
        definition,
        controller,
        CharacteristicSource::Ability(definition),
    );
    let emblem_id = card.id;
    let mut emblem = Permanent::entering(
        card,
        CardPartId::PRIMARY,
        controller,
        game.turns_started[controller.index()],
    );
    emblem.timestamp = game.allocate_continuous_effect_timestamp();
    emblem.emblem_source = Some(AbilityOrigin::Printed {
        definition: crate::card::cards::DOMRI_RADE,
        part: CardPartId::PRIMARY,
        ability: AbilityId(2),
    });
    game.emblems.push(emblem);

    let observation = game.observe(controller);
    let actions = crate::protocol::protocol_actions(&observation);
    let observation_json = crate::protocol::observation_json_for_format(
        &catalog,
        game.format,
        &observation,
        true,
        &actions,
    );
    let definitions = |cards: &[CardInstance]| {
        cards
            .iter()
            .map(|card| card.definition.0)
            .collect::<Vec<_>>()
    };
    let hidden = json!({
        "hands": {
            "p2": definitions(&game.players[PlayerId::Two.index()].hand),
        },
        "libraries": {
            "p1": definitions(&game.players[PlayerId::One.index()].library),
            "p2": definitions(&game.players[PlayerId::Two.index()].library),
        },
    });
    assert_eq!(observation_json["checkpoint"]["hasDeferredState"], false);

    let rebuilt =
        Game::from_observation_checkpoint(catalog, game.format, &observation_json, &hidden, 1_009)
            .expect("emblem reconstructs");
    assert_eq!(rebuilt.emblems.len(), 1);
    assert_eq!(rebuilt.emblems[0].card.id, emblem_id);
    assert_eq!(rebuilt.observed_emblems(), observation.emblems);
}

#[test]
fn a_spell_created_delayed_trigger_reconstructs_at_an_action_boundary() {
    let mut game = crate::game::tests::ready_game();
    let player = PlayerId::One;
    let whelp_id = GameObjectId(10_000);
    game.battlefield.push(crate::game::tests::creature(
        whelp_id.0,
        crate::card::cards::DRAGON_WHELP,
        player,
    ));

    for _ in 0..4 {
        game.players[player.index()].mana_pool.red = 1;
        let ability = {
            let whelp = game
                .battlefield
                .iter()
                .find(|permanent| permanent.card.id == whelp_id)
                .expect("Dragon Whelp remains on the battlefield");
            game.activated_ability_origin(whelp, 0)
        };
        game.apply(
            player,
            crate::Action::ActivateAbility {
                source: whelp_id,
                ability,
                targets: Vec::new(),
                cost_object: None,
                x: 0,
            },
        )
        .expect("Dragon Whelp activates");
        for _ in 0..4 {
            if game.stack.is_empty() {
                break;
            }
            let priority = game.priority;
            game.apply(priority, crate::Action::PassPriority)
                .expect("priority passes while resolving the activation");
        }
    }
    assert_eq!(game.delayed_triggers.len(), 1);
    game.sacrifice_permanent(whelp_id);
    assert!(game.retired_objects.contains_key(&whelp_id));

    let viewer = game.decision_player().expect("the game awaits an action");
    let observation = game.observe(viewer);
    let actions = crate::protocol::protocol_actions(&observation);
    let wire = crate::protocol::observation_json_for_format(
        &game.catalog,
        game.format,
        &observation,
        game.in_pregame(),
        &actions,
    );
    assert_eq!(wire["checkpoint"]["hasDeferredState"], false);
    let rebuilt = Game::from_observation_checkpoint(
        game.catalog.clone(),
        game.format,
        &wire,
        &true_hidden_hypothesis(&game, viewer),
        1_011,
    )
    .expect("the delayed trigger reconstructs");
    assert_eq!(rebuilt.delayed_triggers.len(), 1);
    assert!(rebuilt.retired_objects.contains_key(&whelp_id));
    let rebuilt_observation = rebuilt.observe(viewer);
    let rebuilt_actions = crate::protocol::protocol_actions(&rebuilt_observation);
    assert_eq!(rebuilt_actions, actions);
    assert_eq!(
        crate::protocol::observation_json_for_format(
            &rebuilt.catalog,
            rebuilt.format,
            &rebuilt_observation,
            rebuilt.in_pregame(),
            &rebuilt_actions,
        ),
        wire
    );
}

#[test]
#[ignore = "slow decision-boundary reconstruction audit"]
#[allow(clippy::too_many_lines)]
fn every_sampled_game_decision_reconstructs_from_its_observation() {
    let catalog = crate::poc::catalog().expect("catalog builds");
    let mut audited = 0_usize;
    for (format, games) in [
        (crate::Format::OldSchool9394, 8_usize),
        (crate::Format::IsdRtrStandard, 8_usize),
    ] {
        let decks = crate::protocol::deck_names_for_format(format);
        for game_index in 0..games {
            let first =
                crate::protocol::deck_by_name_for_format(format, decks[game_index % decks.len()])
                    .expect("first deck exists");
            let second = crate::protocol::deck_by_name_for_format(
                format,
                decks[(game_index * 5 + 3) % decks.len()],
            )
            .expect("second deck exists");
            let seed = 10_000 + u64::try_from(game_index).expect("game index fits") * 97;
            let mut game = Game::new_with_format(format, catalog.clone(), [first, second], seed)
                .expect("game starts");
            let mut policies = [
                crate::RandomPolicy::new(seed ^ 0x1111),
                crate::RandomPolicy::new(seed ^ 0x2222),
            ];

            for action_number in 0..2_000 {
                let Some(viewer) = game.decision_player() else {
                    break;
                };
                let observation = game.observe(viewer);
                let actions = crate::protocol::protocol_actions(&observation);
                let wire = crate::protocol::observation_json_for_format(
                    &catalog,
                    format,
                    &observation,
                    game.in_pregame(),
                    &actions,
                );
                let hidden = true_hidden_hypothesis(&game, viewer);
                let rebuilt = Game::from_observation_checkpoint(
                    catalog.clone(),
                    format,
                    &wire,
                    &hidden,
                    seed ^ 0x5555,
                )
                .unwrap_or_else(|error| {
                    let damage_source_retired = game
                        .battlefield
                        .iter()
                        .flat_map(|permanent| permanent.damage_sources.iter().copied())
                        .filter_map(|id| {
                            game.retired_objects.get(&id).map(|retired| {
                                (
                                    id,
                                    match retired {
                                        RetiredObject::Card(_) => "card",
                                        RetiredObject::Permanent { .. } => "permanent",
                                        RetiredObject::Stack(_) => "stack",
                                    },
                                )
                            })
                        })
                        .collect::<Vec<_>>();
                    panic!(
                        "{format:?} game {game_index} action {action_number} ({:?} {:?}) grants={} delayed={} floating={} events={} triggers={} restricted_mana={} damage_sources={} damage_source_retired={damage_source_retired:?} stack_objects={} retired_objects={}: {error}",
                        game.step,
                        game.pending_decisions.first().map(|decision| &decision.continuation),
                        game.temporary_ability_grants.len(),
                        game.delayed_triggers.len(),
                        game.floating_triggers.len(),
                        game.pending_events.len(),
                        game.pending_triggers.len(),
                        game.players.iter().flat_map(|player| &player.mana).filter(|mana| mana.source.is_some() || !mana.restrictions.is_empty() || !mana.spend_effects.is_empty()).count(),
                        game.battlefield.iter().filter(|permanent| !permanent.damage_sources.is_empty()).count(),
                        game.stack.len(),
                        game.retired_objects.len(),
                    )
                });
                let rebuilt_observation = rebuilt.observe(viewer);
                let rebuilt_actions = crate::protocol::protocol_actions(&rebuilt_observation);
                assert_eq!(
                    rebuilt_actions, actions,
                    "{format:?} game {game_index} action {action_number} rebuilt different actions",
                );
                let rebuilt_wire = crate::protocol::observation_json_for_format(
                    &catalog,
                    format,
                    &rebuilt_observation,
                    rebuilt.in_pregame(),
                    &rebuilt_actions,
                );
                assert_eq!(
                    rebuilt_wire, wire,
                    "{format:?} game {game_index} action {action_number} rebuilt different public state",
                );
                audited += 1;

                let action = policies[viewer.index()]
                    .choose_action(&observation)
                    .expect("random policy finds an action");
                game.apply_observed_action(&observation, action)
                    .expect("sampled action is legal");
            }
        }
    }
    assert!(audited >= 1_000, "audit sampled only {audited} decisions");
}

fn true_hidden_hypothesis(game: &Game, viewer: PlayerId) -> Value {
    let definitions = |cards: &[CardInstance]| {
        cards
            .iter()
            .map(|card| card.definition.0)
            .collect::<Vec<_>>()
    };
    let opponent = viewer.opponent();
    let opponent_hand = &game.players[opponent.index()].hand;
    let drawn_indices = game.drawn_this_turn[opponent.index()]
        .iter()
        .filter_map(|id| opponent_hand.iter().position(|card| card.id == *id))
        .collect::<Vec<_>>();
    let miracle_window = game.miracle_window.and_then(|id| {
        opponent_hand
            .iter()
            .position(|card| card.id == id)
            .map(|hand_index| {
                json!({
                    "seat": seat_label(opponent),
                    "handIndex": hand_index,
                })
            })
    });
    let mut discard_choices = serde_json::Map::new();
    if let Some(DecisionContinuation::DiscardForEffect { chosen, .. }) = game
        .pending_decisions
        .first()
        .map(|pending| &pending.continuation)
    {
        for (player, cards) in chosen {
            if *player == viewer {
                continue;
            }
            let hand = &game.players[player.index()].hand;
            let indices = cards
                .iter()
                .filter_map(|id| hand.iter().position(|card| card.id == *id))
                .collect::<Vec<_>>();
            discard_choices.insert(seat_label(*player).into(), json!(indices));
        }
    }
    json!({
        "hands": {
            (seat_label(opponent)): definitions(opponent_hand),
        },
        "libraries": {
            "p1": definitions(&game.players[PlayerId::One.index()].library),
            "p2": definitions(&game.players[PlayerId::Two.index()].library),
        },
        "drawnThisTurn": {
            (seat_label(opponent)): drawn_indices,
        },
        "miracleWindow": miracle_window,
        "decision": {
            "discardChoices": discard_choices,
        },
    })
}
