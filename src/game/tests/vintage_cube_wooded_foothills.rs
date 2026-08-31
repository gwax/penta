//! Wooded Foothills: what the tap in its cost means.
//!
//! Which lands it finds and what the life costs are covered with the
//! fetchland family. What is not is the `{T}` at the front of the cost: a
//! land has no summoning sickness, so a Foothills played this turn cracks
//! at once -- and a Foothills already tapped cannot crack at all.

use super::*;

/// Player One with a Taiga in the library and a Foothills wherever `in_hand`
/// says, on their own main phase with the land drop unspent.
fn staged(in_hand: bool) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].library.clear();
    game.players[PlayerId::One.index()]
        .library
        .push(card(86_000, cards::TAIGA, PlayerId::One));
    let foothills = if in_hand {
        let held = card(86_001, cards::WOODED_FOOTHILLS, PlayerId::One);
        let id = held.id;
        game.players[PlayerId::One.index()].hand.push(held);
        id
    } else {
        let id = game
            .put_onto_battlefield(PlayerId::One, cards::WOODED_FOOTHILLS)
            .expect("cataloged");
        drain_pending(&mut game);
        id
    };
    game.turns_started = [5, 5];
    game.players[PlayerId::One.index()].lands_played_this_turn = 0;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, foothills)
}

fn cracks(game: &Game, foothills: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One).into_iter().find(
        |action| matches!(action, Action::ActivateAbility { source, .. } if *source == foothills),
    )
}

/// Lands are not creatures, so nothing about arriving this turn stops it:
/// play the Foothills and crack it in the same main phase.
#[test]
fn a_fetch_played_this_turn_cracks_at_once() {
    let (mut game, held) = staged(true);

    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == held))
        .expect("the land drop is available");
    game.apply(PlayerId::One, play).expect("it is played");
    drain_pending(&mut game);

    // A land played from hand arrives as a new object, so the permanent is
    // found by what it is rather than by the id it had in hand.
    let played = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::WOODED_FOOTHILLS)
        .expect("it is on the battlefield")
        .card
        .id;
    let crack = cracks(&game, played).expect("summoning sickness is a creature's problem");
    game.apply(PlayerId::One, crack).expect("it activates");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::TAIGA),
        "the Taiga arrived on the turn the Foothills did",
    );
    assert_eq!(game.players[PlayerId::One.index()].life, 19, "one life");
}

/// The `{T}` is a cost like any other: a Foothills that is already tapped
/// has nothing to pay it with, however much life is spare.
#[test]
fn a_tapped_fetch_has_nothing_to_crack_with() {
    let (mut game, foothills) = staged(false);
    assert!(cracks(&game, foothills).is_some(), "untapped it is offered");

    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == foothills)
    {
        permanent.tapped = true;
    }

    assert!(
        cracks(&game, foothills).is_none(),
        "and tapped it is not, life or no life",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        1,
        "the Taiga is still where it was",
    );
}
