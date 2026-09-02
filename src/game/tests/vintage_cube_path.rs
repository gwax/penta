//! Path to Exile: removal that pays its victim, in either direction.

use super::*;

/// Resolves whatever is on the stack, answering nothing.
fn resolve(game: &mut Game) {
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

fn cast_path(game: &mut Game, path: CardInstanceId, target: GameObjectId) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == path
                    && choices
                        .targets()
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|chosen| *chosen == Target::Permanent(target))
            }
            _ => false,
        })
        .expect("one white mana casts it at that creature");
    game.apply(PlayerId::One, cast).expect("it is cast");
    resolve(game);
    drain_pending(game);
}

/// The ordinary use: their creature is exiled, and they are paid one basic
/// land, tapped, for the trouble.
#[test]
fn path_exiles_their_creature_and_pays_them_a_tapped_basic() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = creature(96_000, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    game.players[1].library.clear();
    game.players[1]
        .library
        .push(card(96_001, cards::FOREST, PlayerId::Two));
    let path = card(96_002, cards::PATH_TO_EXILE, PlayerId::One);
    let path_id = path.id;
    game.players[0].hand.push(path);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);

    cast_path(&mut game, path_id, bears_id);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "the creature is gone",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "exiled rather than destroyed",
    );
    let forest = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::FOREST)
        .expect("its controller found a basic land");
    assert_eq!(forest.controller, PlayerId::Two, "the land is theirs");
    assert!(forest.tapped, "and it arrives tapped");
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.controller != PlayerId::One),
        "and nothing came to the caster",
    );
}

/// Pointed at your own creature it is a Rampant Growth: the searcher is the
/// creature's controller, whoever that is.
#[test]
fn path_on_your_own_creature_ramps_you() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = creature(96_010, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(96_011, cards::PLAINS, PlayerId::One));
    let path = card(96_012, cards::PATH_TO_EXILE, PlayerId::One);
    let path_id = path.id;
    game.players[0].hand.push(path);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);

    cast_path(&mut game, path_id, bears_id);

    let plains = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::PLAINS)
        .expect("you searched your own library");
    assert_eq!(plains.controller, PlayerId::One);
    assert!(plains.tapped);
}

/// The search is a "may", and a library with no basic land answers it the
/// same way a decline does: the creature is still exiled.
#[test]
fn a_library_without_a_basic_still_loses_the_creature() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = creature(96_020, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    game.players[1].library.clear();
    game.players[1]
        .library
        .push(card(96_021, cards::GRIZZLY_BEARS, PlayerId::Two));
    let path = card(96_022, cards::PATH_TO_EXILE, PlayerId::One);
    let path_id = path.id;
    game.players[0].hand.push(path);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);

    cast_path(&mut game, path_id, bears_id);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "the exile is not contingent on the search",
    );
    assert_eq!(
        game.players[1].library.len(),
        1,
        "and nothing was taken from a library that held no basic land",
    );
}

/// "The controller of the exiled creature isn't required to search their
/// library for a basic land. If that player doesn't, the player won't
/// shuffle their library." The creature is exiled either way.
#[test]
fn their_search_may_be_declined() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = creature(96_100, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    game.players[1].library.clear();
    game.players[1]
        .library
        .push(card(96_101, cards::FOREST, PlayerId::Two));
    let path = card(96_102, cards::PATH_TO_EXILE, PlayerId::One);
    let path_id = path.id;
    game.players[0].hand.push(path);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == path_id
                    && choices
                        .targets()
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|chosen| *chosen == Target::Permanent(bears_id))
            }
            _ => false,
        })
        .expect("one white mana casts it at that creature");
    game.apply(PlayerId::One, cast).expect("it is cast");
    resolve(&mut game);

    let search = game
        .observe(PlayerId::Two)
        .decision
        .expect("its controller is the one asked");
    game.apply(
        PlayerId::Two,
        Action::ChooseDecision {
            decision: search.id,
            options: Vec::new(),
        },
    )
    .expect("taking nothing is allowed");
    drain_pending(&mut game);

    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "the creature is exiled whatever they do about the land",
    );
    assert_eq!(
        game.players[1]
            .library
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::FOREST],
        "and the Forest they declined is still in the library",
    );
    assert!(
        game.battlefield.is_empty(),
        "with nothing on the battlefield"
    );
}

