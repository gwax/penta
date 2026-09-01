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

/// A Sheoldred across from it turns the deal around: two draws are four
/// life, and the Whisper's own two comes off that.
#[test]
fn a_sheoldred_of_yours_turns_the_life_around() {
    let (mut game, whisper) = staged(6);
    game.put_onto_battlefield(PlayerId::One, cards::SHEOLDRED_THE_APOCALYPSE)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    let life = game.players[PlayerId::One.index()].life;

    cast_it(&mut game, whisper);

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life + 2,
        "two life a card, less the two the Whisper takes",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].hand.len(),
        2,
        "and both cards are in hand",
    );
}

/// Their Orcish Bowmasters reads every draw of yours outside your own draw
/// step, and a Whisper is two of them: two arrows and an Army two counters
/// tall.
#[test]
fn their_bowmasters_shoots_at_both_of_its_draws() {
    let (mut game, whisper) = staged(6);
    game.put_onto_battlefield(PlayerId::Two, cards::ORCISH_BOWMASTERS)
        .expect("cataloged");
    // The Bowmasters' own arrival trigger goes first, aimed at whatever it
    // likes; what this test counts is what the Whisper adds to that.
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: decision
                        .options
                        .iter()
                        .find(|option| option.label == "your opponent")
                        .or_else(|| decision.options.first())
                        .map(|option| vec![option.id])
                        .unwrap_or_default(),
                },
            )
            .expect("the offered choice is legal");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    let army = |game: &Game| {
        game.battlefield
            .iter()
            .find(|permanent| {
                is_token_with(
                    permanent,
                    tokens::creature(&["Orc", "Army"], &[ManaColor::Black], 0, 0),
                )
            })
            .map_or(0, |permanent| {
                permanent.counters(CounterKind::PlusOnePlusOne)
            })
    };
    let counters = army(&game);
    let life = game.players[PlayerId::One.index()].life;
    game.priority = PlayerId::One;

    cast_it(&mut game, whisper);
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: decision
                        .options
                        .iter()
                        .find(|option| option.label == "your opponent")
                        .or_else(|| decision.options.first())
                        .map(|option| vec![option.id])
                        .unwrap_or_default(),
                },
            )
            .expect("the offered choice is legal");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();

    assert_eq!(
        army(&game) - counters,
        2,
        "one amass for each of the two cards drawn",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life - 4,
        "two life for the Whisper and an arrow for each draw",
    );
}
