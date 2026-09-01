//! Memory Jar: seven cards for everybody, on loan until the end step.

use super::*;

/// The Jar on the battlefield since last turn, with `mine` and `theirs` in
/// the two hands and libraries deep enough to draw from.
fn staged(mine: usize, theirs: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    game.players[0].library.clear();
    game.players[1].library.clear();
    for player in [PlayerId::One, PlayerId::Two] {
        for index in 0..12 {
            let id = 65_000 + u32::from(player == PlayerId::Two) * 100 + index;
            game.players[player.index()]
                .library
                .push(card(id, cards::ISLAND, player));
        }
    }
    for index in 0..mine {
        let id = 65_500 + u32::try_from(index).expect("a handful of cards");
        game.players[0]
            .hand
            .push(card(id, cards::LIGHTNING_BOLT, PlayerId::One));
    }
    for index in 0..theirs {
        let id = 65_600 + u32::try_from(index).expect("a handful of cards");
        game.players[1]
            .hand
            .push(card(id, cards::GRIZZLY_BEARS, PlayerId::Two));
    }
    let jar = game
        .put_onto_battlefield(PlayerId::One, cards::MEMORY_JAR)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, jar)
}

fn settle(game: &mut Game) {
    for _ in 0..40 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
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

/// The drawn cards that ended up in the graveyard. The Jar is in there too,
/// having sacrificed itself to do any of this.
fn discarded_islands(game: &Game) -> usize {
    game.players[0]
        .graveyard
        .iter()
        .filter(|card| card.definition == cards::ISLAND)
        .count()
}

fn crack(game: &mut Game, jar: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == jar))
        .expect("the Jar can be cracked");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(game);
}

fn end_step(game: &mut Game) {
    game.step = Step::End;
    game.begin_step_triggers();
    settle(game);
}

/// Seven each, and the old hands are gone for the turn.
#[test]
fn everybody_draws_seven() {
    let (mut game, jar) = staged(2, 3);

    crack(&mut game, jar);

    assert_eq!(game.players[0].hand.len(), 7);
    assert_eq!(game.players[1].hand.len(), 7);
    assert!(
        game.players[0]
            .hand
            .iter()
            .all(|card| card.definition == cards::ISLAND),
        "the new hand is off the top of the library",
    );
    assert_eq!(game.players[0].exile.len(), 2, "the old hand is in exile");
    assert_eq!(game.players[1].exile.len(), 3);
}

/// Face down and nobody may look, your own hand included: what both players
/// get is a count. "You can't look at the cards you exiled until they return
/// to your hand" is the ruling, and CR 713.2 is why -- a card exiled face
/// down is hidden unless something says otherwise, and the Jar says nothing.
#[test]
fn the_exiled_hands_are_face_down_to_everybody() {
    let (mut game, jar) = staged(2, 3);

    crack(&mut game, jar);

    let mine = game.observe(PlayerId::One);
    assert!(
        mine.exiles[0].is_empty(),
        "you cannot look at what you put away either",
    );
    assert!(
        mine.exiles[1].is_empty(),
        "and certainly not at what they put away",
    );
    assert_eq!(
        mine.face_down_exile_sizes,
        [2, 3],
        "both piles are counted by both players",
    );
    assert_eq!(
        game.observe(PlayerId::Two).face_down_exile_sizes,
        [2, 3],
        "which is the same count from the other seat",
    );
}

/// The end step takes the loan back: whatever is left of the seven is
/// discarded and the old hand returns.
#[test]
fn the_end_step_gives_the_old_hand_back() {
    let (mut game, jar) = staged(2, 3);
    crack(&mut game, jar);

    end_step(&mut game);

    assert_eq!(game.players[0].hand.len(), 2, "the hand it took, back");
    assert!(
        game.players[0]
            .hand
            .iter()
            .all(|card| card.definition == cards::LIGHTNING_BOLT),
        "and it is the same cards",
    );
    assert_eq!(game.players[1].hand.len(), 3);
    assert!(
        game.players[0].exile.is_empty() && game.players[1].exile.is_empty(),
        "nothing is left in exile",
    );
    assert_eq!(
        discarded_islands(&game),
        7,
        "the seven unspent cards were discarded",
    );
}

