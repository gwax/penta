//! Timetwister: everyone's hand and graveyard go back in, and everyone draws
//! a fresh seven out of what is left.

use super::*;

/// Player One holding a Timetwister with the mana for it, `library` beneath,
/// `hand` beside it, and `graveyard` behind.
fn staged(
    library: &[CardDefinitionId],
    hand: &[CardDefinitionId],
    graveyard: &[CardDefinitionId],
) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[0].graveyard.clear();
    let mut next = 74_000;
    let mut place = |game: &mut Game, definitions: &[CardDefinitionId], zone: usize| {
        for definition in definitions {
            let placed = card(next, *definition, PlayerId::One);
            next += 1;
            match zone {
                0 => game.players[0].library.push(placed),
                1 => game.players[0].hand.push(placed),
                _ => game.players[0].graveyard.push(placed),
            }
        }
    };
    place(&mut game, library, 0);
    place(&mut game, hand, 1);
    place(&mut game, graveyard, 2);
    let twister = card(74_900, cards::TIMETWISTER, PlayerId::One);
    let twister_id = twister.id;
    game.players[0].hand.push(twister);
    game.turns_started = [4, 4];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].mana_pool.blue = 1;
    game.players[0].mana_pool.colorless = 2;
    (game, twister_id)
}

/// Casts it and lets it resolve.
fn cast(game: &mut Game, twister: GameObjectId) {
    game.apply(
        PlayerId::One,
        cast_action(twister, Vec::new(), Vec::new(), 0),
    )
    .expect("three mana casts it");
    pass_priority_pair(game);
    drain_pending(game);
}

/// What Player One is holding, by definition.
fn hand(game: &Game) -> Vec<CardDefinitionId> {
    let mut definitions = game.players[0]
        .hand
        .iter()
        .map(|card| card.definition)
        .collect::<Vec<_>>();
    definitions.sort_unstable();
    definitions
}

/// Seven cards go in and seven come out, so the draw is every one of them:
/// the card that was in hand and the two that were in the graveyard all come
/// back, because the shuffle put them where the draw would find them.
#[test]
fn what_was_shuffled_in_is_what_is_drawn() {
    let (mut game, twister) = staged(
        &[cards::FOREST; 4],
        &[cards::ISLAND],
        &[cards::SWAMP, cards::MOUNTAIN],
    );

    cast(&mut game, twister);

    let mut expected = vec![cards::FOREST; 4];
    expected.extend([cards::ISLAND, cards::SWAMP, cards::MOUNTAIN]);
    expected.sort_unstable();
    assert_eq!(
        hand(&game),
        expected,
        "four in the library, one in hand and two in the graveyard is the seven",
    );
    assert!(game.players[0].library.is_empty(), "all of it was drawn");
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::TIMETWISTER],
        "and the Timetwister is the only thing in the graveyard it emptied",
    );
}

/// "Draws seven cards", not seven more and not up to seven: a hand of ten
/// goes in whole and comes back three cards smaller.
#[test]
fn a_big_hand_is_traded_for_exactly_seven() {
    let (mut game, twister) = staged(&[cards::FOREST; 20], &[cards::ISLAND; 10], &[]);

    cast(&mut game, twister);

    assert_eq!(game.players[0].hand.len(), 7, "seven, whatever you held");
}

/// The last card of the library is a card like any other. A library that
/// holds exactly seven after the shuffle empties itself and nobody loses:
/// what ends a game is the draw that finds nothing.
#[test]
fn drawing_the_last_seven_is_not_a_loss() {
    let (mut game, twister) = staged(&[cards::FOREST; 6], &[cards::ISLAND], &[]);

    cast(&mut game, twister);
    game.check_state_based_actions();

    assert_eq!(
        game.players[0].hand.len(),
        7,
        "the library had exactly seven"
    );
    assert!(game.players[0].library.is_empty(), "and now it has none");
    assert!(
        game.result.is_none(),
        "an empty library is not a loss by itself"
    );
}

/// Stocks Player Two's zones, replacing whatever the fixture dealt them.
fn stock_them(
    game: &mut Game,
    library: &[CardDefinitionId],
    hand: &[CardDefinitionId],
    graveyard: &[CardDefinitionId],
) {
    game.players[1].library.clear();
    game.players[1].hand.clear();
    game.players[1].graveyard.clear();
    let mut next = 75_000;
    for (definitions, zone) in [(library, 0), (hand, 1), (graveyard, 2)] {
        for definition in definitions {
            let placed = card(next, *definition, PlayerId::Two);
            next += 1;
            match zone {
                0 => game.players[1].library.push(placed),
                1 => game.players[1].hand.push(placed),
                _ => game.players[1].graveyard.push(placed),
            }
        }
    }
}

/// "Each player": the one who cast it is not the only one who wheels. Their
/// hand and graveyard go back in and they draw seven too, however few cards
/// they were holding.
#[test]
fn it_wheels_the_other_player_as_well() {
    let (mut game, twister) = staged(&[cards::FOREST; 10], &[cards::ISLAND], &[]);
    stock_them(
        &mut game,
        &[cards::PLAINS; 4],
        &[cards::SWAMP],
        &[cards::MOUNTAIN, cards::MOUNTAIN],
    );

    cast(&mut game, twister);

    assert_eq!(
        game.players[1].hand.len(),
        7,
        "seven for them as much as for you",
    );
    assert!(
        game.players[1].graveyard.is_empty(),
        "their graveyard went in with everything else",
    );
    assert!(
        game.players[1].library.is_empty(),
        "and their seven cards were the whole of what they had",
    );
    assert_eq!(game.players[0].hand.len(), 7, "and you drew seven as well");
}

/// A player with fewer than seven cards to their name has to draw from an
/// empty library, and that is what ends a game rather than the empty library
/// itself.
#[test]
fn a_player_short_of_seven_draws_from_nothing_and_loses() {
    let (mut game, twister) = staged(&[cards::FOREST; 10], &[cards::ISLAND], &[]);
    stock_them(&mut game, &[cards::PLAINS], &[cards::SWAMP], &[]);

    cast(&mut game, twister);
    game.check_state_based_actions();

    assert_eq!(
        game.result,
        Some(GameResult::Winner {
            winner: PlayerId::One,
            reason: WinReason::OpponentTriedToDrawFromEmptyLibrary,
        }),
        "two cards is not seven",
    );
}
