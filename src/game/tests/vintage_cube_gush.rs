//! Gush: two Islands back to hand instead of five mana, and two cards for
//! them.
//!
//! Which lands pay for it, and that they come back to hand rather than
//! dying, is pinned in `premodern_free_spells`. What is here is the rest of
//! the card: the draw it is played for, the printed cost behind the free
//! one, and when the lands are actually spent.

use super::*;

/// Player One holding a Gush with `islands` Islands out and a stocked
/// library.
fn staged(islands: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..8 {
        game.players[0]
            .library
            .push(card(117_000 + index, cards::MOUNTAIN, PlayerId::One));
    }
    for index in 0..islands {
        game.battlefield.push(creature(
            117_100 + u32::try_from(index).expect("a few lands"),
            cards::ISLAND,
            PlayerId::One,
        ));
    }
    let gush = game
        .build_zone(PlayerId::One, &[cards::GUSH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let gush_id = gush.id;
    game.players[0].hand.push(gush);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, gush_id)
}

/// The free cast of the Gush, if it is on offer.
fn free_cast(game: &Game, gush: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == gush && choices.costs().alternative().is_some())
        })
}

/// The two cards it is played for: the Islands leave and the hand is two
/// deeper for them.
#[test]
fn it_draws_two_for_the_two_islands() {
    let (mut game, gush) = staged(2);
    let library = game.players[0].library.len();

    let cast = free_cast(&game, gush).expect("two Islands pay for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].library.len(),
        library - 2,
        "two cards off the library",
    );
    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .filter(|card| card.definition == cards::MOUNTAIN)
            .count(),
        2,
        "and into your hand, beside the Islands that paid",
    );
}

/// A Gush with no Islands is still a Gush: five mana is what the card says.
#[test]
fn without_islands_it_is_still_castable_for_five() {
    let (mut game, gush) = staged(0);
    assert!(free_cast(&game, gush).is_none(), "nothing to return");

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 4);
    let library = game.players[0].library.len();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == gush))
        .expect("five mana is what it says on the card");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].library.len(),
        library - 2,
        "and it draws the same two",
    );
}

/// The Islands are a cost: they are in hand before anybody may answer the
/// Gush, and a Counterspell takes the cards without giving the lands back.
#[test]
fn the_islands_are_spent_even_when_it_is_countered() {
    let (mut game, gush) = staged(2);
    game.players[1]
        .hand
        .push(card(117_900, cards::COUNTERSPELL, PlayerId::Two));
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);
    let library = game.players[0].library.len();

    let cast = free_cast(&game, gush).expect("two Islands pay for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .filter(|card| card.definition == cards::ISLAND)
            .count(),
        2,
        "both Islands are in hand before the spell resolves",
    );

    game.priority = PlayerId::Two;
    let counter = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, .. } if *card == CardInstanceId(117_900))
        })
        .expect("two blue answers it");
    game.apply(PlayerId::Two, counter).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(game.players[0].library.len(), library, "nothing was drawn");
    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .filter(|card| card.definition == cards::ISLAND)
            .count(),
        2,
        "and the lands stay where the cost put them",
    );
    assert!(
        game.battlefield.is_empty(),
        "with the board two lands lighter"
    );
}
