//! Marsh Flats under a Blood Moon.
//!
//! What a fetchland finds, what it costs, and what it does when it finds
//! nothing are covered with the fetchland family. This is the other thing
//! that happens to one: a Blood Moon leaves it a Mountain, and a Mountain
//! has nothing to sacrifice itself for.

use super::*;

/// The Flats on the battlefield with a Scrubland in the library, and a Blood
/// Moon out when `moon` is set.
fn staged(moon: bool) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].library.clear();
    game.players[PlayerId::One.index()]
        .library
        .push(card(84_000, cards::SCRUBLAND, PlayerId::One));
    if moon {
        game.battlefield
            .push(creature(84_001, cards::BLOOD_MOON, PlayerId::Two));
    }
    let flats = game
        .put_onto_battlefield(PlayerId::One, cards::MARSH_FLATS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, flats)
}

fn cracks(game: &Game, flats: GameObjectId) -> usize {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == flats),
        )
        .count()
}

fn colors(game: &Game, flats: GameObjectId) -> Vec<ManaColor> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateManaAbility { source, color, .. } if source == flats => Some(color),
            _ => None,
        })
        .collect()
}

/// A fetchland's printed line is one activated ability and no mana ability
/// at all; a Blood Moon trades that for the Mountain's.
#[test]
fn a_blood_moon_trades_the_fetch_for_a_red_mana() {
    let (game, flats) = staged(false);
    assert_eq!(cracks(&game, flats), 1, "the search is offered");
    assert!(
        colors(&game, flats).is_empty(),
        "and a fetchland makes no mana of its own",
    );

    let (game, flats) = staged(true);
    assert_eq!(
        cracks(&game, flats),
        0,
        "a Mountain has nothing to sacrifice itself for",
    );
    assert_eq!(
        colors(&game, flats),
        vec![ManaColor::Red],
        "what it has instead is the Mountain's tap",
    );
}

/// The Moon is not a one-way door: answering it hands the fetch back, with
/// the land still in the library to go and get.
#[test]
fn the_fetch_comes_back_when_the_moon_goes() {
    let (mut game, flats) = staged(true);
    assert_eq!(cracks(&game, flats), 0, "held under the Moon");

    let moon = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::BLOOD_MOON)
        .expect("the Moon is out")
        .card
        .id;
    game.move_permanents_to_graveyard(&[moon]);
    game.check_state_based_actions();
    drain_pending(&mut game);

    assert_eq!(cracks(&game, flats), 1, "and offered again once it is gone");

    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == flats))
        .expect("the search is there");
    game.apply(PlayerId::One, crack).expect("it activates");
    pass_priority_pair(&mut game);
    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("it asks which land to take");
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![decision.options[0].id],
        },
    )
    .expect("taking the Scrubland is legal");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::SCRUBLAND),
        "a Swamp Plains is what the Flats were looking for",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        19,
        "a life for it"
    );
}

/// The Moon is only ever late once the ability is announced: the land and
/// the life are costs, the ability is its own object on the stack, and a
/// Blood Moon resolving on top of it takes nothing back. The Scrubland the
/// search finds is read out of the library, where the Moon reaches nothing
/// -- and then arrives on a battlefield where it is a Mountain.
#[test]
fn a_moon_landing_on_top_of_the_fetch_does_not_stop_it() {
    let (mut game, flats) = staged(false);
    let life = game.players[PlayerId::One.index()].life;

    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == flats))
        .expect("with no Moon out it is a fetchland");
    game.apply(PlayerId::One, crack).expect("it activates");
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == flats),
        "the land was sacrificed on announcement",
    );

    game.battlefield
        .push(creature(84_100, cards::BLOOD_MOON, PlayerId::Two));
    game.check_state_based_actions();
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    let found = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SCRUBLAND)
        .expect("the search still found it in the library");
    let id = found.card.id;
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life - 1,
        "one life, paid before the Moon was anywhere",
    );
    assert_eq!(
        colors(&game, id),
        vec![ManaColor::Red],
        "and what arrived is a Mountain, whatever it was in the library",
    );
}
