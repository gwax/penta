//! Xander's Lounge, where the cycle is checked as a family: an Island Swamp
//! Mountain is three basic types on one card, which is three different
//! fetchlands that find it and a cycling clause that is not a land drop.

use super::*;

fn fetch_finds(fetch: CardDefinitionId) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    let source = game
        .put_onto_battlefield(PlayerId::One, fetch)
        .expect("cataloged");
    game.players[PlayerId::One.index()].library.clear();
    game.players[PlayerId::One.index()].library.push(card(
        95_000,
        cards::XANDERS_LOUNGE,
        PlayerId::One,
    ));

    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
        })
        .expect("the fetch is offered");
    game.apply(PlayerId::One, crack).expect("it activates");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);
    game
}

/// Its three basic types pair up three ways, and each pair is a fetchland
/// printed in the cube: a Delta reads the Island and the Swamp, a Tarn the
/// Island and the Mountain, a Mire the Swamp and the Mountain.
#[test]
fn each_of_its_three_fetchlands_finds_it() {
    for fetch in [
        cards::POLLUTED_DELTA,
        cards::SCALDING_TARN,
        cards::BLOODSTAINED_MIRE,
    ] {
        let game = fetch_finds(fetch);
        let found = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.definition == cards::XANDERS_LOUNGE)
            .unwrap_or_else(|| panic!("{fetch:?} found the Lounge"));
        assert!(found.tapped, "and it arrives tapped however it got there");
        assert_eq!(
            game.players[PlayerId::One.index()].life,
            19,
            "the fetch cost its life",
        );
    }
}

/// A Windswept Heath wants a Forest or a Plains, and the Lounge is neither.
#[test]
fn a_fetchland_that_reads_neither_of_its_types_leaves_it_alone() {
    let game = fetch_finds(cards::WINDSWEPT_HEATH);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::XANDERS_LOUNGE),
        "no Forest and no Plains is nothing to find",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        1,
        "and the Lounge is still in the library",
    );
}

/// Cycling is an activated ability rather than a land drop: the turn's drop
/// is still there afterwards, and the land is in the graveyard as a land
/// card.
#[test]
fn cycling_it_spends_no_land_drop() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let lounge = card(95_100, cards::XANDERS_LOUNGE, PlayerId::One);
    let lounge_id = lounge.id;
    game.players[PlayerId::One.index()].hand.push(lounge);
    let second = card(95_101, cards::XANDERS_LOUNGE, PlayerId::One);
    let second_id = second.id;
    game.players[PlayerId::One.index()].hand.push(second);
    game.players[PlayerId::One.index()].mana_pool.colorless = 3;
    game.players[PlayerId::One.index()].lands_played_this_turn = 0;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let cycle = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == lounge_id))
        .expect("three generic cycles it");
    game.apply(PlayerId::One, cycle).expect("it activates");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].lands_played_this_turn,
        0,
        "discarding it is not playing it",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::XANDERS_LOUNGE),
    );
    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == second_id))
        .expect("the land drop was never spent");
    game.apply(PlayerId::One, play)
        .expect("the second is played");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::XANDERS_LOUNGE),
        "and it is the one that arrives on the battlefield",
    );
}
