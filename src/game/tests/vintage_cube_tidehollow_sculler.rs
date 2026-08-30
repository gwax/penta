//! Tidehollow Sculler and the card it is holding.
//!
//! The ordinary case -- take a card, give it back when the body goes -- is
//! covered in `vintage_cube_creatures`. What lives here is the case where
//! the body goes while the arrival trigger is still waiting.

use super::*;

/// The Sculler about to arrive, with `theirs` in the other hand.
fn staged(theirs: &[CardDefinitionId]) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    for (index, definition) in theirs.iter().enumerate() {
        let id = 67_000 + u32::try_from(index).expect("few cards");
        game.players[PlayerId::Two.index()]
            .hand
            .push(card(id, *definition, PlayerId::Two));
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game
}

/// Answers what is waiting without letting the stack resolve, which is the
/// difference that matters here: the trigger has to be sitting on the stack,
/// targeted, when the Sculler is answered.
fn answer_without_resolving(game: &mut Game) {
    for _ in 0..8 {
        let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        else {
            return;
        };
        let options = decision
            .options
            .iter()
            .map(|option| option.id)
            .take(decision.minimum.max(1).min(decision.maximum))
            .collect::<Vec<_>>();
        game.apply(
            decision.player,
            Action::ChooseDecision {
                decision: decision.id,
                options,
            },
        )
        .expect("the offered choice is legal");
    }
}

/// "If Tidehollow Sculler leaves the battlefield before its first ability has
/// resolved, its second ability triggers. This ability does nothing when it
/// resolves. Then its first ability resolves and exiles the chosen card
/// indefinitely." Killing it in response is the worst answer to it there is:
/// the card that comes back is the one nothing has taken yet.
#[test]
fn killing_it_in_response_exiles_the_card_for_good() {
    let mut game = staged(&[cards::LIGHTNING_BOLT]);
    let sculler = game
        .put_onto_battlefield(PlayerId::One, cards::TIDEHOLLOW_SCULLER)
        .expect("cataloged");
    game.begin_trigger_placement();
    answer_without_resolving(&mut game);
    assert_eq!(
        game.stack.len(),
        1,
        "the arrival trigger is on the stack, targeted and unresolved",
    );
    assert!(
        game.players[PlayerId::Two.index()].exile.is_empty(),
        "and has taken nothing yet",
    );

    // The removal resolves first. Its leaves trigger goes on top of the
    // arrival trigger, so it is the one that resolves first, with no
    // exiled card to give back.
    game.move_permanents_to_graveyard(&[sculler]);
    game.check_state_based_actions();
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == sculler),
        "the Sculler is gone",
    );
    assert!(
        game.players[PlayerId::Two.index()]
            .exile
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "and the card it took on the way out stays exiled: {:?}",
        game.players[PlayerId::Two.index()]
            .exile
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
    );
    assert!(
        game.players[PlayerId::Two.index()].hand.is_empty(),
        "with nothing returned to the hand it came from",
    );
}
