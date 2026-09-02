//! Loran of the Third Path: an artifact answer stapled to a body, and a
//! symmetrical draw that is only symmetrical on paper.

use super::*;

fn staged(board: &[(CardDefinitionId, PlayerId)]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[1].library.clear();
    for index in 0..4 {
        game.players[0]
            .library
            .push(card(103_000 + index, cards::GRIZZLY_BEARS, PlayerId::One));
        game.players[1]
            .library
            .push(card(103_100 + index, cards::SAVANNAH_LIONS, PlayerId::Two));
    }
    let mut ids = Vec::new();
    for (index, (definition, controller)) in board.iter().enumerate() {
        let permanent = creature(
            103_200 + u32::try_from(index).expect("few permanents"),
            *definition,
            *controller,
        );
        ids.push(permanent.card.id);
        game.battlefield.push(permanent);
    }
    let loran = game
        .put_onto_battlefield(PlayerId::One, cards::LORAN_OF_THE_THIRD_PATH)
        .expect("cataloged");
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, loran, ids)
}

/// Answers the entry trigger by naming `target`, or nobody when `None`.
fn settle(game: &mut Game, target: Option<GameObjectId>) {
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = match target {
                Some(wanted) => decision
                    .options
                    .iter()
                    .filter(|option| option.card.is_some_and(|(object, _)| object == wanted))
                    .map(|option| option.id)
                    .take(1)
                    .collect(),
                None => Vec::new(),
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
}

/// Arriving answers an artifact.
#[test]
fn arriving_destroys_an_artifact() {
    let (mut game, _, ids) = staged(&[(cards::MOX_JET, PlayerId::Two)]);
    let mox = ids[0];

    settle(&mut game, Some(mox));

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != mox),
    );
}

/// "Up to one": with nothing worth answering it simply arrives.
#[test]
fn it_may_answer_nothing() {
    let (mut game, loran, _) = staged(&[(cards::MOX_JET, PlayerId::Two)]);

    settle(&mut game, None);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::MOX_JET),
        "nothing was named",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == loran),
    );
}

/// Tapping draws for both, and leaves it tapped -- but vigilance means
/// attacking never costs the draw.
#[test]
fn tapping_draws_for_both_players() {
    let (mut game, loran, _) = staged(&[]);
    settle(&mut game, None);
    assert!(
        game.permanent_has_executable_keyword(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == loran)
                .expect("she is there"),
            KeywordAbility::Vigilance,
        )
    );

    let draw = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == loran))
        .expect("tapping is the whole cost");
    game.apply(PlayerId::One, draw).expect("it activates");
    settle(&mut game, None);

    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::GRIZZLY_BEARS],
    );
    assert_eq!(
        game.players[1]
            .hand
            .iter()
            .filter(|card| card.definition == cards::SAVANNAH_LIONS)
            .count(),
        1,
        "and the opponent it named draws too",
    );
}

/// The half the file's own comment claims and never shows: vigilance means
/// she can attack and still tap for the draw, which is why the two abilities
/// sit on the same card.
#[test]
fn she_attacks_and_still_taps_for_the_draw() {
    let (mut game, loran, _) = staged(&[]);
    settle(&mut game, None);
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;

    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: loran,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("a 2/1 may attack");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == loran)
            .expect("she is attacking")
            .tapped,
        "vigilance kept her untapped",
    );
    let draw = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == loran))
        .expect("so the tap is still there to spend");
    game.apply(PlayerId::One, draw).expect("it activates");
    settle(&mut game, None);

    assert_eq!(game.players[0].hand.len(), 1, "you drew");
    assert_eq!(
        game.players[1]
            .hand
            .iter()
            .filter(|card| card.definition == cards::SAVANNAH_LIONS)
            .count(),
        1,
        "and so did the opponent, in the middle of being attacked",
    );
}

/// "Artifact or enchantment": the other half of the entry trigger.
#[test]
fn arriving_answers_an_enchantment_too() {
    let (mut game, _loran, board) = staged(&[(cards::EXPLORATION, PlayerId::Two)]);

    settle(&mut game, Some(board[0]));

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == board[0]),
        "the enchantment was destroyed",
    );
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::EXPLORATION),
        "and it is in its owner's graveyard",
    );
}

/// Runs until the entry trigger asks, and reports what it offers.
fn offered_targets(game: &mut Game) -> Vec<GameObjectId> {
    for _ in 0..16 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the trigger asks")
        .options
        .iter()
        .filter_map(|option| option.card.map(|(object, _)| object))
        .collect()
}

