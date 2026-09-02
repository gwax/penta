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

/// Resolves whatever is waiting, taking the first thing offered.
fn settle(game: &mut Game) {
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
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

/// "Target opponent reveals their hand": a hand of nothing but lands is
/// revealed and holds nothing the Sculler may take, so it stands there with
/// its hands empty and gives nothing back when it goes.
#[test]
fn a_hand_of_lands_leaves_it_holding_nothing() {
    let mut game = staged(&[cards::FOREST, cards::MOUNTAIN]);
    let sculler = game
        .put_onto_battlefield(PlayerId::One, cards::TIDEHOLLOW_SCULLER)
        .expect("cataloged");
    settle(&mut game);

    assert!(
        game.players[PlayerId::Two.index()].exile.is_empty(),
        "there was no nonland card to take",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].hand.len(),
        2,
        "and the hand it read is untouched",
    );

    game.move_permanents_to_graveyard(&[sculler]);
    game.check_state_based_actions();
    settle(&mut game);

    assert_eq!(
        game.players[PlayerId::Two.index()].hand.len(),
        2,
        "with nothing held, its leave trigger returns nothing",
    );
}

/// An empty hand is the same story with nothing even to reveal.
#[test]
fn an_empty_hand_is_read_and_leaves_it_empty_handed() {
    let mut game = staged(&[]);
    game.put_onto_battlefield(PlayerId::One, cards::TIDEHOLLOW_SCULLER)
        .expect("cataloged");
    settle(&mut game);

    assert!(
        game.players[PlayerId::Two.index()].exile.is_empty(),
        "nothing to take from a hand with nothing in it",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::TIDEHOLLOW_SCULLER),
        "and the body arrives all the same",
    );
}

/// "Reveals their hand" is a reveal, so the table sees what it read even
/// when it takes nothing away.
#[test]
fn the_hand_it_reads_is_revealed() {
    let mut game = staged(&[cards::LIGHTNING_BOLT, cards::SERRA_ANGEL]);
    game.put_onto_battlefield(PlayerId::One, cards::TIDEHOLLOW_SCULLER)
        .expect("cataloged");
    game.events.clear();

    settle(&mut game);

    assert!(
        game.events
            .iter()
            .any(|event| matches!(event, GameEvent::CardRevealed { .. })),
        "the hand was shown before anything was chosen from it",
    );
}

/// It is an artifact creature, which is what makes it answerable by the
/// removal a two-drop body would otherwise dodge.
#[test]
fn it_is_an_artifact_as_well_as_a_creature() {
    let mut game = staged(&[cards::LIGHTNING_BOLT]);
    let sculler = game
        .put_onto_battlefield(PlayerId::One, cards::TIDEHOLLOW_SCULLER)
        .expect("cataloged");
    settle(&mut game);

    let types = game
        .permanent_types(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == sculler)
                .expect("it is there"),
        )
        .expect("it has types");
    assert!(types.contains(CardType::Artifact), "an artifact");
    assert!(types.contains(CardType::Creature), "and a creature");
}

/// Two Scullers hold two different cards, and each gives back only its own.
/// What the second ability returns is the card the first one took, not
/// whatever is sitting in exile when the body dies.
#[test]
fn two_scullers_each_give_back_only_what_they_took() {
    let mut game = staged(&[cards::LIGHTNING_BOLT, cards::SERRA_ANGEL]);

    let first = game
        .put_onto_battlefield(PlayerId::One, cards::TIDEHOLLOW_SCULLER)
        .expect("cataloged");
    settle(&mut game);
    let taken_by_first = game.players[PlayerId::Two.index()]
        .exile
        .first()
        .expect("the first one took a card")
        .definition;

    let second = game
        .put_onto_battlefield(PlayerId::One, cards::TIDEHOLLOW_SCULLER)
        .expect("cataloged");
    settle(&mut game);
    let taken_by_second = game.players[PlayerId::Two.index()]
        .exile
        .iter()
        .map(|card| card.definition)
        .find(|definition| *definition != taken_by_first)
        .expect("the second one took the other card");
    assert_eq!(
        game.players[PlayerId::Two.index()].exile.len(),
        2,
        "both cards are held",
    );
    assert!(
        game.players[PlayerId::Two.index()].hand.is_empty(),
        "and their hand is empty",
    );

    game.move_permanents_to_graveyard(&[first]);
    settle(&mut game);

    let hand = |game: &Game| {
        game.players[PlayerId::Two.index()]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        hand(&game),
        vec![taken_by_first],
        "the one that died gave back its own card",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()]
            .exile
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![taken_by_second],
        "and the other Sculler is still holding the other card",
    );

    game.move_permanents_to_graveyard(&[second]);
    settle(&mut game);

    let mut back = hand(&game);
    back.sort_unstable();
    let mut both = vec![taken_by_first, taken_by_second];
    both.sort_unstable();
    assert_eq!(back, both, "and now both are home");
    assert!(game.players[PlayerId::Two.index()].exile.is_empty());
}
