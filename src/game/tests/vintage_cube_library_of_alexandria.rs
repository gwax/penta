//! Library of Alexandria: a land that draws a card for nothing, on the one
//! condition that the hand it is drawing into is exactly seven.

use super::*;

/// Player One with `libraries` Libraries out and a hand of `hand` cards.
fn staged(libraries: usize, hand: usize) -> (Game, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..8 {
        game.players[0]
            .library
            .push(card(128_000 + index, cards::ISLAND, PlayerId::One));
    }
    for index in 0..hand {
        game.players[0].hand.push(card(
            128_100 + u32::try_from(index).expect("a small hand"),
            cards::MOUNTAIN,
            PlayerId::One,
        ));
    }
    let mut ids = Vec::new();
    for index in 0..libraries {
        let land = creature(
            128_200 + u32::try_from(index).expect("a couple of lands"),
            cards::LIBRARY_OF_ALEXANDRIA,
            PlayerId::One,
        );
        ids.push(land.card.id);
        game.battlefield.push(land);
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, ids)
}

/// Whether the draw ability of `library` is on offer.
fn draw_offered(game: &Game, library: GameObjectId) -> bool {
    game.legal_actions(PlayerId::One).iter().any(
        |action| matches!(action, Action::ActivateAbility { source, .. } if *source == library),
    )
}

fn draw_action(game: &Game, library: GameObjectId) -> Action {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == library),
        )
        .expect("the draw is offered")
}

/// "Exactly seven": six is not enough and eight is too many.
#[test]
fn only_a_hand_of_exactly_seven_draws() {
    for (hand, offered) in [(6, false), (7, true), (8, false)] {
        let (game, libraries) = staged(1, hand);
        assert_eq!(
            draw_offered(&game, libraries[0]),
            offered,
            "a hand of {hand}",
        );
    }
}

/// The colourless half asks nothing of the hand: it is there at six cards
/// as readily as at seven.
#[test]
fn the_colorless_half_is_unconditional() {
    let (game, libraries) = staged(1, 6);

    assert!(
        game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::ActivateManaAbility { source, .. } if *source == libraries[0])
        }),
        "the mana ability does not read your hand",
    );
}

/// "You may tap multiples of these in response to each other because the
/// requirement for 7 cards is checked only at the time the ability is
/// announced and not again when it resolves."
#[test]
fn two_libraries_may_both_be_announced_at_seven() {
    let (mut game, libraries) = staged(2, 7);

    let first = draw_action(&game, libraries[0]);
    game.apply(PlayerId::One, first).expect("seven in hand");
    let second = draw_action(&game, libraries[1]);
    game.apply(PlayerId::One, second)
        .expect("still seven: nothing has resolved yet");

    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].hand.len(),
        9,
        "both drew, though the second resolved into a hand of eight",
    );
}

/// Both halves want the same tap, so the mana is paid for with the card: a
/// Library tapped for {C} is a Library that drew nothing.
#[test]
fn tapping_it_for_mana_spends_the_draw() {
    let (mut game, libraries) = staged(1, 7);
    let library = libraries[0];
    assert!(draw_offered(&game, library), "seven in hand, and untapped");

    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: library,
            ability: mana_ability_for(&game, library, ManaColor::Colorless),
            color: ManaColor::Colorless,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("it taps for colourless");

    assert_eq!(game.players[PlayerId::One.index()].mana_pool.colorless, 1);
    assert!(
        !draw_offered(&game, library),
        "and the tap it wanted is spent",
    );
}

/// Nothing on the ability says when: with a hand of exactly seven it draws
/// on their turn as readily as on yours, which is how the seventh card is
/// held until their end step.
#[test]
fn the_draw_may_be_taken_on_their_turn() {
    let (mut game, libraries) = staged(1, 7);
    let library = libraries[0];
    game.active_player = PlayerId::Two;
    game.step = Step::End;
    game.priority = PlayerId::One;

    assert!(
        draw_offered(&game, library),
        "their end step is as good a time as any",
    );
    let action = draw_action(&game, library);
    game.apply(PlayerId::One, action).expect("it activates");
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].hand.len(),
        8,
        "the card is drawn on their turn",
    );
    assert!(
        !draw_offered(&game, library),
        "and eight is not seven, so it says nothing until one goes",
    );
}

/// The counterpart to that ruling, and the reason it is worth knowing:
/// announced one at a time rather than in response, the first draw resolves
/// into a hand of eight and the second Library has nothing to announce.
#[test]
fn a_second_library_announced_after_the_first_resolved_finds_eight() {
    let (mut game, libraries) = staged(2, 7);

    let first = draw_action(&game, libraries[0]);
    game.apply(PlayerId::One, first).expect("seven in hand");
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    assert_eq!(game.players[0].hand.len(), 8, "the first one resolved");
    assert!(
        !draw_offered(&game, libraries[1]),
        "and eight is not exactly seven, so the second may not be announced",
    );
}

/// A land has no summoning sickness, so a Library played into a hand of
/// seven draws the turn it comes down -- which, the land drop having left
/// the hand, is what makes the seventh card the one that matters.
#[test]
fn a_library_played_this_turn_draws_at_once() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..8 {
        game.players[0]
            .library
            .push(card(128_400 + index, cards::ISLAND, PlayerId::One));
    }
    // Seven in hand plus the Library itself: playing it leaves exactly seven.
    for index in 0..7 {
        game.players[0].hand.push(card(
            128_500 + u32::try_from(index).expect("a small hand"),
            cards::MOUNTAIN,
            PlayerId::One,
        ));
    }
    let held = card(128_600, cards::LIBRARY_OF_ALEXANDRIA, PlayerId::One);
    let held_id = held.id;
    game.players[0].hand.push(held);
    game.players[0].lands_played_this_turn = 0;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == held_id))
        .expect("a land drop is available");
    game.apply(PlayerId::One, play).expect("it is playable");
    drain_pending(&mut game);
    let library = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::LIBRARY_OF_ALEXANDRIA)
        .expect("it was played")
        .card
        .id;

    assert_eq!(game.players[0].hand.len(), 7, "seven left behind it");
    assert!(
        draw_offered(&game, library),
        "and a land is not summoning sick, so it draws at once",
    );

    let draw = draw_action(&game, library);
    game.apply(PlayerId::One, draw).expect("it activates");
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(&mut game);

    assert_eq!(game.players[0].hand.len(), 8, "the card is drawn");
}
