//! Luminarch Aspirant: two mana that adds a counter every turn it lives,
//! and adds it before attackers are declared.

use super::*;

/// Her on the battlefield since last turn, with `others` beside her.
fn staged(others: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let aspirant = game
        .put_onto_battlefield(PlayerId::One, cards::LUMINARCH_ASPIRANT)
        .expect("cataloged");
    let mut ids = Vec::new();
    for definition in others {
        ids.push(
            game.put_onto_battlefield(PlayerId::One, *definition)
                .expect("cataloged"),
        );
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, aspirant, ids)
}

/// Runs the turn forward into combat, pointing the trigger at `wanted`.
fn into_combat(game: &mut Game, wanted: GameObjectId) {
    while game.step != Step::BeginningOfCombat {
        game.advance_step();
    }
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options: Vec<_> = decision
                .options
                .iter()
                .filter(|option| option.card.is_some_and(|(object, _)| object == wanted))
                .map(|option| option.id)
                .take(1)
                .collect();
            let options = if options.len() < decision.minimum {
                decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(decision.minimum)
                    .collect()
            } else {
                options
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
    game.check_state_based_actions();
}

fn counters(game: &Game, id: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
        .counters(CounterKind::PlusOnePlusOne)
}

/// She can point it at herself, and a 1/1 becomes a 2/2.
#[test]
fn she_can_grow_herself() {
    let (mut game, aspirant, _) = staged(&[]);

    into_combat(&mut game, aspirant);

    assert_eq!(counters(&game, aspirant), 1);
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == aspirant)
        .expect("still there");
    assert_eq!(game.power(permanent), Some(2));
    assert_eq!(game.toughness(permanent), Some(2));
}

/// Or at something else she controls.
#[test]
fn she_can_grow_another_creature() {
    let (mut game, aspirant, others) = staged(&[cards::GRIZZLY_BEARS]);
    let bears = others[0];

    into_combat(&mut game, bears);

    assert_eq!(counters(&game, bears), 1);
    assert_eq!(
        counters(&game, aspirant),
        0,
        "only one counter, not one each"
    );
}

/// The counter lands before attackers are declared, which is the whole
/// point of the timing.
#[test]
fn the_counter_arrives_before_attackers() {
    let (mut game, aspirant, _) = staged(&[]);

    into_combat(&mut game, aspirant);

    assert_eq!(game.step, Step::BeginningOfCombat, "still before attackers");
    assert_eq!(counters(&game, aspirant), 1);
}

/// It is a targeted trigger: a creature an opponent controls is not on
/// offer.
#[test]
fn it_cannot_point_at_their_creature() {
    let (mut game, aspirant, _) = staged(&[]);
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    while game.step != Step::BeginningOfCombat {
        game.advance_step();
    }
    // The trigger has to reach the stack before it asks, so pass until it
    // does -- but stop at the question rather than answering it.
    let mut decision = None;
    for _ in 0..8 {
        if let Some(pending) = game.pending_decisions.first() {
            decision = Some(pending.observation.clone());
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    let decision = decision.expect("the trigger asks for a target");
    assert!(
        decision
            .options
            .iter()
            .all(|option| option.card.is_none_or(|(object, _)| object != theirs)),
        "their creature is not a legal target",
    );
    assert!(
        decision
            .options
            .iter()
            .any(|option| option.card.is_some_and(|(object, _)| object == aspirant)),
        "and she is",
    );
}

/// Two turns, two counters: it fires every turn she survives.
#[test]
fn it_fires_again_next_turn() {
    let (mut game, aspirant, _) = staged(&[]);

    into_combat(&mut game, aspirant);
    for _ in 0..40 {
        if game.turn > 9 && game.active_player == PlayerId::One {
            break;
        }
        game.advance_step();
        drain_pending(&mut game);
    }
    into_combat(&mut game, aspirant);

    assert_eq!(counters(&game, aspirant), 2);
}

/// "At the beginning of combat on *your* turn": their combat is not yours,
/// however long she has been standing there.
#[test]
fn their_combat_does_not_fire_it() {
    let (mut game, aspirant, others) = staged(&[cards::GRIZZLY_BEARS]);
    let bears = others[0];
    game.active_player = PlayerId::Two;
    game.step = Step::Upkeep;
    game.priority = PlayerId::Two;

    while game.step != Step::BeginningOfCombat {
        game.advance_step();
    }
    drain_pending(&mut game);

    assert!(
        game.pending_decisions.is_empty(),
        "nothing was asked, so nothing triggered",
    );
    assert_eq!(counters(&game, aspirant), 0, "she grew nothing");
    assert_eq!(counters(&game, bears), 0, "and neither did the bear");
}

/// The trigger targets, and a target that is gone when it resolves takes the
/// whole trigger with it: an Aspirant pointed at herself and answered in
/// response leaves the counter nowhere to go.
#[test]
fn a_target_that_dies_first_counters_the_trigger() {
    let (mut game, aspirant, others) = staged(&[cards::GRIZZLY_BEARS]);
    let bears = others[0];

    while game.step != Step::BeginningOfCombat {
        game.advance_step();
    }
    // Onto the stack, where its target is named, but not yet resolved.
    game.finish_rules_procedure();
    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the trigger asks what to grow");
    let herself = decision
        .options
        .iter()
        .find(|option| option.card.is_some_and(|(object, _)| object == aspirant))
        .expect("she is one of the options")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![herself],
        },
    )
    .expect("naming herself is legal");

    game.move_permanents_to_graveyard(&[aspirant]);
    game.check_state_based_actions();
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        if game.apply(game.priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == aspirant),
        "she was answered with her trigger on the stack",
    );
    assert_eq!(
        counters(&game, bears),
        0,
        "and the counter went nowhere, the bear included",
    );
}
