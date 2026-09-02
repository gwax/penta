//! Narset, Parter of Veils: she finds the spell the deck is built around
//! and turns every draw spell the other player has into one card.

use super::*;

/// Narset on the battlefield under Player One, with `library` on top of
/// their library -- the last entry is the top card.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[1].library.clear();
    for index in 0..6 {
        game.players[1]
            .library
            .push(card(115_100 + index, cards::ISLAND, PlayerId::Two));
    }
    for definition in library {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].library.push(card);
    }
    let narset = game
        .put_onto_battlefield(PlayerId::One, cards::NARSET_PARTER_OF_VEILS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, narset)
}

/// Activates the minus, taking the first card offered when `take` is set.
fn dig(game: &mut Game, narset: GameObjectId, take: bool) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == narset),
        )
        .expect("the minus is offered");
    game.apply(PlayerId::One, action).expect("it activates");
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            // A look that found nothing takeable still offers its one
            // option, and taking it is not allowed: the maximum is what
            // says how much of the offer may actually be selected.
            let options = if take {
                decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(decision.maximum.min(1))
                    .collect()
            } else {
                Vec::new()
            };
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
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
    drain_pending(game);
}

/// Their first draw of the turn lands and the rest do not.
#[test]
fn an_opponent_draws_only_one_card_a_turn() {
    let (mut game, _) = staged(&[]);
    let library = game.players[1].library.len();

    game.draw_cards(PlayerId::Two, 1);
    assert_eq!(game.players[1].hand.len(), 1, "the first one lands");

    game.draw_cards(PlayerId::Two, 3);

    assert_eq!(game.players[1].hand.len(), 1, "and no more do");
    assert_eq!(
        game.players[1].library.len(),
        library - 1,
        "the cards stay in the library rather than being lost",
    );
}

/// It binds them and not you.
#[test]
fn you_draw_as_many_as_you_like() {
    let (mut game, _) = staged(&[cards::ISLAND, cards::ISLAND, cards::ISLAND, cards::ISLAND]);

    game.draw_cards(PlayerId::One, 3);

    assert_eq!(
        game.players[0].hand.len(),
        3,
        "your own draws are untouched"
    );
}

/// The minus finds a noncreature, nonland card and buries the rest.
#[test]
fn the_minus_finds_a_spell() {
    let (mut game, narset) = staged(&[
        cards::ISLAND,
        cards::GRIZZLY_BEARS,
        cards::MOUNTAIN,
        cards::ANCESTRAL_RECALL,
    ]);

    dig(&mut game, narset, true);

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::ANCESTRAL_RECALL),
        "the only spell among the four is in hand",
    );
    assert_eq!(game.players[0].hand.len(), 1, "and only that one");
    assert_eq!(
        game.players[0].library.len(),
        3,
        "the other three went to the bottom",
    );
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == narset)
            .expect("she is there")
            .counters(CounterKind::Loyalty),
        3,
        "five minus two",
    );
}

/// Creatures and lands are not what she is looking for.
#[test]
fn creatures_and_lands_stay_buried() {
    let (mut game, narset) = staged(&[
        cards::ISLAND,
        cards::GRIZZLY_BEARS,
        cards::MOUNTAIN,
        cards::SAVANNAH_LIONS,
    ]);

    dig(&mut game, narset, true);

    assert!(
        game.players[0].hand.is_empty(),
        "there was nothing she could take",
    );
    assert_eq!(game.players[0].library.len(), 4, "all four went back down");
}

/// "You may reveal": taking nothing is a legal answer.
#[test]
fn she_may_take_nothing() {
    let (mut game, narset) = staged(&[
        cards::ISLAND,
        cards::ISLAND,
        cards::ISLAND,
        cards::ANCESTRAL_RECALL,
    ]);

    dig(&mut game, narset, false);

    assert!(game.players[0].hand.is_empty(), "she declined the card");
    assert_eq!(game.players[0].library.len(), 4);
}

/// "If a spell or ability instructs that player to draw multiple cards, that
/// player will just draw one card." A single instruction to draw three is
/// not blocked outright: the first card lands and the other two do not.
#[test]
fn a_multiple_draw_becomes_a_single_one() {
    let (mut game, _) = staged(&[]);
    let library = game.players[PlayerId::Two.index()].library.len();

    game.draw_cards(PlayerId::Two, 3);

    assert_eq!(
        game.players[PlayerId::Two.index()].hand.len(),
        1,
        "one of the three arrived",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].library.len(),
        library - 1,
        "and the other two never left the library",
    );
}

/// "Your opponents can each draw a maximum of one card each on each player's
/// turn." It is a limit per turn rather than a limit for the game, so the
/// next turn hands them another one.
#[test]
fn the_limit_starts_over_each_turn() {
    let (mut game, _) = staged(&[]);
    game.draw_cards(PlayerId::Two, 2);
    assert_eq!(
        game.players[PlayerId::Two.index()].hand.len(),
        1,
        "one this turn",
    );

    game.commit_next_turn(PlayerId::Two, Vec::new());
    drain_pending(&mut game);

    let held = game.players[PlayerId::Two.index()].hand.len();
    game.draw_cards(PlayerId::Two, 2);
    assert_eq!(
        game.players[PlayerId::Two.index()].hand.len(),
        held + 1,
        "and one more on the next turn, no more and no fewer",
    );
}

/// "Narset will 'see' cards drawn by opponents earlier in the turn she
/// entered the battlefield." Two cards already drawn is already over the
/// limit, so she shuts the door behind them without taking anything back.
#[test]
fn she_counts_draws_from_before_she_arrived() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    game.players[PlayerId::Two.index()].library.clear();
    for index in 0..6 {
        game.players[PlayerId::Two.index()].library.push(card(
            115_300 + index,
            cards::ISLAND,
            PlayerId::Two,
        ));
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    // Two cards drawn before she is anywhere near the battlefield.
    game.draw_cards(PlayerId::Two, 2);
    assert_eq!(
        game.players[PlayerId::Two.index()].hand.len(),
        2,
        "both land while nothing is stopping them",
    );

    game.put_onto_battlefield(PlayerId::One, cards::NARSET_PARTER_OF_VEILS)
        .expect("cataloged");
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::Two.index()].hand.len(),
        2,
        "the two already drawn are not taken back",
    );

    game.draw_cards(PlayerId::Two, 1);

    assert_eq!(
        game.players[PlayerId::Two.index()].hand.len(),
        2,
        "but the turn's one is long spent, so nothing more arrives",
    );
}
