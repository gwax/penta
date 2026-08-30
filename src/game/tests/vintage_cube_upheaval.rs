//! Upheaval: six mana that empties the board, including the lands that paid
//! for it.

use super::*;

/// Player One with an Upheaval in hand and the six mana it costs.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    let upheaval = card(76_000, cards::UPHEAVAL, PlayerId::One);
    let upheaval_id = upheaval.id;
    game.players[0].hand.push(upheaval);
    game.turns_started = [6, 6];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].mana_pool.colorless = 4;
    game.players[0].mana_pool.blue = 2;
    (game, upheaval_id)
}

/// Casts it and lets it resolve.
fn cast(game: &mut Game, upheaval: GameObjectId) {
    game.apply(
        PlayerId::One,
        cast_action(upheaval, Vec::new(), Vec::new(), 0),
    )
    .expect("six mana casts it");
    pass_priority_pair(game);
    drain_pending(game);
    game.check_state_based_actions();
}

/// "To their owners' hands": a creature you have taken control of goes home
/// to the player who owns it, not to the player sweeping the board.
#[test]
fn a_stolen_creature_goes_back_to_its_owner() {
    let (mut game, upheaval) = staged();
    let mut stolen = creature(76_100, cards::GRIZZLY_BEARS, PlayerId::Two);
    stolen.controller = PlayerId::One;
    game.battlefield.push(stolen);

    cast(&mut game, upheaval);

    assert!(game.battlefield.is_empty(), "the board is empty");
    assert!(
        game.players[0].hand.is_empty(),
        "controlling it is not owning it",
    );
    assert_eq!(
        game.players[1]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::GRIZZLY_BEARS],
        "and its owner is who it goes back to",
    );
}

/// A token that leaves the battlefield ceases to exist, so the hand it was
/// returned to never holds it (CR 111.7).
#[test]
fn a_token_returned_to_hand_ceases_to_exist() {
    let (mut game, upheaval) = staged();
    game.battlefield.push(token_permanent(
        76_200,
        tokens::creature(&["Bird"], &[ManaColor::White], 1, 1),
        PlayerId::One,
    ));

    cast(&mut game, upheaval);

    assert!(game.battlefield.is_empty(), "the Bird left the battlefield");
    assert!(
        game.players[0].hand.is_empty(),
        "and there is nothing in hand where it went",
    );
}

/// "All permanents" is every permanent and only permanents: the Upheaval is
/// a sorcery on the stack while it resolves, so it finishes in the graveyard
/// rather than back in the hand it was cast from.
#[test]
fn the_upheaval_itself_is_not_swept_up() {
    let (mut game, upheaval) = staged();
    game.battlefield
        .push(creature(76_300, cards::SAVANNAH_LIONS, PlayerId::One));
    game.battlefield
        .push(creature(76_301, cards::MOUNTAIN, PlayerId::Two));

    cast(&mut game, upheaval);

    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::UPHEAVAL],
        "the sorcery went where sorceries go",
    );
    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::SAVANNAH_LIONS],
        "the hand holds what the board held, and nothing else",
    );
    assert_eq!(
        game.players[1]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::MOUNTAIN],
        "lands included",
    );
}
