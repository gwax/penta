//! Generous Plunderer: he hands the other player a Treasure every upkeep
//! and then bills them for it on the attack.

use super::*;

/// The Plunderer on the battlefield under Player One since last turn.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let plunderer = game
        .put_onto_battlefield(PlayerId::One, cards::GENEROUS_PLUNDERER)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.players[0].life = 20;
    game.players[1].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, plunderer)
}

/// Answers decisions, accepting the offer when `accept` and declining
/// otherwise.
fn settle(game: &mut Game, accept: bool) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            // A target decision has no declining option; the offer to make
            // the Treasures does, and that is the one this steers.
            let options = decision
                .options
                .iter()
                .find(|option| (option.label != "Decline") == accept)
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
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(game);
}

fn treasures(game: &Game, player: PlayerId) -> Vec<&Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.controller == player)
        .filter(|permanent| game.effective_subtypes(permanent).contains(&"Treasure"))
        .collect()
}

/// Runs Player One's upkeep trigger.
fn upkeep(game: &mut Game, accept: bool) {
    game.active_player = PlayerId::One;
    game.priority = PlayerId::One;
    game.step = Step::Upkeep;
    game.handle_upkeep_triggers();
    settle(game, accept);
    game.step = Step::PrecombatMain;
}

/// The upkeep makes one Treasure for you and a tapped one for them.
#[test]
fn the_upkeep_pays_them_too() {
    let (mut game, _) = staged();

    upkeep(&mut game, true);

    let mine = treasures(&game, PlayerId::One);
    let theirs = treasures(&game, PlayerId::Two);
    assert_eq!(mine.len(), 1, "one Treasure for you");
    assert!(!mine[0].tapped, "and yours is usable now");
    assert_eq!(theirs.len(), 1, "and one for them");
    assert!(theirs[0].tapped, "which arrives tapped");
}

/// "You may": declining makes neither.
#[test]
fn declining_makes_nothing() {
    let (mut game, _) = staged();

    upkeep(&mut game, false);

    assert!(treasures(&game, PlayerId::One).is_empty());
    assert!(treasures(&game, PlayerId::Two).is_empty(), "nor theirs");
}

/// Attacking bills them for every artifact they have.
#[test]
fn attacking_bills_them_for_their_artifacts() {
    let (mut game, plunderer) = staged();
    upkeep(&mut game, true);
    game.put_onto_battlefield(PlayerId::Two, cards::HOWLING_MINE)
        .expect("cataloged");
    drain_pending(&mut game);

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(plunderer, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    settle(&mut game, true);

    assert_eq!(
        game.players[1].life, 18,
        "the Treasure he gave them and the Mine they had",
    );
}

/// With nothing of theirs on the battlefield the trigger deals nothing.
#[test]
fn an_empty_board_bills_them_for_nothing() {
    let (mut game, plunderer) = staged();

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(plunderer, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    settle(&mut game, true);

    assert_eq!(game.players[1].life, 20, "no artifacts, no damage");
}

/// Your own artifacts are not theirs.
#[test]
fn your_own_artifacts_do_not_count() {
    let (mut game, plunderer) = staged();
    game.put_onto_battlefield(PlayerId::One, cards::HOWLING_MINE)
        .expect("cataloged");
    drain_pending(&mut game);

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(plunderer, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    settle(&mut game, true);

    assert_eq!(game.players[1].life, 20, "the Mine is yours, not theirs");
}

/// He has menace.
#[test]
fn he_has_menace() {
    let (game, plunderer) = staged();
    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == plunderer)
        .expect("he is there");

    assert!(game.permanent_has_executable_keyword(body, KeywordAbility::Menace));
}

/// "You don't choose a target for the first triggered ability at the time it
/// triggers. Rather, a second 'reflexive' ability triggers when you create a
/// Treasure token this way. You choose a target for that ability as it goes
/// on the stack. Each player may respond to this triggered ability as
/// normal." So the Treasure you keep exists before the gift is even on the
/// stack, and the gift waits there for an answer.
#[test]
fn the_gift_is_a_separate_trigger_the_table_may_respond_to() {
    let (mut game, _) = staged();
    game.active_player = PlayerId::One;
    game.priority = PlayerId::One;
    game.step = Step::Upkeep;
    game.handle_upkeep_triggers();

    // Put the upkeep trigger on the stack and let it resolve into the offer.
    for _ in 0..8 {
        if game
            .pending_decisions
            .first()
            .is_some_and(|pending| pending.observation.options.len() == 2)
        {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    let offer = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the upkeep asks whether to make a Treasure");
    assert!(
        offer
            .options
            .iter()
            .all(|option| option.card.is_none() || option.label == "Decline"),
        "and it names nobody: {:?}",
        offer.options,
    );
    let accept = offer
        .options
        .iter()
        .find(|option| option.label != "Decline")
        .expect("it can be accepted")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: offer.id,
            options: vec![accept],
        },
    )
    .expect("accepting is legal");

    assert_eq!(
        treasures(&game, PlayerId::One).len(),
        1,
        "your own Treasure is made as the offer is accepted",
    );
    assert!(
        treasures(&game, PlayerId::Two).is_empty(),
        "theirs is not, because the gift has not resolved yet",
    );
    assert!(
        game.pending_decisions.iter().any(|pending| {
            pending
                .observation
                .prompt
                .contains("choose target opponent")
        }),
        "the gift is its own ability, and names its target on the way to the stack: {:?}",
        game.pending_decisions
            .iter()
            .map(|pending| pending.observation.prompt.clone())
            .collect::<Vec<_>>(),
    );

    settle(&mut game, true);

    let gift = treasures(&game, PlayerId::Two);
    assert_eq!(gift.len(), 1, "and then they get theirs");
    assert!(gift[0].tapped, "tapped, as the gift says");
}

/// "Equal to the number of artifacts they control" is counted as the attack
/// trigger resolves, not as it is put on the stack: an artifact answered in
/// that window is one they no longer control, and the bill is smaller for
/// it.
#[test]
fn an_artifact_answered_in_response_is_one_they_are_not_billed_for() {
    let (mut game, plunderer) = staged();
    let mine = game
        .put_onto_battlefield(PlayerId::Two, cards::HOWLING_MINE)
        .expect("cataloged");
    game.put_onto_battlefield(PlayerId::Two, cards::SOL_RING)
        .expect("cataloged");
    drain_pending(&mut game);

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(plunderer, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();

    game.destroy_permanent(mine);
    game.check_state_based_actions();
    settle(&mut game, true);

    assert_eq!(
        game.players[1].life, 19,
        "one artifact left when the count was taken",
    );
}

/// And the same window read the other way: an artifact that arrives before
/// the trigger resolves is counted too.
#[test]
fn an_artifact_that_arrives_in_response_is_billed_for() {
    let (mut game, plunderer) = staged();
    game.put_onto_battlefield(PlayerId::Two, cards::HOWLING_MINE)
        .expect("cataloged");
    drain_pending(&mut game);

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(plunderer, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();

    game.put_onto_battlefield(PlayerId::Two, cards::SOL_RING)
        .expect("cataloged");
    game.check_state_based_actions();
    settle(&mut game, true);

    assert_eq!(
        game.players[1].life, 18,
        "both of them were there when the count was taken",
    );
}