/// "Destroy", which is a word an indestructible artifact ignores. Naming the
/// Plate is legal -- the trigger restricts what it may point at by type and
/// nothing else -- and it is still there afterwards.
#[test]
fn an_indestructible_artifact_survives_being_named() {
    let (mut game, _loran, board) = staged(&[(cards::DARKSTEEL_PLATE, PlayerId::Two)]);
    let plate = board[0];

    assert!(
        offered_targets(&mut game).contains(&plate),
        "indestructible is not untargetable",
    );
    settle(&mut game, Some(plate));

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == plate),
        "destroying it is what fails, not naming it",
    );
}

/// "You and target opponent each draw a card": your own draw is not a
/// target, and the one target there is cannot be you.
#[test]
fn the_draw_names_an_opponent_and_never_you() {
    let (mut game, loran, _) = staged(&[]);
    settle(&mut game, None);

    let named = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } if source == loran => Some(targets),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(named.len(), 1, "one opponent is one way to activate her");
    assert_eq!(
        named[0]
            .iter()
            .flat_map(TargetSelection::targets)
            .collect::<Vec<_>>(),
        vec![&Target::Player(PlayerId::Two)],
        "the opponent is named and you are not",
    );
}

/// The half of the symmetry that is not symmetrical at all: against an empty
/// library, the card she hands the opponent is the one they cannot draw.
#[test]
fn the_shared_draw_can_be_the_thing_that_kills() {
    let (mut game, loran, _) = staged(&[]);
    settle(&mut game, None);
    game.players[1].library.clear();

    let draw = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == loran))
        .expect("tapping is the whole cost");
    game.apply(PlayerId::One, draw).expect("it activates");
    settle(&mut game, None);
    game.check_state_based_actions();

    assert_eq!(
        game.result,
        Some(GameResult::Winner {
            winner: PlayerId::One,
            reason: WinReason::OpponentTriedToDrawFromEmptyLibrary,
        }),
        "she drew her own card and they could not draw theirs",
    );
}

/// "Target artifact or enchantment" names no controller: one of yours is as
/// nameable as one of theirs, which is the only way she answers a Mox you
/// no longer want.
#[test]
fn her_entry_trigger_will_name_your_own_artifact() {
    let (mut game, _loran, board) = staged(&[(cards::MOX_JET, PlayerId::One)]);
    let mine = board[0];

    assert!(
        offered_targets(&mut game).contains(&mine),
        "your own artifact is on the menu",
    );
    settle(&mut game, Some(mine));

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == mine),
        "and naming it destroys it",
    );
}

/// The other end of the same asymmetry: the draw she gives you is a draw
/// like any other, so an empty library of your own is what the tap costs.
#[test]
fn the_shared_draw_can_be_the_thing_that_kills_you() {
    let (mut game, loran, _) = staged(&[]);
    settle(&mut game, None);
    game.players[0].library.clear();

    let draw = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == loran))
        .expect("tapping is the whole cost");
    game.apply(PlayerId::One, draw).expect("it activates");
    settle(&mut game, None);
    game.check_state_based_actions();

    assert_eq!(
        game.result,
        Some(GameResult::Winner {
            winner: PlayerId::Two,
            reason: WinReason::OpponentTriedToDrawFromEmptyLibrary,
        }),
        "they drew theirs and she could not draw hers",
    );
}

/// Her draw is a creature's tap ability, so it waits a turn like any other.
/// Vigilance is about not tapping to attack and does nothing for summoning
/// sickness: the turn she lands she neither attacks nor draws.
#[test]
fn the_draw_waits_a_turn_like_any_tap_ability() {
    fn her(game: &Game, loran: GameObjectId) -> &Permanent {
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == loran)
            .expect("she is there")
    }

    let (mut game, loran, _) = staged(&[]);
    settle(&mut game, None);
    let arrived = game.turns_started[PlayerId::One.index()];
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == loran)
        .expect("she is there")
        .entered_controller_turn = arrived;

    let taps = |game: &Game| {
        game.legal_actions(PlayerId::One).into_iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if source == loran),
        )
    };

    assert!(
        game.permanent_has_executable_keyword(her(&game, loran), KeywordAbility::Vigilance),
        "she has vigilance the turn she lands",
    );
    assert!(
        !taps(&game),
        "and still cannot tap for the draw, which vigilance was never about",
    );
    assert!(
        !game.can_attack(her(&game, loran)),
        "nor attack, for the same reason",
    );

    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == loran)
        .expect("she is there")
        .entered_controller_turn = arrived - 1;

    assert!(taps(&game), "a turn later the draw is hers to take");
    assert!(game.can_attack(her(&game, loran)), "and so is the attack");
}
