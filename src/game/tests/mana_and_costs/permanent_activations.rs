// Activations paid for out of the board rather than out of the pool.
//
// Split from the cost tests next door for the source-size budget, and split
// here because these ask what a permanent's own ability may spend and when it
// may spend it -- its own mana, its own tap, its own turn -- rather than how a
// spell's cost is planned. Included textually, so the imports here are that
// module's.

#[test]
fn mishras_factory_can_use_its_own_mana_to_animate() {
    let mut game = ready_game();
    let factory = creature(10_000, cards::MISHRA_S_FACTORY, PlayerId::One);
    let factory_id = factory.card.id;
    game.battlefield = vec![factory];
    let animate = Action::ActivateAbility {
        source: factory_id,
        ability: activated_ability_for(&game, factory_id, 0),
        targets: Vec::new(),
        cost_objects: Vec::new(),
        x: 0,
        modes: Vec::new(),
        mana_payment: None,
    };

    assert!(game.legal_actions(PlayerId::One).contains(&animate));
    game.apply(PlayerId::One, animate).unwrap();
    drain_pending(&mut game);

    let factory = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == factory_id)
        .unwrap();
    assert!(factory.tapped);
    assert_eq!(game.power(factory), Some(2));
    assert_eq!(game.players[0].mana_pool.total(), 0);

    let shatter = card(10_001, cards::SHATTER, PlayerId::Two);
    game.players[1].hand.push(shatter.clone());
    game.players[1].mana_pool.red = 2;
    game.priority = PlayerId::Two;
    assert!(game.legal_actions(PlayerId::Two).contains(&cast_action(
        shatter.id,
        vec![Target::Permanent(factory_id)],
        Vec::new(),
        0,
    )));
}

#[test]
fn an_animated_untapped_mishras_factory_can_block() {
    let mut game = ready_game();
    let mut attacker = creature(10_000, cards::GOBLINS_OF_THE_FLARG, PlayerId::One);
    attacker.attacking = true;
    let attacker_id = attacker.card.id;
    let factory = creature(10_001, cards::MISHRA_S_FACTORY, PlayerId::Two);
    let factory_id = factory.card.id;
    game.battlefield = vec![attacker, factory];
    attach_constant_resolved_characteristics(
        &mut game,
        factory_id,
        &TEST_MISHRAS_FACTORY_CHARACTERISTICS,
        ContinuousEffectExpiration::EndOfTurn,
    );
    game.active_player = PlayerId::One;
    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;

    assert!(
        game.legal_actions(PlayerId::Two)
            .contains(&Action::DeclareBlocker {
                blocker: factory_id,
                attacker: attacker_id,
            })
    );
}

#[test]
fn strip_mine_can_be_activated_in_response_to_strip_mine() {
    let mut game = ready_game();
    let first_strip = creature(10_000, cards::STRIP_MINE, PlayerId::One);
    let second_strip = creature(10_001, cards::STRIP_MINE, PlayerId::Two);
    let other_land = creature(10_002, cards::MOUNTAIN, PlayerId::Two);
    let first_strip_id = first_strip.card.id;
    let second_strip_id = second_strip.card.id;
    let other_land_id = other_land.card.id;
    game.battlefield = vec![first_strip, second_strip, other_land];
    game.priority = PlayerId::Two;

    game.apply(
        PlayerId::Two,
        Action::ActivateAbility {
            source: second_strip_id,
            ability: activated_ability_for(&game, second_strip_id, 0),
            targets: activated_targets(Target::Permanent(first_strip_id)),
            cost_objects: Vec::new(),
            x: 0,
            modes: Vec::new(),
            mana_payment: None,
        },
    )
    .unwrap();
    game.apply(PlayerId::Two, Action::PassPriority).unwrap();

    let response = Action::ActivateAbility {
        source: first_strip_id,
        ability: activated_ability_for(&game, first_strip_id, 0),
        targets: activated_targets(Target::Permanent(other_land_id)),
        cost_objects: Vec::new(),
        x: 0,
        modes: Vec::new(),
        mana_payment: None,
    };
    assert!(game.legal_actions(PlayerId::One).contains(&response));
    game.apply(PlayerId::One, response).unwrap();
    assert_eq!(game.stack.len(), 2);

    pass_priority_pair(&mut game);
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != other_land_id)
    );
    assert_eq!(game.stack.len(), 1);

    pass_priority_pair(&mut game);
    assert!(game.stack.is_empty());
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| ![first_strip_id, second_strip_id].contains(&permanent.card.id))
    );
}

