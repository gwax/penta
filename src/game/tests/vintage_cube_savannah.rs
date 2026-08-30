//! Savannah: a Forest Plains that is not a basic land. The types are what a
//! fetchland and a mana ability read; the missing supertype is what a
//! Wasteland and a basic-land search read.

use super::*;

/// Player One with a Savannah out and a Forest beside it for contrast.
fn staged() -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let savannah = game
        .put_onto_battlefield(PlayerId::One, cards::SAVANNAH)
        .expect("cataloged");
    let forest = game
        .put_onto_battlefield(PlayerId::One, cards::FOREST)
        .expect("cataloged");
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.empty_mana_pools();
    (game, savannah, forest)
}

fn colors_of(game: &Game, id: GameObjectId) -> Vec<ManaColor> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateManaAbility { source, color, .. } if source == id => Some(color),
            _ => None,
        })
        .collect()
}

/// "This has the mana abilities associated with both of its basic land
/// types."
#[test]
fn it_taps_for_both_of_its_types() {
    let (game, savannah, _) = staged();

    assert_eq!(
        colors_of(&game, savannah),
        vec![ManaColor::Green, ManaColor::White],
        "green and white, in printed subtype order",
    );
}

/// "Things that affect basic lands don't affect it." A Wasteland answers
/// nonbasic lands, and having Forest and Plains printed on it does not make
/// the Savannah basic.
#[test]
fn a_wasteland_answers_it_and_not_the_forest() {
    let (mut game, savannah, forest) = staged();
    let wasteland = game
        .put_onto_battlefield(PlayerId::Two, cards::WASTELAND)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::Two;

    let named = |game: &Game, land: GameObjectId| {
        game.legal_actions(PlayerId::Two).into_iter().any(|action| {
            matches!(
                &action,
                Action::ActivateAbility { source, targets, .. }
                    if *source == wasteland
                        && targets
                            .iter()
                            .any(|selection| selection.targets() == [Target::Permanent(land)])
            )
        })
    };
    assert!(named(&game, savannah), "a nonbasic land, types and all");
    assert!(!named(&game, forest), "and a Forest is not one");

    let destroy = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                Action::ActivateAbility { source, targets, .. }
                    if *source == wasteland
                        && targets
                            .iter()
                            .any(|selection| selection.targets() == [Target::Permanent(savannah)])
            )
        })
        .expect("it may be named");
    game.apply(PlayerId::Two, destroy).expect("it activates");
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == savannah),
        "and destroyed",
    );
}

