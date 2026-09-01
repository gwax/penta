//! Dark Confidant: an extra card every upkeep, at whatever the top of the
//! library happens to cost.

use super::*;

/// Player One with the Confidant out and `library` stacked so the last
/// entry is on top.
fn staged(library: &[CardDefinitionId]) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for definition in library {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].library.push(card);
    }
    game.put_onto_battlefield(PlayerId::One, cards::DARK_CONFIDANT)
        .expect("cataloged");
    drain_pending(&mut game);
    game.players[0].life = 20;
    game
}

/// Runs the upkeep trigger of whoever's turn it is now.
fn take_upkeep(game: &mut Game, player: PlayerId) {
    game.active_player = player;
    game.step = Step::Upkeep;
    game.priority = player;
    game.handle_upkeep_triggers();
    for _ in 0..16 {
        if let Some(pending) = game.pending_decisions.first() {
            let decision = pending.observation.clone();
            let options = decision
                .options
                .iter()
                .map(|option| option.id)
                .take(decision.minimum.max(1))
                .collect();
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
    game.check_state_based_actions();
}

/// The card comes off the top and its cost comes off your life total.
#[test]
fn the_upkeep_card_costs_its_mana_value() {
    let mut game = staged(&[cards::GRIZZLY_BEARS]);

    take_upkeep(&mut game, PlayerId::One);

    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::GRIZZLY_BEARS],
    );
    assert_eq!(game.players[0].life, 18, "a two-drop costs two life");
    assert!(game.players[0].library.is_empty());
}

/// A land is free, which is the whole reason the decks that play him keep
/// their curve down.
#[test]
fn a_land_costs_nothing() {
    let mut game = staged(&[cards::MOUNTAIN]);

    take_upkeep(&mut game, PlayerId::One);

    assert_eq!(game.players[0].life, 20);
    assert_eq!(game.players[0].hand.len(), 1);
}

/// "Your upkeep": the other player's does nothing.
#[test]
fn he_is_silent_on_their_upkeep() {
    let mut game = staged(&[cards::GRIZZLY_BEARS]);

    take_upkeep(&mut game, PlayerId::Two);

    assert_eq!(game.players[0].life, 20);
    assert!(game.players[0].hand.is_empty());
    assert_eq!(game.players[0].library.len(), 1);
}

/// "If a card in a player's library has {X} in its mana cost, X is 0."
/// A Braingeyser costs its owner two life off the top, however many cards
/// it would have drawn had it been cast.
#[test]
fn an_x_spell_counts_its_x_as_zero() {
    let mut game = staged(&[cards::BRAINGEYSER]);

    take_upkeep(&mut game, PlayerId::One);

    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::BRAINGEYSER],
    );
    assert_eq!(
        game.players[0].life, 18,
        "{{X}}{{U}}{{U}} is a mana value of two in the library",
    );
}

/// The life is lost, not paid, and nothing stops it: a Confidant flipping
/// something expensive at a low enough life total kills his own controller.
#[test]
fn he_will_kill_his_controller() {
    let mut game = staged(&[cards::SERRA_ANGEL]);
    game.players[0].life = 4;

    take_upkeep(&mut game, PlayerId::One);

    assert_eq!(game.players[0].life, -1, "a five-drop off four life");
    assert!(
        game.result.is_some(),
        "which is a loss, checked as a state-based action",
    );
}

/// A split card off the top is worth both halves anywhere but the stack, so
/// a Life // Death costs its {G} and its {1}{B} together: three life for
/// one card.
#[test]
fn a_split_card_costs_both_of_its_halves() {
    let mut game = staged(&[cards::LIFE_DEATH]);

    take_upkeep(&mut game, PlayerId::One);

    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::LIFE_DEATH],
        "the card came to hand",
    );
    assert_eq!(
        game.players[0].life, 17,
        "one for the Life and two for the Death",
    );
}

