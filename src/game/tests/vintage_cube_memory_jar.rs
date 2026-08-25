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

/// Face down: each player sees their own exiled cards and counts the other
/// player's without seeing them.
#[test]
fn the_exiled_hands_are_face_down() {
    let (mut game, jar) = staged(2, 3);

    crack(&mut game, jar);

    let mine = game.observe(PlayerId::One);
    assert_eq!(mine.exiles[0].len(), 2, "you can see what you put away");
    assert!(
        mine.exiles[1].is_empty(),
        "and cannot see what they put away",
    );
    assert_eq!(
        mine.face_down_exile_sizes[1], 3,
        "only how much of it there was",
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
