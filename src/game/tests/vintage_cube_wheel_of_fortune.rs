//! Wheel of Fortune, and the size of the hands it throws away.
//!
//! The shape of the card -- both hands emptied and seven dealt, the draw
//! order under CR 121.2c, and what a short library makes of it -- is in
//! `activation_costs_and_turns`. Those tests each start from a hand of one.
//! What is here is the discard when there is a real hand to lose, and when
//! the card may be cast at all.

use super::*;

/// Player One holding the Wheel and `mine` cards besides, with `theirs`
/// across the table and the three mana for it.
fn staged(mine: usize, theirs: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    game.players[0].graveyard.clear();
    game.players[1].graveyard.clear();
    let wheel = card(99_000, cards::WHEEL_OF_FORTUNE, PlayerId::One);
    let wheel_id = wheel.id;
    game.players[0].hand.push(wheel);
    for index in 0..mine {
        game.players[0].hand.push(card(
            99_100 + u32::try_from(index).expect("a hand"),
            cards::MOUNTAIN,
            PlayerId::One,
        ));
    }
    for index in 0..theirs {
        game.players[1].hand.push(card(
            99_200 + u32::try_from(index).expect("a hand"),
            cards::ISLAND,
            PlayerId::Two,
        ));
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 3);
    (game, wheel_id)
}

fn cast_it(game: &mut Game, wheel: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == wheel))
        .expect("three mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    pass_priority_pair(game);
    drain_pending(game);
}

/// "Each player discards their hand": the whole of it, however big, and it
/// is a discard rather than a trim to seven. Nine cards go and seven come
/// back, which is the trade a player with a full hand is making.
#[test]
fn a_hand_of_nine_is_discarded_whole_and_seven_come_back() {
    let (mut game, wheel) = staged(9, 2);

    cast_it(&mut game, wheel);

    assert_eq!(game.players[0].hand.len(), 7, "seven for the caster");
    assert_eq!(game.players[1].hand.len(), 7, "and seven for them");
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .filter(|card| card.definition == cards::MOUNTAIN)
            .count(),
        9,
        "every one of the nine was discarded, not just the excess",
    );
    assert_eq!(
        game.players[1]
            .graveyard
            .iter()
            .filter(|card| card.definition == cards::ISLAND)
            .count(),
        2,
        "and their two went to their own graveyard",
    );
    assert!(
        game.players[0]
            .hand
            .iter()
            .all(|card| !(99_100..99_109).contains(&card.id.0)),
        "and not one of the nine is back in hand: the discard happened first",
    );
}

/// An empty hand is a hand to discard all the same, and seven still arrive.
#[test]
fn an_empty_hand_still_draws_its_seven() {
    let (mut game, wheel) = staged(0, 0);

    cast_it(&mut game, wheel);

    assert_eq!(game.players[0].hand.len(), 7);
    assert_eq!(game.players[1].hand.len(), 7);
    assert!(
        game.players[1].graveyard.is_empty(),
        "with nothing discarded on the way",
    );
}

/// A sorcery: it waits for your own turn with the stack empty, which is the
/// whole reason the symmetry is survivable.
#[test]
fn it_waits_for_your_own_main_phase() {
    let (mut game, wheel) = staged(1, 1);
    let castable = |game: &Game| {
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == wheel))
    };
    assert!(castable(&game), "your main phase, and the stack empty");

    game.active_player = PlayerId::Two;
    game.turns_started = [5, 6];
    assert!(!castable(&game), "and never on theirs");
}
