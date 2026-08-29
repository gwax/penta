//! Frantic Search: two cards deep, two cards back, and the three mana
//! returned -- which is what makes it free, and what makes the lands it
//! untaps worth reading carefully.

use super::*;

/// Player One holding the Search with three mana up, `mine` tapped lands of
/// their own and `theirs` tapped across the table, and a library to draw
/// from.
fn staged(mine: usize, theirs: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..4 {
        game.players[0]
            .library
            .push(card(125_000 + index, cards::COUNTERSPELL, PlayerId::One));
    }
    for (player, count) in [(PlayerId::One, mine), (PlayerId::Two, theirs)] {
        for index in 0..count {
            let mut land = creature(
                125_100
                    + u32::from(player == PlayerId::Two) * 50
                    + u32::try_from(index).expect("a few lands"),
                cards::ISLAND,
                player,
            );
            land.tapped = true;
            game.battlefield.push(land);
        }
    }
    let search = card(125_200, cards::FRANTIC_SEARCH, PlayerId::One);
    let search_id = search.id;
    game.players[0].hand.push(search);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, search_id)
}

/// Casts it and stops at the decision it asks.
fn cast(game: &mut Game, search: GameObjectId) -> DecisionObservation {
    game.apply(
        PlayerId::One,
        cast_action(search, Vec::new(), Vec::new(), 0),
    )
    .expect("three mana casts it");
    pass_until_decision(game);
    game.observe(PlayerId::One)
        .decision
        .expect("something is being asked")
}

fn untapped_of(game: &Game, player: PlayerId) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.controller == player && !permanent.tapped)
        .count()
}

/// "They don't have to be lands that you control": their tapped Island is
/// as good a choice as yours, whatever you would want that for.
#[test]
fn their_lands_are_choices_too() {
    let (mut game, search) = staged(1, 2);

    let decision = cast(&mut game, search);
    assert_eq!(
        decision.options.len(),
        3,
        "every tapped land on the battlefield is on offer",
    );
    let theirs = decision
        .options
        .iter()
        .filter(|option| {
            option.card.is_some_and(|(id, _)| {
                game.battlefield.iter().any(|permanent| {
                    permanent.card.id == id && permanent.controller == PlayerId::Two
                })
            })
        })
        .map(|option| option.id)
        .collect::<Vec<_>>();
    assert_eq!(theirs.len(), 2, "two of them are theirs");

    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: theirs,
        },
    )
    .expect("nothing about the choice asks who controls them");
    drain_pending(&mut game);

    assert_eq!(untapped_of(&game, PlayerId::Two), 2, "theirs came back up");
    assert_eq!(
        untapped_of(&game, PlayerId::One),
        0,
        "and yours stayed where it was",
    );
}

/// "Up to three": fewer is an answer, and none at all is one too.
#[test]
fn untapping_nothing_is_an_answer() {
    let (mut game, search) = staged(3, 0);
    let hand = game.players[0].hand.len();

    let decision = cast(&mut game, search);
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: Vec::new(),
        },
    )
    .expect("up to three is a maximum");
    drain_pending(&mut game);

    assert_eq!(untapped_of(&game, PlayerId::One), 0, "nothing came back up");
    assert_eq!(
        game.players[0].hand.len(),
        hand - 1,
        "and the draws and discards still cancelled out",
    );
}

/// "Nothing can happen between the two, and no player may choose to take
/// actions": the discard is part of the resolution.
#[test]
fn nobody_acts_between_the_draws_and_the_discards() {
    let (mut game, search) = staged(1, 0);
    game.players[0]
        .hand
        .push(card(125_300, cards::MOUNTAIN, PlayerId::One));
    game.players[0]
        .hand
        .push(card(125_301, cards::FOREST, PlayerId::One));

    let decision = cast(&mut game, search);
    assert!(
        decision.prompt.to_lowercase().contains("discard"),
        "the discard is what it stops on: {}",
        decision.prompt,
    );
    assert_eq!(
        game.legal_actions(PlayerId::Two),
        vec![Action::Concede],
        "and the other player has no window while it waits",
    );
}