/// The other half of the same ruling: a search that names basic land cards
/// passes it over, however many basic land types it has.
#[test]
fn a_basic_land_search_will_not_find_it() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for (index, definition) in [cards::SAVANNAH, cards::FOREST].into_iter().enumerate() {
        game.players[0].library.push(card(
            98_000 + u32::try_from(index).expect("two cards"),
            definition,
            PlayerId::One,
        ));
    }
    let growth = game
        .build_zone(PlayerId::One, &[cards::RAMPANT_GROWTH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let growth_id = growth.id;
    game.players[0].hand.push(growth);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == growth_id))
        .expect("two mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);

    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the search asks");
    let offered = decision
        .options
        .iter()
        .filter_map(|option| match option.card {
            Some((_, ObjectCharacteristics::Card { definition, .. })) => Some(definition),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        offered,
        vec![cards::FOREST],
        "the Forest is a basic land card and the Savannah is not",
    );
}

/// The other nine of the cycle, each with the pair its basic land types
/// make: one card printed ten ways, and what is worth checking per member
/// is which two colours it answers for.
const ORIGINAL_DUALS: [(CardDefinitionId, [ManaColor; 2]); 10] = [
    (cards::BADLANDS, [ManaColor::Black, ManaColor::Red]),
    (cards::BAYOU, [ManaColor::Black, ManaColor::Green]),
    (cards::PLATEAU, [ManaColor::Red, ManaColor::White]),
    (cards::SAVANNAH, [ManaColor::Green, ManaColor::White]),
    (cards::SCRUBLAND, [ManaColor::White, ManaColor::Black]),
    (cards::TAIGA, [ManaColor::Red, ManaColor::Green]),
    (cards::TROPICAL_ISLAND, [ManaColor::Green, ManaColor::Blue]),
    (cards::TUNDRA, [ManaColor::White, ManaColor::Blue]),
    (cards::UNDERGROUND_SEA, [ManaColor::Blue, ManaColor::Black]),
    (cards::VOLCANIC_ISLAND, [ManaColor::Blue, ManaColor::Red]),
];

/// "This has the mana abilities associated with both of its basic land
/// types" -- and only those two, for every member of the cycle.
#[test]
fn every_original_dual_taps_for_its_own_two() {
    for (definition, colors) in ORIGINAL_DUALS {
        let mut game = ready_game();
        game.battlefield.clear();
        let land = game
            .put_onto_battlefield(PlayerId::One, definition)
            .expect("cataloged");
        drain_pending(&mut game);
        game.active_player = PlayerId::One;
        game.step = Step::PrecombatMain;
        game.priority = PlayerId::One;

        let mut offered = colors_of(&game, land);
        offered.sort_unstable();
        let mut expected = colors.to_vec();
        expected.sort_unstable();
        assert_eq!(offered, expected, "{definition:?} makes its own two");
        assert!(
            !game
                .battlefield
                .iter()
                .find(|permanent| permanent.card.id == land)
                .expect("it entered")
                .tapped,
            "{definition:?} asks nothing on the way in",
        );
    }
}

/// "The mana abilities associated with both of its basic land types" are two
/// abilities on one land, and they share its tap: a Scrubland makes one mana
/// a turn, of whichever colour you asked for, and then offers nothing.
#[test]
fn tapping_a_dual_for_one_colour_spends_the_other() {
    let mut game = ready_game();
    game.battlefield.clear();
    let land = game
        .put_onto_battlefield(PlayerId::One, cards::SCRUBLAND)
        .expect("cataloged");
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    assert_eq!(colors_of(&game, land).len(), 2, "white and black on offer");

    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: land,
            ability: mana_ability_for(&game, land, ManaColor::White),
            color: ManaColor::White,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("it taps for white");

    assert_eq!(game.players[0].mana_pool.white, 1);
    assert_eq!(
        game.players[0].mana_pool.black, 0,
        "the other half was never paid for",
    );
    assert!(
        colors_of(&game, land).is_empty(),
        "and a tapped land offers neither colour",
    );
}

/// "Land type changing effects that change a dual land's land type will
/// remove the old land types completely." A Blood Moon makes every nonbasic
/// land a Mountain, and a Badlands keeps neither of its own types.
#[test]
fn a_blood_moon_takes_both_of_a_duals_types() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let badlands = game
        .put_onto_battlefield(PlayerId::One, cards::BADLANDS)
        .expect("cataloged");
    game.battlefield
        .push(creature(64_000, cards::BLOOD_MOON, PlayerId::One));
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.empty_mana_pools();

    let land = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == badlands)
        .expect("it is on the battlefield");
    assert_eq!(
        game.effective_subtypes(land).as_ref(),
        &["Mountain"],
        "the Swamp went with the rest of what it was",
    );
    assert_eq!(
        colors_of(&game, badlands),
        vec![ManaColor::Red],
        "so it makes red and nothing else",
    );
}

/// "Text-changing effects that just change one of the two land types will
/// leave the other type unaffected." A Magical Hack turning its Swamp into
/// an Island leaves the Mountain where it was.
#[test]
fn magical_hack_changes_one_of_a_duals_types_and_leaves_the_other() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let badlands = game
        .put_onto_battlefield(PlayerId::One, cards::BADLANDS)
        .expect("cataloged");
    drain_pending(&mut game);
    let hack = card(64_100, cards::MAGICAL_HACK, PlayerId::One);
    let hack_id = hack.id;
    game.players[0].hand.push(hack);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.empty_mana_pools();
    game.players[0].mana_pool.blue = 1;

    game.apply(
        PlayerId::One,
        cast_action(hack_id, vec![Target::Permanent(badlands)], Vec::new(), 0),
    )
    .expect("a Badlands has land types to rewrite");
    pass_priority_pair(&mut game);
    choose_decision_by_label(&mut game, PlayerId::One, "Swamp → Island");

    let land = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == badlands)
        .expect("it is on the battlefield");
    assert_eq!(
        game.effective_subtypes(land).as_ref(),
        &["Island", "Mountain"],
        "one word was rewritten and the other was not",
    );
    let mut colors = colors_of(&game, badlands);
    colors.sort_by_key(|color| format!("{color:?}"));
    assert_eq!(
        colors,
        vec![ManaColor::Blue, ManaColor::Red],
        "and the black went with the Swamp",
    );
}