/// "If the target creature is an illegal target by the time it tries to
/// resolve, it won't resolve and none of its effects will happen. The
/// creature's controller won't search for a basic land card."
#[test]
fn an_answered_creature_leaves_them_no_land_either() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = creature(96_200, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    game.players[1].library.clear();
    game.players[1]
        .library
        .push(card(96_201, cards::FOREST, PlayerId::Two));
    let path = card(96_202, cards::PATH_TO_EXILE, PlayerId::One);
    let path_id = path.id;
    game.players[0].hand.push(path);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == path_id
                    && choices
                        .targets()
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|chosen| *chosen == Target::Permanent(bears_id))
            }
            _ => false,
        })
        .expect("one white mana casts it at that creature");
    game.apply(PlayerId::One, cast).expect("it is cast");

    // The creature it named dies before it resolves.
    game.move_permanents_to_graveyard(&[bears_id]);
    resolve(&mut game);
    drain_pending(&mut game);

    assert!(
        game.players[1].exile.is_empty(),
        "nothing was exiled: the spell never resolved",
    );
    assert_eq!(
        game.players[1]
            .library
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::FOREST],
        "and they were never asked about a land",
    );
    assert!(game.battlefield.is_empty());
}

/// "A basic land card": the supertype and not the type. A library of duals
/// answers the search with nothing, and the one Plains beside them is the
/// only thing on offer.
#[test]
fn only_a_basic_land_answers_the_search() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = creature(96_300, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    game.players[1].library.clear();
    for (instance, definition) in [
        (96_301, cards::TAIGA),
        (96_302, cards::GAEAS_CRADLE),
        (96_303, cards::PLAINS),
    ] {
        game.players[1]
            .library
            .push(card(instance, definition, PlayerId::Two));
    }
    let path = card(96_304, cards::PATH_TO_EXILE, PlayerId::One);
    let path_id = path.id;
    game.players[0].hand.push(path);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == path_id
                    && choices
                        .iter_targets()
                        .any(|chosen| *chosen == Target::Permanent(bears_id))
            }
            _ => false,
        })
        .expect("one white mana casts it at that creature");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..8 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }

    let offered = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the search asks them which land to take")
        .options
        .iter()
        .filter_map(|option| {
            option
                .card
                .and_then(|(_, characteristics)| characteristics.card_definition())
        })
        .collect::<Vec<_>>();

    assert_eq!(
        offered,
        vec![cards::PLAINS],
        "a Taiga and a Cradle are lands and neither is basic",
    );
}

/// It exiles rather than destroys, so what survives destruction does not
/// survive this -- and a token exiled ceases to exist while its controller
/// is paid all the same.
#[test]
fn exile_answers_what_destruction_cannot() {
    let mut game = ready_game();
    game.battlefield.clear();
    let myr = game
        .put_onto_battlefield(PlayerId::Two, cards::DARKSTEEL_MYR)
        .expect("cataloged");
    drain_pending(&mut game);
    game.players[1].library.clear();
    game.players[1]
        .library
        .push(card(96_400, cards::PLAINS, PlayerId::Two));
    let path = card(96_401, cards::PATH_TO_EXILE, PlayerId::One);
    let path_id = path.id;
    game.players[0].hand.push(path);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);

    cast_path(&mut game, path_id, myr);
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == myr),
        "indestructible is no answer to being exiled",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::DARKSTEEL_MYR),
        "and it is in exile rather than the graveyard",
    );
}

/// The land is *put onto* the battlefield rather than played, so it costs
/// them nothing but the shuffle: their land drop is untouched and the Island
/// in their hand still goes down that turn. It is the half of the drawback
/// that makes Path a real cost rather than pure profit.
#[test]
fn the_land_it_pays_them_does_not_use_their_land_drop() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    game.players[PlayerId::Two.index()].library.clear();
    let bears = creature(96_400, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    game.players[PlayerId::Two.index()]
        .library
        .push(card(96_401, cards::FOREST, PlayerId::Two));
    let island = card(96_402, cards::ISLAND, PlayerId::Two);
    let island_id = island.id;
    game.players[PlayerId::Two.index()].hand.push(island);
    let path = card(96_403, cards::PATH_TO_EXILE, PlayerId::One);
    let path_id = path.id;
    game.players[PlayerId::One.index()].hand.push(path);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.players[PlayerId::Two.index()].lands_played_this_turn = 0;

    cast_path(&mut game, path_id, bears_id);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::FOREST),
        "the Forest arrived for them",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].lands_played_this_turn,
        0,
        "put onto the battlefield is not played, so the drop is unspent",
    );

    // Their turn, and the Island still goes down on top of the Forest.
    game.turn += 1;
    game.active_player = PlayerId::Two;
    game.turns_started[PlayerId::Two.index()] += 1;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;

    let play = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == island_id))
        .expect("their land drop was never spent");
    game.apply(PlayerId::Two, play)
        .expect("the Island is played");
    drain_pending(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.controller == PlayerId::Two)
            .count(),
        2,
        "the Forest they were paid and the Island they played",
    );
}
