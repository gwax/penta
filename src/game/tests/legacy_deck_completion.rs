use super::*;

fn cast(game: &mut Game, definition: CardDefinitionId, target: Option<Target>) {
    game.players[0].hand = game.build_zone(PlayerId::One, &[definition]).unwrap();
    let held = game.players[0].hand[0].id;
    game.priority = PlayerId::One;
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|a| {
            matches!(a,
        Action::CastSpell { card, choices, .. } if *card == held && target.is_none_or(|t|
        choices.targets().iter().any(|s| s.targets().contains(&t))))
        })
        .expect("requested cast is legal");
    game.apply(PlayerId::One, action).unwrap();
}

#[test]
fn burning_wish_takes_only_an_outside_game_sorcery_and_exiles_itself() {
    let mut game = ready_game();
    game.players[0].outside_game = game
        .build_zone(
            PlayerId::One,
            &[cards::LIGHTNING_BOLT, cards::FOREST, cards::WRATH_OF_GOD],
        )
        .unwrap();
    game.players[0].exile = game.build_zone(PlayerId::One, &[cards::PONDER]).unwrap();
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 2);
    cast(&mut game, cards::BURNING_WISH, None);
    drain_pending(&mut game);
    assert_eq!(game.players[0].hand.len(), 1);
    assert_eq!(game.players[0].hand[0].definition, cards::WRATH_OF_GOD);
    assert_eq!(game.players[0].outside_game.len(), 2);
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|c| c.definition == cards::BURNING_WISH)
    );
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|c| c.definition == cards::PONDER)
    );
}

#[test]
fn unmask_can_target_its_controller_and_discards_a_nonland() {
    let mut game = ready_game();
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 4);
    cast(
        &mut game,
        cards::UNMASK,
        Some(Target::Player(PlayerId::One)),
    );
    game.players[0].hand = game
        .build_zone(PlayerId::One, &[cards::SWAMP, cards::GRIZZLY_BEARS])
        .unwrap();
    drain_pending(&mut game);
    assert_eq!(game.players[0].hand.len(), 1);
    assert_eq!(game.players[0].hand[0].definition, cards::SWAMP);
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|c| c.definition == cards::GRIZZLY_BEARS)
    );
}

#[test]
fn sand_scout_groups_land_arrivals_and_limits_the_trigger_per_turn() {
    let mut game = ready_game();
    game.put_onto_battlefield(PlayerId::One, cards::SAND_SCOUT)
        .unwrap();
    drain_pending(&mut game);
    let lands = [cards::FOREST, cards::ISLAND]
        .map(|c| game.put_onto_battlefield(PlayerId::One, c).unwrap());
    game.move_permanents_to_zone(&lands, ZoneKind::Graveyard, ZonePlacement::Top);
    drain_pending(&mut game);
    let token_count = |game: &Game| {
        game.battlefield
            .iter()
            .filter(|p| p.card.definition.is_token())
            .count()
    };
    assert_eq!(token_count(&game), 1);
    let land = game
        .put_onto_battlefield(PlayerId::One, cards::MOUNTAIN)
        .unwrap();
    game.move_permanents_to_zone(&[land], ZoneKind::Graveyard, ZonePlacement::Top);
    drain_pending(&mut game);
    assert_eq!(token_count(&game), 1);
}

#[test]
fn peer_into_the_abyss_rounds_up_and_reads_the_targeted_players_life() {
    for target in [PlayerId::One, PlayerId::Two] {
        let mut game = ready_game();
        game.players[target.index()].library =
            game.build_zone(target, &[cards::FOREST; 5]).unwrap();
        game.players[target.index()].life = 17;
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 7);
        cast(
            &mut game,
            cards::PEER_INTO_THE_ABYSS,
            Some(Target::Player(target)),
        );
        let before = game.players[target.index()].hand.len();
        drain_pending(&mut game);
        assert_eq!(game.players[target.index()].library.len(), 2);
        assert_eq!(game.players[target.index()].hand.len(), before + 3);
        assert_eq!(game.players[target.index()].life, 8);
    }
}

#[test]
fn gaddock_teeg_prohibits_noncreature_x_even_at_zero_but_allows_creatures() {
    let mut game = ready_game();
    game.put_onto_battlefield(PlayerId::One, cards::GADDOCK_TEEG)
        .unwrap();
    game.players[0].hand = game
        .build_zone(
            PlayerId::One,
            &[
                cards::FIREBALL,
                cards::WRATH_OF_GOD,
                cards::SERRA_ANGEL,
                cards::LIGHTNING_BOLT,
            ],
        )
        .unwrap();
    for color in [ManaColor::Red, ManaColor::White] {
        game.add_unrestricted_mana(PlayerId::One, color, 10);
    }
    let actions = game.legal_actions(PlayerId::One);
    for (index, allowed) in [(0, false), (1, false), (2, true), (3, true)] {
        let held = game.players[0].hand[index].id;
        assert_eq!(
            actions
                .iter()
                .any(|a| matches!(a, Action::CastSpell {card, ..} if *card == held)),
            allowed
        );
    }
}