#[test]
fn icatian_javelineers_cannot_activate_until_their_controller_turn() {
    let mut game = ready_game();
    let mut javeliners = creature(10_000, cards::ICATIAN_JAVELINEERS, PlayerId::One);
    javeliners.counters.set(CounterKind::named("javelin"), 1);
    javeliners.entered_controller_turn = game.turns_started[PlayerId::One.index()];
    let source = javeliners.card.id;
    game.battlefield = vec![javeliners];
    let action = Action::ActivateAbility {
        source,
        ability: activated_ability_for(&game, source, 0),
        targets: activated_targets(Target::Player(PlayerId::Two)),
        cost_objects: Vec::new(),
        x: 0,
        modes: Vec::new(),
        mana_payment: None,
    };
    assert_eq!(game.power(&game.battlefield[0]), Some(1));
    assert_eq!(game.toughness(&game.battlefield[0]), Some(1));

    assert!(!game.legal_actions(PlayerId::One).contains(&action));

    game.start_next_turn();
    game.priority = PlayerId::One;
    assert_eq!(game.active_player, PlayerId::Two);
    assert!(!game.legal_actions(PlayerId::One).contains(&action));

    game.start_next_turn();
    game.priority = PlayerId::One;
    assert_eq!(game.active_player, PlayerId::One);
    assert!(game.legal_actions(PlayerId::One).contains(&action));
}

#[test]
fn icatian_javelineers_counter_cost_preserves_white_source_targeting() {
    let mut game = ready_game();
    game.turns_started[PlayerId::One.index()] = 1;
    let mut javelineers = creature(10_000, cards::ICATIAN_JAVELINEERS, PlayerId::One);
    javelineers.counters.set(CounterKind::named("javelin"), 1);
    let source = javelineers.card.id;
    let knight = creature(10_001, cards::BLACK_KNIGHT, PlayerId::Two);
    let knight_id = knight.card.id;
    game.battlefield = vec![javelineers, knight];

    let protected_target = Action::ActivateAbility {
        source,
        ability: activated_ability_for(&game, source, 0),
        targets: activated_targets(Target::Permanent(knight_id)),
        cost_objects: Vec::new(),
        x: 0,
        modes: Vec::new(),
        mana_payment: None,
    };
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .contains(&protected_target),
        "protection from white sees the activated ability's white source",
    );

    let player_target = Action::ActivateAbility {
        source,
        ability: activated_ability_for(&game, source, 0),
        targets: activated_targets(Target::Player(PlayerId::Two)),
        cost_objects: Vec::new(),
        x: 0,
        modes: Vec::new(),
        mana_payment: None,
    };
    game.apply(PlayerId::One, player_target).unwrap();
    let javelineers = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == source)
        .expect("paying the counter cost leaves the source on the battlefield");
    assert!(javelineers.tapped);
    assert_eq!(javelineers.counters(CounterKind::named("javelin")), 0);
    assert!(!game.legal_actions(PlayerId::One).iter().any(
        |action| matches!(action, Action::ActivateAbility { source: candidate, .. } if *candidate == source)
    ));

    pass_priority_pair(&mut game);
    assert_eq!(game.players[PlayerId::Two.index()].life, 19);
}

#[test]
fn an_unplannable_payment_reports_what_the_planner_saw() {
    let mut game = ready_game();
    let mut mountain = creature(10_000, cards::MOUNTAIN, PlayerId::One);
    mountain.tapped = true;
    let mox = creature(10_001, cards::MOX_RUBY, PlayerId::One);
    game.battlefield.extend([mountain, mox]);

    let cost = ManaCost {
        generic: 5,
        ..ManaCost::default()
    };
    let report = game.unplannable_payment(PlayerId::One, cost, 0, None, &ManaPaymentPurpose::Other);

    assert!(
        report.starts_with("a legal payment has a complete mana activation plan"),
        "the invariant still names itself first: {report}"
    );
    assert!(report.contains("cost {5} with x 0"), "{report}");
    assert!(
        report.contains("affordable per the gate: false"),
        "the report says whether the gate and the plan actually disagree: {report}"
    );
    assert!(
        report.contains("Mox Ruby produces [Red]"),
        "an untapped source is listed with what it makes: {report}"
    );
    assert!(
        !report.contains("Mountain"),
        "a tapped land offers no activation, so it is not a candidate: {report}"
    );
}
