//! Glorybringer, and exert: a choice made as it attacks, paid for with the
//! next untap step.

use super::*;

/// A declare-attackers step with an untapped Glorybringer and a pair of
/// creatures across the table -- one of them a Dragon.
fn staged() -> (Game, GameObjectId, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let dragon = game
        .put_onto_battlefield(PlayerId::One, cards::GLORYBRINGER)
        .expect("cataloged");
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    let their_dragon = game
        .put_onto_battlefield(PlayerId::Two, cards::GLORYBRINGER)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;
    (game, dragon, bears, their_dragon)
}

fn exert_actions(game: &Game) -> Vec<GameObjectId> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ExertAttacker { attacker } => Some(attacker),
            _ => None,
        })
        .collect()
}

fn attack_with(game: &mut Game, attacker: GameObjectId) {
    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("it attacks");
}

/// Exert is offered only once the creature is actually attacking.
#[test]
fn exert_is_offered_only_to_a_declared_attacker() {
    let (mut game, dragon, _bears, _their_dragon) = staged();
    assert!(
        exert_actions(&game).is_empty(),
        "nothing has been declared yet",
    );

    attack_with(&mut game, dragon);

    assert_eq!(exert_actions(&game), vec![dragon]);
}

/// A creature with no exert clause is never offered it.
#[test]
fn a_creature_without_the_clause_is_never_offered_it() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;
    attack_with(&mut game, bears);

    assert!(exert_actions(&game).is_empty());
}

/// Exerting costs the next untap step, and only one however the turn goes.
#[test]
fn exerting_owes_an_untap_step() {
    let (mut game, dragon, _bears, _their_dragon) = staged();
    attack_with(&mut game, dragon);

    game.apply(PlayerId::One, Action::ExertAttacker { attacker: dragon })
        .expect("it exerts");

    let exerted = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == dragon)
        .expect("it is still there");
    assert!(exerted.exerted);
    assert_eq!(exerted.skipped_untap_steps, 1);
    assert!(
        exert_actions(&game).is_empty(),
        "and it cannot be exerted twice",
    );
}

/// "When you do": the reflexive trigger deals four to something they
/// control.
#[test]
fn exerting_deals_four_to_their_creature() {
    let (mut game, dragon, bears, _their_dragon) = staged();
    attack_with(&mut game, dragon);
    game.apply(PlayerId::One, Action::ExertAttacker { attacker: dragon })
        .expect("it exerts");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != bears),
        "four damage kills a 2/2",
    );
}

/// Not exerting fires nothing.
#[test]
fn attacking_without_exerting_deals_no_damage() {
    let (mut game, dragon, bears, _their_dragon) = staged();
    attack_with(&mut game, dragon);
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears),
        "the bear is untouched",
    );
    let attacker = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == dragon)
        .expect("it is still there");
    assert_eq!(attacker.skipped_untap_steps, 0, "and nothing was owed");
}

/// "Non-Dragon": their own Glorybringer is not a legal target.
#[test]
fn the_trigger_does_not_name_a_dragon() {
    let (mut game, dragon, bears, their_dragon) = staged();
    attack_with(&mut game, dragon);
    game.apply(PlayerId::One, Action::ExertAttacker { attacker: dragon })
        .expect("it exerts");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");

    let choice = game
        .observe(PlayerId::One)
        .decision
        .expect("the reflexive trigger asks for its target");
    let offered = choice
        .options
        .iter()
        .filter_map(|option| option.card.map(|(instance, _)| instance))
        .collect::<Vec<_>>();

    assert!(offered.contains(&bears), "the bear is on the menu");
    assert!(
        !offered.contains(&their_dragon),
        "their Dragon is not: {offered:?}",
    );
}

/// Flying and haste are printed on it, so it attacks the turn it lands.
#[test]
fn it_flies_and_has_haste() {
    let (game, dragon, _bears, _their_dragon) = staged();
    let glorybringer = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == dragon)
        .expect("it is there");

    assert!(game.permanent_has_executable_keyword(glorybringer, KeywordAbility::Flying));
    assert!(game.permanent_has_executable_keyword(glorybringer, KeywordAbility::Haste));
}