/// A card spent during the turn is not there to be discarded, which is the
/// whole reason to crack it: the loan is only taken back from what is left.
#[test]
fn what_you_spent_is_yours() {
    let (mut game, jar) = staged(1, 0);
    crack(&mut game, jar);
    // Spend one of the seven the ordinary way.
    let island = game.players[0].hand[0].id;
    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == island))
        .expect("a land in hand is playable");
    game.apply(PlayerId::One, play).expect("it is played");
    settle(&mut game);

    end_step(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::ISLAND),
        "the land stayed where it was played",
    );
    assert_eq!(game.players[0].hand.len(), 1, "and the old hand came back");
    assert_eq!(
        discarded_islands(&game),
        6,
        "six of the seven were left to throw away",
    );
}

/// An empty hand exiles nothing and still draws seven, which is how the Jar
/// is usually cracked.
#[test]
fn an_empty_hand_still_draws() {
    let (mut game, jar) = staged(0, 0);

    crack(&mut game, jar);

    assert_eq!(game.players[0].hand.len(), 7);
    assert!(game.players[0].exile.is_empty(), "nothing to put away");

    end_step(&mut game);
    assert!(
        game.players[0].hand.is_empty(),
        "and nothing to come back to",
    );
}

/// "Each player ... draws seven cards" is not optional and not yours alone:
/// a Jar cracked against a library three deep is a Jar that kills its
/// opponent, once the state-based check catches up.
#[test]
fn it_draws_the_other_player_out_of_their_library() {
    let (mut game, jar) = staged(1, 1);
    game.players[1].library.truncate(3);

    crack(&mut game, jar);
    game.check_state_based_actions();

    assert_eq!(
        game.players[0].hand.len(),
        7,
        "you drew your seven off a library that had them",
    );
    assert!(
        game.players[1].library.is_empty(),
        "and they emptied theirs trying",
    );
    assert_eq!(
        game.result,
        Some(GameResult::Winner {
            winner: PlayerId::One,
            reason: WinReason::OpponentTriedToDrawFromEmptyLibrary,
        }),
        "which is a loss for the player who could not finish the draw",
    );
}

/// "At the beginning of the next end step" is the next one there is, not the
/// next one of yours: a Jar cracked on their turn is settled at the end of
/// that turn.
#[test]
fn cracked_on_their_turn_it_settles_at_the_end_of_theirs() {
    let (mut game, jar) = staged(2, 2);
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    crack(&mut game, jar);
    assert_eq!(game.players[0].hand.len(), 7, "seven for the cracker");
    assert_eq!(game.players[1].hand.len(), 7, "and seven for them");

    end_step(&mut game);

    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .filter(|card| card.definition == cards::LIGHTNING_BOLT)
            .count(),
        2,
        "their end step gave the old hand back",
    );
    assert_eq!(
        game.players[1]
            .hand
            .iter()
            .filter(|card| card.definition == cards::GRIZZLY_BEARS)
            .count(),
        2,
        "to both players",
    );
    assert_eq!(discarded_islands(&game), 7, "and took the loan back");
}

/// The delayed trigger is a one-shot: it settles the loan at the next end
/// step and is not there at the one after.
#[test]
fn the_delayed_trigger_fires_once_and_no_more() {
    let (mut game, jar) = staged(2, 0);
    crack(&mut game, jar);
    end_step(&mut game);
    let hand = game.players[0]
        .hand
        .iter()
        .map(|card| card.definition)
        .collect::<Vec<_>>();
    assert_eq!(hand.len(), 2, "the old hand came back");
    let buried = discarded_islands(&game);

    // A fresh turn, and its end step.
    game.turns_started[PlayerId::One.index()] += 1;
    game.step = Step::PrecombatMain;
    end_step(&mut game);

    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        hand,
        "the second end step took nothing and gave nothing",
    );
    assert_eq!(discarded_islands(&game), buried, "and buried nothing more");
}
