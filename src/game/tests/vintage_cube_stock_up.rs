//! Stock Up: five cards seen and two kept.
//!
//! That it sees five, takes exactly two and buries the rest is covered with
//! the library spells. What this adds is which two: the cards named are the
//! cards that arrive, and a library too short to show five shows what it
//! has.

use super::*;

/// Player One with `library` stacked top-first and a Stock Up in hand.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].library.clear();
    // The library's last element is its top, so a top-first list goes in
    // backwards.
    for (index, definition) in library.iter().rev().enumerate() {
        game.players[PlayerId::One.index()].library.push(card(
            92_000 + u32::try_from(index).expect("a short library"),
            *definition,
            PlayerId::One,
        ));
    }
    let stock_up = card(92_500, cards::STOCK_UP, PlayerId::One);
    let stock_up_id = stock_up.id;
    game.players[PlayerId::One.index()].hand.push(stock_up);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, stock_up_id)
}

/// Casts it and stops on the choice it puts up.
fn cast_and_look(game: &mut Game, stock_up: GameObjectId) -> DecisionObservation {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == stock_up))
        .expect("three mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_until_decision(game);
    game.pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the dig asks which cards to take")
}

fn hand(game: &Game) -> Vec<CardDefinitionId> {
    let mut cards = game.players[PlayerId::One.index()]
        .hand
        .iter()
        .map(|card| card.definition)
        .collect::<Vec<_>>();
    cards.sort_unstable();
    cards
}

/// The two named are the two that come: the Lotus and the Recall out of five
/// distinguishable cards, with the other three left to the bottom.
#[test]
fn the_two_it_names_are_the_two_it_takes() {
    let (mut game, stock_up) = staged(&[
        cards::GRIZZLY_BEARS,
        cards::BLACK_LOTUS,
        cards::MOUNTAIN,
        cards::ANCESTRAL_RECALL,
        cards::SERRA_ANGEL,
    ]);

    let look = cast_and_look(&mut game, stock_up);
    let wanted = [cards::BLACK_LOTUS, cards::ANCESTRAL_RECALL]
        .into_iter()
        .map(|definition| {
            look.options
                .iter()
                .find(|option| {
                    option
                        .card
                        .and_then(|(_, characteristics)| characteristics.card_definition())
                        == Some(definition)
                })
                .map_or_else(
                    || panic!("{definition:?} is among the five"),
                    |option| option.id,
                )
        })
        .collect::<Vec<_>>();
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: look.id,
            options: wanted,
        },
    )
    .expect("taking those two is legal");
    drain_pending(&mut game);

    let mut expected = vec![cards::BLACK_LOTUS, cards::ANCESTRAL_RECALL];
    expected.sort_unstable();
    assert_eq!(hand(&game), expected, "what was named is what arrived");
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        3,
        "and the three it passed over went back under the library",
    );
    for definition in [cards::GRIZZLY_BEARS, cards::MOUNTAIN, cards::SERRA_ANGEL] {
        assert!(
            game.players[PlayerId::One.index()]
                .library
                .iter()
                .any(|card| card.definition == definition),
            "{definition:?} is still in the library",
        );
    }
}

/// "Look at the top five cards" of a library that has three shows three, and
/// two of those three are still what it takes.
#[test]
fn a_library_shorter_than_five_shows_what_it_has() {
    let (mut game, stock_up) = staged(&[
        cards::BLACK_LOTUS,
        cards::ANCESTRAL_RECALL,
        cards::GRIZZLY_BEARS,
    ]);

    let look = cast_and_look(&mut game, stock_up);
    assert_eq!(look.options.len(), 3, "three cards is all there is to see");
    assert_eq!(
        (look.minimum, look.maximum),
        (2, 2),
        "and two of them is still the price",
    );
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: look.id,
            options: look
                .options
                .iter()
                .take(2)
                .map(|option| option.id)
                .collect(),
        },
    )
    .expect("taking two of the three is legal");
    drain_pending(&mut game);

    assert_eq!(hand(&game).len(), 2, "two came to hand");
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        1,
        "and the third went back under an empty library",
    );
}
