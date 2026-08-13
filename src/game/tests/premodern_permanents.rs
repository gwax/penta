use super::*;

#[test]
fn claws_of_gix_sacrifices_the_chosen_permanent_before_gaining_life() {
    let mut game = ready_game();
    let claws = creature(10_000, cards::CLAWS_OF_GIX, PlayerId::One);
    let source = claws.card.id;
    let fodder = creature(10_001, cards::MOUNTAIN, PlayerId::One);
    let fodder_id = fodder.card.id;
    game.battlefield.extend([claws, fodder]);
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    game.apply(
        PlayerId::One,
        Action::ActivateAbility {
            source,
            ability: activated_ability_for(&game, source, 0),
            targets: Vec::new(),
            cost_object: Some(fodder_id),
            x: 0,
        },
    )
    .unwrap();

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != fodder_id),
        "the permanent is sacrificed as a cost"
    );
    assert_eq!(game.players[PlayerId::One.index()].life, 20);
    pass_priority_pair(&mut game);
    assert_eq!(game.players[PlayerId::One.index()].life, 21);
}

#[test]
fn warmth_triggers_only_for_an_opponents_red_spell() {
    let mut game = ready_game();
    game.battlefield
        .push(creature(10_000, cards::WARMTH, PlayerId::One));
    let incinerate = card(10_001, cards::INCINERATE, PlayerId::Two);
    game.players[PlayerId::Two.index()]
        .hand
        .push(incinerate.clone());
    game.players[PlayerId::Two.index()].mana_pool.colorless = 1;
    game.players[PlayerId::Two.index()].mana_pool.red = 1;
    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(
            incinerate.id,
            vec![Target::Player(PlayerId::One)],
            Vec::new(),
            0,
        ),
    )
    .unwrap();
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        19,
        "Warmth gained two before Incinerate dealt three"
    );
}

#[test]
fn root_maze_makes_future_artifacts_and_lands_enter_tapped() {
    let mut game = ready_game();
    let root = game
        .put_onto_battlefield(PlayerId::One, cards::ROOT_MAZE)
        .expect("cataloged");
    let artifact = game
        .put_onto_battlefield(PlayerId::Two, cards::DARKSTEEL_INGOT)
        .expect("cataloged");
    let land = game
        .put_onto_battlefield(PlayerId::Two, cards::ISLAND)
        .expect("cataloged");
    let creature = game
        .put_onto_battlefield(PlayerId::Two, cards::SAVANNAH_LIONS)
        .expect("cataloged");

    for object in [artifact, land] {
        assert!(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == object)
                .expect("the permanent entered")
                .tapped
        );
    }
    assert!(
        !game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == creature)
            .expect("the creature entered")
            .tapped
    );
    assert!(
        !game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == root)
            .expect("Root Maze was already entering before its effect existed")
            .tapped
    );
}

#[test]
fn phyrexian_arena_draws_then_costs_one_life_each_upkeep() {
    let mut game = ready_game();
    game.battlefield
        .push(creature(10_000, cards::PHYREXIAN_ARENA, PlayerId::One));
    game.turn = 2;
    game.step = Step::Upkeep;
    game.handle_upkeep_triggers();
    drain_pending(&mut game);

    assert_eq!(game.players[PlayerId::One.index()].hand.len(), 1);
    assert_eq!(game.players[PlayerId::One.index()].life, 19);
}

#[test]
fn tranquil_domain_spares_auras_and_upheaval_returns_every_permanent() {
    let mut game = ready_game();
    let host = creature(10_000, cards::SAVANNAH_LIONS, PlayerId::One);
    let host_id = host.card.id;
    let mut aura = creature(10_001, cards::VOLCANIC_STRENGTH, PlayerId::One);
    aura.attached_to = Some(host_id);
    let aura_id = aura.card.id;
    let maze = creature(10_002, cards::ROOT_MAZE, PlayerId::Two);
    let maze_id = maze.card.id;
    game.battlefield.extend([host, aura, maze]);
    let domain = card(10_003, cards::TRANQUIL_DOMAIN, PlayerId::One);
    game.players[PlayerId::One.index()]
        .hand
        .push(domain.clone());
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    game.players[PlayerId::One.index()].mana_pool.green = 1;
    game.apply(
        PlayerId::One,
        cast_action(domain.id, Vec::new(), Vec::new(), 0),
    )
    .unwrap();
    pass_priority_pair(&mut game);
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != maze_id)
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == aura_id)
    );

    let opposing_land = creature(10_004, cards::MOUNTAIN, PlayerId::Two);
    game.battlefield.push(opposing_land);
    let upheaval = card(10_005, cards::UPHEAVAL, PlayerId::One);
    game.players[PlayerId::One.index()]
        .hand
        .push(upheaval.clone());
    game.players[PlayerId::One.index()].mana_pool.colorless = 4;
    game.players[PlayerId::One.index()].mana_pool.blue = 2;
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(upheaval.id, Vec::new(), Vec::new(), 0),
    )
    .unwrap();
    pass_priority_pair(&mut game);

    assert!(game.battlefield.is_empty());
    assert!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .any(|card| card.definition == cards::SAVANNAH_LIONS)
    );
    assert!(
        game.players[PlayerId::Two.index()]
            .hand
            .iter()
            .any(|card| card.definition == cards::MOUNTAIN)
    );
}
