//! Woodfall Primus: eight mana for two Naturalizes and a trampling body
//! that has to be answered twice.

use super::*;

/// The Primus in hand with the mana for it, and `theirs` on the battlefield
/// under Player Two.
fn staged(theirs: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    let mut ids = Vec::new();
    for definition in theirs {
        ids.push(
            game.put_onto_battlefield(PlayerId::Two, *definition)
                .expect("cataloged"),
        );
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [8, 8];
    drain_pending(&mut game);
    let primus = game
        .build_zone(PlayerId::One, &[cards::WOODFALL_PRIMUS])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = primus.id;
    game.players[0].hand.push(primus);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 3);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 5);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, id, ids)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .take(decision.minimum.max(1))
                .map(|option| option.id)
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

/// Casts it, aiming the enters trigger at `target` when there is one.
fn cast(game: &mut Game, primus: GameObjectId, target: Option<GameObjectId>) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == primus))
        .expect("eight mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    for _ in 0..24 {
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
    game.check_state_based_actions();
}

fn body(game: &Game) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::WOODFALL_PRIMUS)
}

fn on_battlefield(game: &Game, definition: CardDefinitionId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.definition == definition)
}

/// It arrives and takes a land with it.
#[test]
fn it_destroys_a_noncreature_permanent_as_it_enters() {
    let (mut game, primus, ids) = staged(&[cards::SOL_RING]);

    cast(&mut game, primus, Some(ids[0]));

    assert!(!on_battlefield(&game, cards::SOL_RING), "the Ring is gone");
    let primus = body(&game).expect("and the Treefolk stayed");
    assert_eq!(game.power(primus), Some(6));
    assert!(game.has_trample(primus), "with trample");
}

/// A creature is not a legal thing for it to answer.
#[test]
fn it_cannot_answer_a_creature() {
    let (mut game, primus, _) = staged(&[cards::GRIZZLY_BEARS]);

    cast(&mut game, primus, None);

    assert!(
        on_battlefield(&game, cards::GRIZZLY_BEARS),
        "the Bears are not a noncreature permanent",
    );
    assert!(body(&game).is_some(), "and it still enters");
}

/// Persist: it comes back once, smaller, and destroys something else on the
/// way in.
#[test]
fn persist_returns_it_once_with_a_minus_counter() {
    let (mut game, primus, ids) = staged(&[cards::SOL_RING, cards::MOX_PEARL]);
    cast(&mut game, primus, Some(ids[0]));
    let first = body(&game).expect("it is here").card.id;

    game.move_permanents_to_graveyard(&[first]);
    cast_settle_with_target(&mut game, ids[1]);

    let returned = body(&game).expect("persist brought it back");
    assert_eq!(
        returned.counters(CounterKind::MinusOneMinusOne),
        1,
        "with a -1/-1 counter",
    );
    assert_eq!(game.power(returned), Some(5), "a 5/5 now");
    assert_eq!(game.toughness(returned), Some(5));
    assert!(
        !on_battlefield(&game, cards::MOX_PEARL),
        "and it answered something else on the way back",
    );

    // Second death: the counter is on it, so persist does nothing.
    let second = returned.card.id;
    game.move_permanents_to_graveyard(&[second]);
    settle(&mut game);

    assert!(body(&game).is_none(), "it stays dead the second time");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::WOODFALL_PRIMUS),
        "in its owner's graveyard",
    );
}

/// Answers whatever the returning body's trigger asks, naming `target`.
fn cast_settle_with_target(game: &mut Game, target: GameObjectId) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .filter(|option| option.card.is_some_and(|(object, _)| object == target))
                .map(|option| option.id)
                .take(1)
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

/// Persist returns it to its owner, not to whoever killed it.
#[test]
fn persist_returns_it_to_its_owner() {
    let (mut game, primus, ids) = staged(&[cards::SOL_RING]);
    cast(&mut game, primus, Some(ids[0]));
    let first = body(&game).expect("it is here").card.id;
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == first)
        .expect("it is here")
        .controller = PlayerId::Two;

    game.move_permanents_to_graveyard(&[first]);
    settle(&mut game);

    assert_eq!(
        body(&game).expect("it came back").controller,
        PlayerId::One,
        "under its owner's control",
    );
}

/// "When a permanent with persist returns to the battlefield, it's a new
/// object with no memory of or connection to its previous existence." What
/// it carried before -- counters it grew, and the tap it was under -- does
/// not come back with it.
#[test]
fn what_returns_remembers_nothing_but_its_own_counter() {
    let (mut game, primus, ids) = staged(&[cards::SOL_RING, cards::MOX_PEARL]);
    cast(&mut game, primus, Some(ids[0]));
    let first = body(&game).expect("it is here").card.id;
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == first)
    {
        permanent.add_counters(CounterKind::PlusOnePlusOne, 2);
        permanent.tapped = true;
    }
    assert_eq!(
        game.power(body(&game).expect("it is here")),
        Some(8),
        "an 8/8 while it stands there",
    );

    game.move_permanents_to_graveyard(&[first]);
    cast_settle_with_target(&mut game, ids[1]);

    let returned = body(&game).expect("persist brought it back");
    assert_eq!(
        returned.counters(CounterKind::PlusOnePlusOne),
        0,
        "the counters it grew stayed behind",
    );
    assert_eq!(
        returned.counters(CounterKind::MinusOneMinusOne),
        1,
        "and the one persist gives it came with it",
    );
    assert_eq!(game.power(returned), Some(5), "so a 5/5 rather than a 7/7");
    assert!(!returned.tapped, "and untapped, whatever it was before");
}