#[test]
fn earthbend_returns_the_exact_land_even_after_its_abilities_and_creator_are_gone() {
    let mut game = ready_game();
    let land = game
        .put_onto_battlefield(PlayerId::One, cards::FOREST)
        .unwrap();
    let cub = game
        .put_onto_battlefield(PlayerId::One, cards::BADGERMOLE_CUB)
        .unwrap();
    drain_pending(&mut game);
    let permanent = game.battlefield.iter().find(|p| p.card.id == land).unwrap();
    assert_eq!(game.power(permanent), Some(1));
    assert!(game.permanent_has_executable_keyword(permanent, KeywordAbility::Haste));
    game.move_permanents_to_zone(&[cub], ZoneKind::Exile, ZonePlacement::Top);
    game.put_onto_battlefield(PlayerId::Two, cards::HUMILITY)
        .unwrap();
    game.move_permanents_to_zone(&[land], ZoneKind::Exile, ZonePlacement::Top);
    drain_pending(&mut game);
    let returned = game
        .battlefield
        .iter()
        .find(|p| p.card.definition == cards::FOREST)
        .unwrap();
    assert_ne!(returned.card.id, land);
    assert!(returned.tapped);
    assert_eq!(returned.counters(CounterKind::PlusOnePlusOne), 0);
    assert!(!game.permanent_types(returned).unwrap().is_creature());
}

#[test]
fn summon_bahamut_crosses_both_first_chapters_in_one_counter_placement() {
    let mut game = ready_game();
    let saga = game
        .put_onto_battlefield(PlayerId::One, cards::SUMMON_BAHAMUT)
        .unwrap();
    // Isolate the simultaneous placement from the entry chapter, whose optional
    // destruction target may otherwise include the Saga itself.
    game.pending_triggers.clear();
    game.battlefield
        .iter_mut()
        .find(|p| p.card.id == saga)
        .unwrap()
        .set_counters(CounterKind::Lore, 2);
    game.capture_counters_placed(&[saga], CounterKind::Lore, 2);
    assert_eq!(
        game.pending_triggers
            .iter()
            .filter(|t| t.source.object == saga)
            .count(),
        2
    );
}

#[test]
fn lattice_allows_any_color_but_does_not_pay_colorless_symbols() {
    let mut game = ready_game();
    game.put_onto_battlefield(PlayerId::One, cards::MYCOSYNTH_LATTICE)
        .unwrap();
    let bear = put_ready(&mut game, cards::GRIZZLY_BEARS);
    let permanent = game.battlefield.iter().find(|p| p.card.id == bear).unwrap();
    assert!(
        game.permanent_types(permanent)
            .unwrap()
            .contains(CardType::Artifact)
    );
    assert_eq!(game.object_colors(bear), [false; 5]);
    game.players[0].graveyard = game
        .build_zone(PlayerId::One, &[cards::GRIZZLY_BEARS])
        .unwrap();
    assert_eq!(
        game.object_colors(game.players[0].graveyard[0].id),
        [false; 5]
    );
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    cast(
        &mut game,
        cards::LIGHTNING_BOLT,
        Some(Target::Player(PlayerId::Two)),
    );
    assert_eq!(
        game.object_colors(game.stack.last().unwrap().id),
        [false; 5]
    );
    drain_pending(&mut game);
    assert_eq!(game.players[1].life, 17);
    game.players[0].hand = game
        .build_zone(PlayerId::One, &[cards::THOUGHT_KNOT_SEER])
        .unwrap();
    assert_eq!(game.object_colors(game.players[0].hand[0].id), [false; 5]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 4);
    game.priority = PlayerId::One;
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|a| matches!(a, Action::CastSpell { .. }))
    );
}

mod effects;
mod payments;

fn activate_mana(game: &mut Game, source: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|a| matches!(a, Action::ActivateManaAbility { source: id, .. } if *id == source))
        .expect("mana activation is available");
    game.apply(PlayerId::One, action).unwrap();
}
fn activate(game: &mut Game, source: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|a| matches!(a, Action::ActivateAbility { source: id, .. } if *id == source))
        .expect("activation is available");
    game.apply(PlayerId::One, action).unwrap();
}
fn put_ready(game: &mut Game, definition: CardDefinitionId) -> GameObjectId {
    let id = game
        .put_onto_battlefield(PlayerId::One, definition)
        .unwrap();
    game.battlefield
        .iter_mut()
        .find(|p| p.card.id == id)
        .unwrap()
        .entered_controller_turn = 0;
    id
}
fn stop_at_decision(game: &mut Game) {
    for _ in 0..20 {
        if !game.pending_decisions.is_empty() {
            return;
        }
        assert!(
            !game.stack.is_empty() || !game.pending_triggers.is_empty(),
            "expected a decision"
        );
        game.apply(game.priority, Action::PassPriority).unwrap();
    }
    panic!("decision did not arrive");
}
fn choose_label(game: &mut Game, label: &str) {
    let label = match label {
        "Yes" => "Do it",
        "No" => "Decline",
        other => other,
    };
    let decision = game.pending_decisions[0].observation.clone();
    let option = decision
        .options
        .iter()
        .find(|o| o.label.contains(label))
        .unwrap_or_else(|| panic!("missing {label} in {:?}", decision.options));
    game.apply(
        decision.player,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![option.id],
        },
    )
    .unwrap();
}
