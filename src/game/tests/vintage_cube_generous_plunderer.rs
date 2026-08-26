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
