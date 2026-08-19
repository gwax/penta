//! Creatures that reach into an opponent's hand.
//!
//! Both of these take a card and hold it rather than destroying it, so what
//! each test has to establish twice over is which cards were eligible and
//! that the held card comes back.

use super::*;

/// The Freebooter takes the answer rather than the threat: a creature card is
/// never eligible, and neither is a land. And "until this creature leaves the
/// battlefield" is one printed ability, so the return rides on the same
/// resolution rather than on a second clause.
#[test]
fn the_freebooter_takes_a_noncreature_nonland_card_and_gives_it_back() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    for (instance, definition) in [
        (71_000, cards::LIGHTNING_BOLT),
        (71_001, cards::ANCESTRAL_RECALL),
        (71_002, cards::SERRA_ANGEL),
        (71_003, cards::FOREST),
    ] {
        game.players[PlayerId::Two.index()]
            .hand
            .push(card(instance, definition, PlayerId::Two));
    }

    let freebooter = game
        .put_onto_battlefield(PlayerId::One, cards::KITESAIL_FREEBOOTER)
        .expect("cataloged");

    let mut offered = Vec::new();
    for _ in 0..8 {
        if let Some(decision) = game.observe(PlayerId::One).decision {
            let cards = decision
                .options
                .iter()
                .filter_map(|option| option.card.map(|(_, definition)| definition))
                .collect::<Vec<_>>();
            if !cards.is_empty() {
                offered = cards;
            }
            game.apply(
                PlayerId::One,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: vec![decision.options[0].id],
                },
            )
            .expect("the offered choice is legal");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        assert!(
            game.apply(player, Action::PassPriority).is_ok(),
            "the enters trigger is waiting",
        );
    }
    offered.sort_unstable();
    let mut expected = vec![cards::LIGHTNING_BOLT, cards::ANCESTRAL_RECALL];
    expected.sort_unstable();
    assert_eq!(
        offered, expected,
        "the Angel is a creature and the Forest is a land",
    );

    assert_eq!(game.players[PlayerId::Two.index()].exile.len(), 1);
    let held = game.players[PlayerId::Two.index()].exile[0].definition;

    game.move_permanents_to_graveyard(&[freebooter]);
    drain_pending(&mut game);

    assert!(
        game.players[PlayerId::Two.index()]
            .hand
            .iter()
            .any(|card| card.definition == held),
        "the card comes back when the body goes",
    );
    assert!(game.players[PlayerId::Two.index()].exile.is_empty());
}

/// The Squadron grows every creature token that arrives, whoever made it and
/// whatever made it -- but only tokens, and only yours.
#[test]
fn the_squadron_puts_a_counter_on_every_creature_token_you_control() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.put_onto_battlefield(PlayerId::One, cards::SECURITRON_SQUADRON)
        .expect("cataloged");
    drain_pending(&mut game);

    let size_of = |game: &Game, definition| {
        let permanent = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.definition == definition)
            .expect("the token arrived");
        (game.power(permanent), game.toughness(permanent))
    };

    game.create_token(PlayerId::One, cards::BEAST_TOKEN_3_3_GREEN);
    drain_pending(&mut game);
    assert_eq!(
        size_of(&game, cards::BEAST_TOKEN_3_3_GREEN),
        (Some(4), Some(4)),
        "a 3/3 token arrives and is grown",
    );

    // A Food token is a token but not a creature.
    game.create_token(PlayerId::One, cards::FOOD_TOKEN);
    drain_pending(&mut game);
    let food = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::FOOD_TOKEN)
        .expect("the Food arrived");
    assert_eq!(food.counters(CounterKind::PlusOnePlusOne), 0);

    // An opponent's token is not one you control.
    game.create_token(PlayerId::Two, cards::KNIGHT_TOKEN_2_2_WHITE);
    drain_pending(&mut game);
    assert_eq!(
        size_of(&game, cards::KNIGHT_TOKEN_2_2_WHITE),
        (Some(2), Some(2)),
        "and the other player's token is untouched",
    );
}
