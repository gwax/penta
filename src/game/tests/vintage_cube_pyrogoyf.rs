//! Pyrogoyf: whichever Lhurgoyf arrives is the one that throws the fire, and
//! the fire is its own -- its power, and its colour.

use super::*;

/// Pyrogoyf already out, with one card type in each graveyard: it is a 2/3,
/// and a Nethergoyf arriving beside it would be a 1/2, because Nethergoyf
/// counts only your own pile.
fn staged() -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].graveyard.clear();
    game.players[PlayerId::Two.index()].graveyard.clear();
    game.players[PlayerId::One.index()].graveyard.push(card(
        63_000,
        cards::GRIZZLY_BEARS,
        PlayerId::One,
    ));
    game.players[PlayerId::Two.index()].graveyard.push(card(
        63_001,
        cards::LIGHTNING_BOLT,
        PlayerId::Two,
    ));
    game.put_onto_battlefield(PlayerId::One, cards::PYROGOYF)
        .expect("cataloged");
    settle_declining(&mut game);
    game
}

/// Answers the pending trigger by taking the named target, then lets the
/// ability resolve.
fn aim_at(game: &mut Game, label: &str) {
    let decision = loop {
        if let Some(decision) = game.observe(PlayerId::One).decision {
            break decision;
        }
        let player = game.priority;
        assert!(
            game.apply(player, Action::PassPriority).is_ok(),
            "the enters trigger should be waiting on a target",
        );
    };
    let target = decision
        .options
        .iter()
        .find(|option| option.label == label)
        .unwrap_or_else(|| {
            panic!(
                "{label} is one of the offered targets; got {:?}",
                decision
                    .options
                    .iter()
                    .map(|option| option.label.clone())
                    .collect::<Vec<_>>()
            )
        })
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![target],
        },
    )
    .expect("a target is chosen");
    drain_pending(game);
}

/// Clears whatever the staging triggered, taking the opponent each time so
/// the board is quiet before the test's own arrival.
fn settle_declining(game: &mut Game) {
    for _ in 0..8 {
        if game.observe(PlayerId::One).decision.is_some() {
            aim_at(game, "your opponent");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(game);
}

/// "That creature deals damage equal to its power": the Lhurgoyf that
/// arrived, not the one whose ability it is. Nethergoyf counts only your
/// graveyard, so the two are different sizes and the difference shows.
#[test]
fn the_lhurgoyf_that_entered_deals_its_own_power() {
    let mut game = staged();
    let before = game.players[PlayerId::Two.index()].life;

    game.put_onto_battlefield(PlayerId::One, cards::NETHERGOYF)
        .expect("cataloged");
    aim_at(&mut game, "your opponent");

    assert_eq!(
        before - game.players[PlayerId::Two.index()].life,
        1,
        "the 1/2 Nethergoyf that entered, not the 2/3 Pyrogoyf watching it",
    );
}

/// And it deals it: protection from black stops a black Lhurgoyf's damage,
/// which is the whole of what naming the source is for. Pyrogoyf's own
/// ability put it on the stack, so the target was legal to choose.
#[test]
fn protection_answers_the_lhurgoyf_that_entered() {
    let mut game = staged();
    let knight = game
        .put_onto_battlefield(PlayerId::Two, cards::WHITE_KNIGHT)
        .expect("cataloged");
    drain_pending(&mut game);

    game.put_onto_battlefield(PlayerId::One, cards::NETHERGOYF)
        .expect("cataloged");
    aim_at(&mut game, "White Knight");

    let knight = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == knight)
        .expect("it survived");
    assert_eq!(
        knight.damage, 0,
        "a black source deals it nothing, whoever's ability sent it",
    );
}

/// The red one is not stopped, which is what makes the test above about the
/// source rather than about protection swallowing everything. Pyrogoyf
/// arrives to an empty board so that its own arrival is the only trigger.
#[test]
fn a_red_lhurgoyf_still_burns_through_protection_from_black() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].graveyard.clear();
    game.players[PlayerId::Two.index()].graveyard.clear();
    game.players[PlayerId::One.index()].graveyard.push(card(
        63_100,
        cards::GRIZZLY_BEARS,
        PlayerId::One,
    ));
    game.players[PlayerId::Two.index()].graveyard.push(card(
        63_101,
        cards::LIGHTNING_BOLT,
        PlayerId::Two,
    ));
    let knight = game
        .put_onto_battlefield(PlayerId::Two, cards::WHITE_KNIGHT)
        .expect("cataloged");
    drain_pending(&mut game);

    game.put_onto_battlefield(PlayerId::One, cards::PYROGOYF)
        .expect("cataloged");
    aim_at(&mut game, "White Knight");

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == knight),
        "two from a red source killed the 2/2: protection from black says \
         nothing about red",
    );
}