/// "You can exert it even if there isn't a legal target for that triggered
/// ability." A board with nothing but their own Dragon on it is a board the
/// trigger cannot point at, and the exert is offered all the same.
#[test]
fn it_may_be_exerted_with_nothing_to_shoot() {
    let (mut game, dragon, bears, _their_dragon) = staged();
    game.battlefield
        .retain(|permanent| permanent.card.id != bears);

    attack_with(&mut game, dragon);
    assert_eq!(
        exert_actions(&game),
        vec![dragon],
        "the offer does not depend on the trigger having a target",
    );

    game.apply(PlayerId::One, Action::ExertAttacker { attacker: dragon })
        .expect("exerting is legal");
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == dragon && permanent.exerted),
        "and the Dragon owes its untap step for nothing",
    );
}

/// "You can't do so later in combat": exert is a choice made as the attack
/// is declared, and once the declaration is over the offer is gone.
#[test]
fn exert_is_not_offered_after_the_declaration() {
    let (mut game, dragon, _bears, _their_dragon) = staged();
    attack_with(&mut game, dragon);
    assert_eq!(exert_actions(&game), vec![dragon], "while it is being made");

    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");

    assert!(
        exert_actions(&game).is_empty(),
        "and not once it is finished",
    );
    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;
    assert!(
        exert_actions(&game).is_empty(),
        "nor later in the combat it started",
    );
}

/// "A creature an opponent controls": your own board is not on the menu,
/// however tempting a blocker you no longer want might be.
#[test]
fn the_trigger_does_not_name_your_own_creature() {
    let (mut game, dragon, bears, _their_dragon) = staged();
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;

    attack_with(&mut game, dragon);
    game.apply(PlayerId::One, Action::ExertAttacker { attacker: dragon })
        .expect("it exerts");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");

    let choice = game
        .observe(PlayerId::One)
        .decision
        .expect("the reflexive trigger asks for its target");
    let offered = choice
        .options
        .iter()
        .filter_map(|option| option.card.map(|(instance, _)| instance))
        .collect::<Vec<_>>();

    assert!(offered.contains(&bears), "their bear is on the menu");
    assert!(
        !offered.contains(&mine),
        "yours is not, whoever would rather it were: {offered:?}",
    );
}

/// "If an exerted creature is already untapped during your next untap step
/// (most likely because it had vigilance or an effect untapped it), exert's
/// effect preventing it from untapping expires without having done
/// anything." The step it was owed is the step it is spent on, tapped or
/// not: it does not wait around for the next one.
#[test]
fn an_exert_owed_by_an_untapped_creature_expires_unspent() {
    let (mut game, dragon, _bears, _their_dragon) = staged();
    attack_with(&mut game, dragon);
    game.apply(PlayerId::One, Action::ExertAttacker { attacker: dragon })
        .expect("it exerts");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    drain_pending(&mut game);

    // Standing in for the vigilance or the untapper: it is not tapped when
    // the untap step it owes comes around.
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == dragon)
    {
        permanent.tapped = false;
    }
    game.commit_next_turn(PlayerId::Two, Vec::new());
    drain_pending(&mut game);
    game.commit_next_turn(PlayerId::One, Vec::new());
    drain_pending(&mut game);

    let after = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == dragon)
        .expect("it is still there");
    assert!(!after.tapped, "there was nothing for the skip to do");
    assert_eq!(
        after.skipped_untap_steps, 0,
        "and it was spent on that step all the same",
    );

    // Which is to say the turn after is an ordinary one.
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == dragon)
    {
        permanent.tapped = true;
    }
    game.commit_next_turn(PlayerId::Two, Vec::new());
    drain_pending(&mut game);
    game.commit_next_turn(PlayerId::One, Vec::new());
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == dragon)
            .expect("still there")
            .tapped,
        "the Dragon untaps normally once the exert is behind it",
    );
}
