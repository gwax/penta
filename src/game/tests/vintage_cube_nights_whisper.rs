//! Night's Whisper: two cards for two mana and two life.
//!
//! The life is the back half of one sentence about you rather than a cost,
//! so nothing checks whether you can afford it: the cards are drawn and the
//! life is lost whatever that leaves you at.

use super::*;

/// Player One holding a Whisper with the mana for it and `library` cards
/// beneath.
fn staged(library: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].library.clear();
    for index in 0..library {
        game.players[PlayerId::One.index()].library.push(card(
            93_000 + u32::try_from(index).expect("a short library"),
            cards::ISLAND,
            PlayerId::One,
        ));
    }
    let whisper = card(93_500, cards::NIGHTS_WHISPER, PlayerId::One);
    let whisper_id = whisper.id;
    game.players[PlayerId::One.index()].hand.push(whisper);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, whisper_id)
}

fn cast_it(game: &mut Game, whisper: GameObjectId) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == whisper))
        .expect("two mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(game);
    game.check_state_based_actions();
}

/// Two cards, two life, and the Whisper itself spent.
#[test]
fn it_draws_two_and_costs_two_life() {
    let (mut game, whisper) = staged(5);

    cast_it(&mut game, whisper);

    assert_eq!(
        game.players[PlayerId::One.index()].hand.len(),
        2,
        "the Whisper left and two came back",
    );
    assert_eq!(game.players[PlayerId::One.index()].life, 18);
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        3,
        "two off the top",
    );
}

/// The life is not a cost, so nothing stops it being paid: at two life the
/// spell is castable, the cards are drawn, and the game ends with them in
/// hand.
#[test]
fn at_two_life_it_still_draws_and_still_kills_you() {
    let (mut game, whisper) = staged(5);
    game.players[PlayerId::One.index()].life = 2;

    cast_it(&mut game, whisper);

    assert_eq!(
        game.players[PlayerId::One.index()].hand.len(),
        2,
        "the draw happened first, as the card prints it",
    );
    assert_eq!(game.players[PlayerId::One.index()].life, 0);
    assert_eq!(
        game.result,
        Some(GameResult::Winner {
            winner: PlayerId::Two,
            reason: WinReason::OpponentLostAllLife,
        }),
        "and two life is what the sentence took",
    );
}

/// A library with one card in it is one card and a draw that finds nothing:
/// what ends the game is the empty draw rather than the life.
#[test]
fn drawing_two_off_one_card_ends_it_another_way() {
    let (mut game, whisper) = staged(1);

    cast_it(&mut game, whisper);

    assert_eq!(
        game.result,
        Some(GameResult::Winner {
            winner: PlayerId::Two,
            reason: WinReason::OpponentTriedToDrawFromEmptyLibrary,
        }),
        "the second card was not there to draw",
    );
}
