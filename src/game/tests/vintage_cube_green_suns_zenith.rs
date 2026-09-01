//! Green Sun's Zenith: a green creature of your choosing, and the card goes
//! back into the deck rather than to a graveyard.

use super::*;

/// Player One holding the Zenith, with `library` behind it and `mana` green.
fn staged(library: &[CardDefinitionId], mana: u16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for (index, definition) in library.iter().enumerate() {
        let id = 278_000 + u32::try_from(index).expect("a short list");
        game.players[0]
            .library
            .push(card(id, *definition, PlayerId::One));
    }
    let zenith = game
        .build_zone(PlayerId::One, &[cards::GREEN_SUN_S_ZENITH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let zenith_id = zenith.id;
    game.players[0].hand.push(zenith);
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, mana);
    (game, zenith_id)
}

/// Answers every question, taking the first offer where one is required.
fn settle(game: &mut Game) {
    for _ in 0..32 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .map(|option| option.id)
                .take(decision.maximum.min(1))
                .collect();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the offered choice is legal");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

/// Casts it for `x`, and returns the names the search offered.
fn cast_for(game: &mut Game, zenith: GameObjectId, x: u16) -> Vec<String> {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == zenith && choices.x() == x)
        })
        .unwrap_or_else(|| panic!("it is castable for {x}"));
    game.apply(PlayerId::One, action).expect("it is cast");
    for _ in 0..8 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    let offered = game
        .pending_decisions
        .first()
        .map(|pending| {
            pending
                .observation
                .options
                .iter()
                .map(|option| option.label.clone())
                .collect()
        })
        .unwrap_or_default();
    settle(game);
    offered
}

/// X bounds what the search may find: two is enough for the Bears and not
/// for the Wurm.
#[test]
fn x_bounds_what_it_finds() {
    let (mut game, zenith) = staged(&[cards::GRIZZLY_BEARS, cards::CRAW_WURM], 3);

    let offered = cast_for(&mut game, zenith, 2);

    assert_eq!(
        offered,
        vec!["Grizzly Bears".to_owned()],
        "a six-drop is not a two-drop",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == ObjectKind::Card(cards::GRIZZLY_BEARS)),
        "and what it found is on the battlefield",
    );
}

/// A larger X reaches further.
#[test]
fn a_larger_x_reaches_the_wurm() {
    let (mut game, zenith) = staged(&[cards::GRIZZLY_BEARS, cards::CRAW_WURM], 7);

    let offered = cast_for(&mut game, zenith, 6);

    assert_eq!(offered.len(), 2, "both are within six");
}

/// Only green creatures, whatever the mana value.
#[test]
fn it_finds_only_green_creatures() {
    let (mut game, zenith) = staged(
        &[cards::SERRA_ANGEL, cards::LIGHTNING_BOLT, cards::FOREST],
        7,
    );

    let offered = cast_for(&mut game, zenith, 6);

    assert!(
        offered.is_empty(),
        "a white creature, a red instant and a land are none of them",
    );
}

/// The Zenith itself is shuffled into the library rather than binned.
#[test]
fn the_zenith_goes_back_into_the_library() {
    let (mut game, zenith) = staged(&[cards::GRIZZLY_BEARS], 3);

    cast_for(&mut game, zenith, 2);

    assert!(game.players[0].graveyard.is_empty(), "not a graveyard card");
    assert_eq!(
        game.players[0]
            .library
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::GREEN_SUN_S_ZENITH],
        "it is back in the deck",
    );
}

/// X of zero still finds the creatures that cost nothing -- and with none in
/// the library, the spell simply shuffles itself back.
#[test]
fn zero_finds_nothing_in_a_library_of_two_drops() {
    let (mut game, zenith) = staged(&[cards::GRIZZLY_BEARS], 1);

    let offered = cast_for(&mut game, zenith, 0);

    assert!(offered.is_empty(), "nothing costs nothing here");
    assert!(game.battlefield.is_empty());
    // The library is shuffled, so what it holds is the claim rather than
    // the order it holds them in.
    let library = &game.players[0].library;
    assert_eq!(library.len(), 2);
    for wanted in [cards::GRIZZLY_BEARS, cards::GREEN_SUN_S_ZENITH] {
        assert!(
            library.iter().any(|card| card.definition == wanted),
            "the Bears stayed and the Zenith joined them",
        );
    }
}

