//! Demonic Tutor: two mana for any card in the deck, and the two things it
//! does not do -- it does not show anybody what it took, and it does not
//! draw it.

use super::*;

/// Player One holding the Tutor with the mana for it and `library` behind.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for (index, definition) in library.iter().enumerate() {
        game.players[0].library.push(card(
            123_000 + u32::try_from(index).expect("a short library"),
            *definition,
            PlayerId::One,
        ));
    }
    let tutor = card(123_100, cards::DEMONIC_TUTOR, PlayerId::One);
    let tutor_id = tutor.id;
    game.players[0].hand.push(tutor);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.cards_drawn_this_turn = [0; 2];
    (game, tutor_id)
}

/// Casts the Tutor and stops with the search waiting.
fn cast(game: &mut Game, tutor: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == tutor))
        .expect("two mana casts it");
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
}

/// "You don't reveal the card to your opponent": the search is the
/// searcher's business from beginning to end.
#[test]
fn nobody_else_sees_what_it_took() {
    let (mut game, tutor) = staged(&[cards::BLACK_LOTUS, cards::MOUNTAIN]);

    cast(&mut game, tutor);
    assert!(
        game.observe(PlayerId::One).decision.is_some(),
        "the searcher picks on resolution",
    );
    assert!(
        game.observe(PlayerId::Two).decision.is_none(),
        "and the other player is not shown the library",
    );
    drain_pending(&mut game);

    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::CardRevealed { .. })),
        "nothing was revealed on the way to hand",
    );
    assert_eq!(game.players[0].hand.len(), 1, "and a card is in hand");
}

/// "This card is put directly into your hand. It is not drawn." Anything
/// counting draws counts none of it.
#[test]
fn what_it_finds_is_not_a_draw() {
    let (mut game, tutor) = staged(&[cards::BLACK_LOTUS]);

    cast(&mut game, tutor);
    drain_pending(&mut game);

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::BLACK_LOTUS),
        "the Lotus is in hand",
    );
    assert_eq!(
        game.cards_drawn_this_turn[0], 0,
        "and no card was drawn to put it there",
    );
}
