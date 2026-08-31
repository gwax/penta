//! Gau, Feral Youth: a two-drop that grows every attack and throws that
//! growth at the opponent once the graveyard has given something up.

use super::*;

/// Player One with Gau out since last turn, and `graveyard` behind him.
fn staged(graveyard: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    for definition in graveyard {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].graveyard.push(card);
    }
    let gau = game
        .put_onto_battlefield(PlayerId::One, cards::GAU_FERAL_YOUTH)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    game.card_left_graveyard_this_turn = [false; 2];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, gau)
}

fn deciding(game: &Game) -> Option<PlayerId> {
    game.pending_decisions
        .first()
        .map(|pending| pending.observation.player)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if deciding(game).is_some() {
            return;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

/// Walks into the end step and lets whatever triggers there resolve.
fn reach_end_step(game: &mut Game) {
    game.step = Step::PostcombatMain;
    game.priority = PlayerId::One;
    for _ in 0..16 {
        settle(game);
        if game.step == Step::End && game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    settle(game);
}

fn counters(game: &Game, gau: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == gau)
        .expect("he is on the battlefield")
        .counters(CounterKind::PlusOnePlusOne)
}

/// Rage: attacking puts a counter on him.
#[test]
fn attacking_grows_him() {
    let (mut game, gau) = staged(&[]);
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    assert_eq!(counters(&game, gau), 0, "nothing on him yet");

    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: gau,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("he attacks");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    settle(&mut game);

    assert_eq!(counters(&game, gau), 1, "one counter for the attack");
}

/// With nothing having left the graveyard, the end step does nothing.
#[test]
fn a_quiet_graveyard_deals_no_damage() {
    let (mut game, _gau) = staged(&[cards::LIGHTNING_BOLT]);

    reach_end_step(&mut game);

    assert_eq!(
        game.players[1].life, 20,
        "the intervening-if found nothing to answer it",
    );
}

/// A card leaving the graveyard turns the end step on, and the damage is
/// his power.
#[test]
fn a_card_leaving_the_graveyard_deals_his_power() {
    let (mut game, gau) = staged(&[cards::LIGHTNING_BOLT]);
    let buried = game.players[0].graveyard[0].id;
    game.move_target_to_zone(
        Target::Card(buried),
        ZoneKind::Exile,
        ZoneMoveCause::Effect {
            controller: PlayerId::One,
        },
        None,
        ZonePlacement::Top,
    );
    settle(&mut game);
    assert_eq!(counters(&game, gau), 0, "he is still a 2/2");

    reach_end_step(&mut game);

    assert_eq!(game.players[1].life, 18, "two damage, which is his power");
}

/// The damage is read off him when it happens, so counters raise it.
#[test]
fn counters_raise_the_damage() {
    let (mut game, gau) = staged(&[cards::LIGHTNING_BOLT]);
    let buried = game.players[0].graveyard[0].id;
    game.move_target_to_zone(
        Target::Card(buried),
        ZoneKind::Hand,
        ZoneMoveCause::Effect {
            controller: PlayerId::One,
        },
        None,
        ZonePlacement::Top,
    );
    settle(&mut game);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == gau)
    {
        permanent.set_counters(CounterKind::PlusOnePlusOne, 3);
    }

    reach_end_step(&mut game);

    assert_eq!(game.players[1].life, 15, "a 5/5 deals five");
}

/// It is your own graveyard: theirs emptying does nothing.
#[test]
fn their_graveyard_does_not_turn_it_on() {
    let (mut game, _gau) = staged(&[]);
    let theirs = game
        .build_zone(PlayerId::Two, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = theirs.id;
    game.players[1].graveyard.push(theirs);
    game.move_target_to_zone(
        Target::Card(id),
        ZoneKind::Exile,
        ZoneMoveCause::Effect {
            controller: PlayerId::Two,
        },
        None,
        ZonePlacement::Top,
    );
    settle(&mut game);

    reach_end_step(&mut game);

    assert_eq!(game.players[1].life, 20, "the clause names your graveyard");
}

/// A card put *into* the graveyard is not a card leaving it.
#[test]
fn filling_the_graveyard_does_not_turn_it_on() {
    let (mut game, _gau) = staged(&[]);
    let held = game
        .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = held.id;
    game.players[0].hand.push(held);
    game.move_target_to_zone(
        Target::Card(id),
        ZoneKind::Graveyard,
        ZoneMoveCause::Effect {
            controller: PlayerId::One,
        },
        None,
        ZonePlacement::Top,
    );
    settle(&mut game);

    reach_end_step(&mut game);

    assert_eq!(game.players[1].life, 20, "the clause reads one direction");
}

/// Lifts `index`'s card out of Player One's graveyard, which is what the
/// end step asks about.
fn exile_from_graveyard(game: &mut Game, index: usize) {
    let buried = game.players[0].graveyard[index].id;
    game.move_target_to_zone(
        Target::Card(buried),
        ZoneKind::Exile,
        ZoneMoveCause::Effect {
            controller: PlayerId::One,
        },
        None,
        ZonePlacement::Top,
    );
    settle(game);
}

/// "At the beginning of each end step", which the record reads as theirs as
/// much as yours: a graveyard emptied on their turn pays out on their turn.
#[test]
fn their_end_step_pays_out_too() {
    let (mut game, _gau) = staged(&[cards::LIGHTNING_BOLT]);
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;
    exile_from_graveyard(&mut game, 0);

    game.step = Step::PostcombatMain;
    for _ in 0..16 {
        settle(&mut game);
        if game.step == Step::End && game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    settle(&mut game);

    assert_eq!(
        game.players[1].life, 18,
        "the end step is theirs and the damage is still yours to deal",
    );
}

/// "If Gau is no longer on the battlefield when its last ability resolves,
/// use its power as it last existed on the battlefield." Answering him
/// underneath his own trigger does not answer the trigger.
#[test]
fn a_dead_gau_still_deals_the_power_he_had() {
    let (mut game, gau) = staged(&[cards::LIGHTNING_BOLT]);
    exile_from_graveyard(&mut game, 0);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == gau)
    {
        permanent.add_counters(CounterKind::PlusOnePlusOne, 2);
    }

    // Into the end step and no further: the trigger is on the stack with him
    // still standing under it.
    game.step = Step::PostcombatMain;
    game.priority = PlayerId::One;
    for _ in 0..16 {
        if game.step == Step::End && !game.stack.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    assert_eq!(game.stack.len(), 1, "his end-step trigger is waiting");

    game.move_permanents_to_graveyard(&[gau]);
    game.check_state_based_actions();
    settle(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == gau),
        "he is gone",
    );
    assert_eq!(
        game.players[1].life, 16,
        "and the four he was is what the trigger throws",
    );
}
