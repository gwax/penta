//! Imperial Seal: any card in the deck, on top of the library, for one mana
//! and two life you do not get to decline.

use super::search_and_reveal::stack_library;
use super::*;

/// Player One with a Seal in hand, one black mana, and `library` beneath --
/// listed top card first.
fn staged(library: &[(u32, CardDefinitionId)]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.players[0].library.clear();
    stack_library(&mut game, library);
    let seal = card(50_950, cards::IMPERIAL_SEAL, PlayerId::One);
    let seal_id = seal.id;
    game.players[0].hand.push(seal);
    game.players[0].mana_pool.black = 1;
    (game, seal_id)
}

/// Casts it and stops at the search it asks.
fn cast_to_the_search(game: &mut Game, seal: GameObjectId) -> DecisionObservation {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == seal))
        .expect("one black mana casts it in a main phase");
    game.apply(PlayerId::One, action).expect("it is cast");
    pass_priority_pair(game);
    game.observe(PlayerId::One).decision.expect("a search")
}

/// Runs whatever is left on the stack.
fn settle(game: &mut Game) {
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

/// "Search your library for a card" with nothing after it: a land, a
/// creature and an instant are all equally what it is looking for.
#[test]
fn the_search_is_for_any_card_at_all() {
    let (mut game, seal) = staged(&[
        (50_960, cards::GRIZZLY_BEARS),
        (50_961, cards::LIGHTNING_BOLT),
        (50_962, cards::MOUNTAIN),
    ]);

    let decision = cast_to_the_search(&mut game, seal);
    let mut offered = decision
        .options
        .iter()
        .filter_map(|option| {
            option
                .card
                .and_then(|(_, characteristics)| characteristics.card_definition())
        })
        .collect::<Vec<_>>();
    offered.sort_unstable();
    let mut expected = vec![cards::GRIZZLY_BEARS, cards::LIGHTNING_BOLT, cards::MOUNTAIN];
    expected.sort_unstable();

    assert_eq!(offered, expected, "the whole library is eligible");
}

/// "You lose 2 life" is not conditional on anything, and losing your last
/// two ends the game. The card you tutored for is on top of a library you
/// will not draw from.
#[test]
fn the_two_life_is_paid_even_when_it_is_your_last() {
    let (mut game, seal) = staged(&[(50_970, cards::GRIZZLY_BEARS), (50_971, cards::SERRA_ANGEL)]);
    game.players[0].life = 2;

    let decision = cast_to_the_search(&mut game, seal);
    let angel = decision
        .options
        .iter()
        .find(|option| {
            option.card.is_some_and(|(_, characteristics)| {
                characteristics.card_definition() == Some(cards::SERRA_ANGEL)
            })
        })
        .expect("the Angel is in there")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![angel],
        },
    )
    .expect("the search is answered");
    settle(&mut game);

    assert_eq!(
        game.players[0].library.last().map(|card| card.definition),
        Some(cards::SERRA_ANGEL),
        "the Seal did everything it promised",
    );
    assert_eq!(game.players[0].life, 0);
    assert_eq!(
        game.result,
        Some(GameResult::Winner {
            winner: PlayerId::Two,
            reason: WinReason::OpponentLostAllLife,
        }),
        "and the last two life is still two life",
    );
}