/// With nothing to reveal there is nothing to put into your hand and
/// nothing to pay for. The draw step is what an empty library punishes, and
/// that is not this trigger's business.
#[test]
fn an_empty_library_reveals_nothing_and_costs_nothing() {
    let mut game = staged(&[]);

    take_upkeep(&mut game, PlayerId::One);

    assert!(game.players[0].hand.is_empty(), "nothing was revealed");
    assert_eq!(game.players[0].life, 20, "and nothing was paid for it");
    assert!(
        game.result().is_none(),
        "an empty library is not by itself a loss",
    );
}

/// "Reveal the top card of your library": a reveal, so the table sees what
/// he turned up before it goes anywhere.
#[test]
fn the_card_is_revealed_on_the_way_to_your_hand() {
    let mut game = staged(&[cards::LIGHTNING_BOLT]);
    game.events.clear();

    take_upkeep(&mut game, PlayerId::One);

    assert_eq!(
        game.events
            .iter()
            .filter(|event| matches!(event, GameEvent::CardRevealed { .. }))
            .count(),
        1,
        "one card, shown once",
    );
    assert_eq!(game.players[0].hand.len(), 1, "and then it is yours");
}

/// "Put that card into your hand" is not a draw, so nothing that watches
/// draws sees this one. The draw taken afterwards, off the same library,
/// is what a draw looks like by comparison.
#[test]
fn putting_the_card_into_your_hand_is_not_a_draw() {
    let mut game = staged(&[cards::LIGHTNING_BOLT, cards::LIGHTNING_BOLT]);
    game.cards_drawn_this_turn = [0; 2];

    take_upkeep(&mut game, PlayerId::One);

    assert_eq!(game.players[0].hand.len(), 1, "the card arrived");
    assert_eq!(
        game.cards_drawn_this_turn[0], 0,
        "and no draw was taken for it",
    );

    game.draw_instruction(PlayerId::One, 1);
    drain_pending(&mut game);

    assert_eq!(
        game.cards_drawn_this_turn[0], 1,
        "which is what the counter does record",
    );
}

/// Two Confidants are two triggers: two cards off the top and both their
/// costs, which is how the card kills the player who overcommitted to it.
#[test]
fn a_second_confidant_is_a_second_trigger() {
    let mut game = staged(&[cards::LIGHTNING_BOLT, cards::LIGHTNING_BOLT]);
    game.put_onto_battlefield(PlayerId::One, cards::DARK_CONFIDANT)
        .expect("cataloged");
    drain_pending(&mut game);
    game.players[0].life = 20;

    take_upkeep(&mut game, PlayerId::One);

    assert_eq!(game.players[0].hand.len(), 2, "one card apiece");
    assert!(game.players[0].library.is_empty(), "both came off the top");
    assert_eq!(game.players[0].life, 18, "and a life apiece");
}

/// The trigger is on the stack in its own right: a Confidant answered in
/// response is a Confidant who still turns the card over and still charges
/// for it.
#[test]
fn killing_him_in_response_does_not_stop_the_trigger() {
    let mut game = staged(&[cards::SERRA_ANGEL]);
    let confidant = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::DARK_CONFIDANT)
        .expect("he is out")
        .card
        .id;
    game.active_player = PlayerId::One;
    game.step = Step::Upkeep;
    game.priority = PlayerId::One;
    game.handle_upkeep_triggers();
    for _ in 0..8 {
        if !game.stack.is_empty() {
            break;
        }
        if game.apply(game.priority, Action::PassPriority).is_err() {
            break;
        }
    }
    assert!(!game.stack.is_empty(), "his trigger is waiting");

    game.destroy_permanent(confidant);
    game.check_state_based_actions();
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == confidant),
        "he is gone before it resolves",
    );

    for _ in 0..16 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        if game.apply(game.priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();

    assert_eq!(game.players[0].hand.len(), 1, "the Angel came over anyway");
    assert_eq!(game.players[0].life, 15, "and cost her five all the same");
}
