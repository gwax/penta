use super::*;
use crate::ManaColor;
use crate::card::KeywordAbility;
use crate::game::DecisionContinuation;

#[test]
fn catalog_semantics_rehydrate_an_animation_without_a_card_name_switch() {
    let catalog = crate::poc::catalog().expect("catalog builds");
    let key = animation_json(&crate::card::abilities::MISHRAS_FACTORY_ANIMATION);
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
    let locator = ability_locator_json(&catalog, |candidate| *candidate == top_level)
        .expect("top-level ability has a locator");
    assert_eq!(catalog_ability(&catalog, &locator), Some(top_level));

    let granted_text =
        "At the beginning of your upkeep, sacrifice this artifact unless you pay {2}.";
    let locator = ability_locator_json(&catalog, |candidate| candidate.text == granted_text)
        .expect("nested granted ability has a locator");
    let rebuilt = catalog_ability(&catalog, &locator).expect("nested locator resolves");
    assert_eq!(rebuilt.text, granted_text);
    assert!(
        !locator["nested"]
            .as_array()
            .expect("nested path")
            .is_empty(),
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
        assert_eq!(parse_keyword(&keyword_json(keyword)), Ok(keyword));
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
fn a_hidden_zone_decision_without_id_reconciliation_fails_closed() {
    let catalog = crate::poc::catalog().expect("catalog builds");
    let deck = crate::Deck {
        main: vec![crate::card::cards::MOUNTAIN; 60],
        sideboard: Vec::new(),
    };
    let mut game = Game::new(catalog, [deck.clone(), deck], 47).expect("game starts");
    let player = PlayerId::One;
    let card = game.players[player.index()].hand[0].id;
    game.queue_miracle_reveal(player, card);
    game.pending_decisions[0].continuation = DecisionContinuation::Tutor;

    let checkpoint = game.checkpoint_json(player);
    assert!(checkpoint["decisionState"].is_null());
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
