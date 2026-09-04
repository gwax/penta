//! Crabomination: one card out of each of three zones, and a single free
//! cast spread across the pile.

use super::*;

/// Player Two holding one card in each of the three zones the pile draws
/// from, so every pick has exactly one legal answer to find.
fn staged() -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    for player in &mut game.players {
        player.hand.clear();
        player.library.clear();
        player.graveyard.clear();
        player.exile.clear();
    }
    for (zone, definition) in [
        (ZoneKind::Library, cards::GRIZZLY_BEARS),
        (ZoneKind::Graveyard, cards::LIGHTNING_BOLT),
        (ZoneKind::Hand, cards::ISLAND),
    ] {
        let card = game
            .build_zone(PlayerId::Two, &[definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        match zone {
            ZoneKind::Library => game.players[1].library.push(card),
            ZoneKind::Graveyard => game.players[1].graveyard.push(card),
            _ => game.players[1].hand.push(card),
        }
    }
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game
}

/// Answers every open decision, declining any offer so the pile is only
/// exiled and nothing is cast out of it.
fn settle_declining(game: &mut Game) {
    for _ in 0..32 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .find(|option| option.label == "Decline")
                .or_else(|| decision.options.first())
                .map(|option| vec![option.id])
                .unwrap_or_default();
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
        if game.stack.is_empty()
            && game.pending_triggers.is_empty()
            && game.pending_decisions.is_empty()
        {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(game);
    game.check_state_based_actions();
}

/// One card leaves each of the three zones, and all three land in exile.
#[test]
fn the_pile_takes_one_card_from_each_zone() {
    let mut game = staged();

    game.put_onto_battlefield(PlayerId::One, cards::CRABOMINATION)
        .expect("cataloged");
    settle_declining(&mut game);

    assert_eq!(game.players[1].library.len(), 0, "the top card of library");
    assert_eq!(game.players[1].graveyard.len(), 0, "one from the graveyard");
    assert_eq!(game.players[1].hand.len(), 0, "one from the hand");
    assert_eq!(
        game.players[1].exile.len(),
        3,
        "all three go to the same pile",
    );
}

/// The pile is drawn from the targeted opponent, not from its controller.
#[test]
fn the_controllers_own_zones_are_untouched() {
    let mut game = staged();
    let card = game
        .build_zone(PlayerId::One, &[cards::ISLAND])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    game.players[0].hand.push(card);

    game.put_onto_battlefield(PlayerId::One, cards::CRABOMINATION)
        .expect("cataloged");
    settle_declining(&mut game);

    assert_eq!(game.players[0].hand.len(), 1, "his own hand is left alone");
    assert_eq!(game.players[0].exile.len(), 0, "and nothing of his exiled");
}