/// "If Green Sun's Zenith is countered, none of its effects will happen.
/// Notably, it will be put into its owner's graveyard rather than shuffled
/// into its owner's library." The shuffle-back is part of the resolution,
/// not part of the card.
#[test]
fn a_countered_zenith_goes_to_the_graveyard() {
    let (mut game, zenith) = staged(&[cards::GRIZZLY_BEARS], 3);
    let counterspell = card(278_500, cards::COUNTERSPELL, PlayerId::Two);
    let counterspell_id = counterspell.id;
    game.players[1].hand.push(counterspell);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == zenith && choices.x() == 2)
        })
        .expect("it is castable for two");
    game.apply(PlayerId::One, cast).expect("it is cast");
    let on_stack = game.stack.last().expect("the Zenith is on the stack").id;
    game.apply(PlayerId::One, Action::PassPriority)
        .expect("they get a word in");
    game.apply(
        PlayerId::Two,
        cast_action(
            counterspell_id,
            vec![Target::Spell(on_stack)],
            Vec::new(),
            0,
        ),
    )
    .expect("two blue answers it");
    settle(&mut game);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GREEN_SUN_S_ZENITH),
        "a countered spell goes to its owner's graveyard",
    );
    assert_eq!(
        game.players[0]
            .library
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::GRIZZLY_BEARS],
        "and the search never happened, so the Bears are still in there",
    );
    assert!(
        game.battlefield.is_empty(),
        "nothing was put onto the battlefield",
    );
}

/// The line the card is famous for: X of nothing finds a Dryad Arbor, whose
/// type line says Land Creature — Forest Dryad and whose mana value is zero.
/// It is put onto the battlefield rather than played, so the land drop is
/// still there afterwards.
#[test]
fn zero_finds_a_dryad_arbor_and_spends_no_land_drop() {
    let (mut game, zenith) = staged(&[cards::DRYAD_ARBOR, cards::GRIZZLY_BEARS], 1);
    game.players[PlayerId::One.index()].lands_played_this_turn = 0;

    let offered = cast_for(&mut game, zenith, 0);
    assert_eq!(
        offered,
        vec!["Dryad Arbor".to_owned()],
        "a green creature card of mana value zero, and the only one",
    );

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::DRYAD_ARBOR),
        "it arrived on the battlefield",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].lands_played_this_turn,
        0,
        "putting a land onto the battlefield is not playing one",
    );
}

/// "Search your library": a Wurm in hand and one in the graveyard are not in
/// it, so a library with nothing green leaves the Zenith with nothing to
/// find -- and it still shuffles itself back.
#[test]
fn it_searches_the_library_and_nowhere_else() {
    let (mut game, zenith) = staged(&[cards::ISLAND, cards::LIGHTNING_BOLT], 7);
    game.players[PlayerId::One.index()].hand.push(card(
        278_500,
        cards::WORLDSPINE_WURM,
        PlayerId::One,
    ));
    game.players[PlayerId::One.index()].graveyard.push(card(
        278_501,
        cards::GRIZZLY_BEARS,
        PlayerId::One,
    ));

    let offered = cast_for(&mut game, zenith, 6);
    assert!(
        offered.is_empty() || !offered.iter().any(|name| name == "Worldspine Wurm"),
        "neither the hand nor the graveyard is searched: {offered:?}",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::WORLDSPINE_WURM),
        "so nothing arrived",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .library
            .iter()
            .any(|card| card.definition == cards::GREEN_SUN_S_ZENITH),
        "and the Zenith shuffled itself back all the same",
    );
}
