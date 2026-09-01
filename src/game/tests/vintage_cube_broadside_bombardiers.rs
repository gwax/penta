//! Broadside Bombardiers: a hasty attacker that turns whatever else is
//! lying around into reach, once a turn and only after it has attacked.

use super::*;

/// The Goblin on the battlefield ready to attack, with `others` beside it.
fn staged(others: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let bombardiers = game
        .put_onto_battlefield(PlayerId::One, cards::BROADSIDE_BOMBARDIERS)
        .expect("cataloged");
    let mut ids = Vec::new();
    for definition in others {
        ids.push(
            game.put_onto_battlefield(PlayerId::One, *definition)
                .expect("cataloged"),
        );
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    game.players[1].life = 20;
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;
    (game, bombardiers, ids)
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

fn attack(game: &mut Game, bombardiers: GameObjectId) {
    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: bombardiers,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("haste lets it attack");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration finishes");
    drain_pending(game);
}

/// Every boast activation on offer, by which permanent it would throw.
fn boasts(game: &Game, bombardiers: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == bombardiers))
        .collect()
}

fn boast_throwing(game: &mut Game, bombardiers: GameObjectId, thrown: GameObjectId) {
    let action = boasts(game, bombardiers)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                cost_objects,
                targets,
                ..
            } => {
                cost_objects.contains(&thrown)
                    && targets.iter().any(|selection| {
                        selection.targets().contains(&Target::Player(PlayerId::Two))
                    })
            }
            _ => false,
        })
        .expect("that permanent can be thrown across the table");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(game);
}

/// Boast is only for a creature that attacked.
#[test]
fn it_cannot_boast_before_attacking() {
    let (game, bombardiers, _) = staged(&[cards::GRIZZLY_BEARS]);

    assert!(
        boasts(&game, bombardiers).is_empty(),
        "nothing has attacked yet",
    );
}

/// Two plus the sacrificed permanent's mana value: a Bears is two, so the
/// throw is four.
#[test]
fn it_throws_a_two_drop_for_four() {
    let (mut game, bombardiers, ids) = staged(&[cards::GRIZZLY_BEARS]);
    attack(&mut game, bombardiers);

    boast_throwing(&mut game, bombardiers, ids[0]);

    assert_eq!(game.players[1].life, 16, "two plus the Bears' two");
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == ids[0]),
        "and the Bears paid for it",
    );
}

/// The bigger the thing thrown, the bigger the throw.
#[test]
fn a_five_drop_throws_for_seven() {
    let (mut game, bombardiers, ids) = staged(&[cards::SERRA_ANGEL]);
    attack(&mut game, bombardiers);

    boast_throwing(&mut game, bombardiers, ids[0]);

    assert_eq!(game.players[1].life, 13, "two plus the Angel's five");
}

/// An artifact is a legal thing to throw, and a Sol Ring is worth one.
#[test]
fn an_artifact_can_be_thrown_too() {
    let (mut game, bombardiers, ids) = staged(&[cards::SOL_RING]);
    attack(&mut game, bombardiers);

    boast_throwing(&mut game, bombardiers, ids[0]);

    assert_eq!(game.players[1].life, 17, "two plus the Ring's one");
}

/// Only once each turn, however much is left to throw.
#[test]
fn it_boasts_only_once_a_turn() {
    let (mut game, bombardiers, ids) = staged(&[cards::GRIZZLY_BEARS, cards::SOL_RING]);
    attack(&mut game, bombardiers);
    boast_throwing(&mut game, bombardiers, ids[0]);

    assert!(
        boasts(&game, bombardiers).is_empty(),
        "the second Goblin-throw waits for next turn",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == ids[1]),
        "and the Ring is still there",
    );
}

/// "Another": with nothing else around, there is nothing to sacrifice.
#[test]
fn it_cannot_throw_itself() {
    let (mut game, bombardiers, _) = staged(&[]);
    attack(&mut game, bombardiers);

    assert!(
        boasts(&game, bombardiers).is_empty(),
        "the Goblin is not another creature",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == bombardiers),
        "and it is still here",
    );
}

/// "A boast ability can be activated at any point after the creature with
/// that ability has been declared as an attacker ... during the postcombat
/// main phase, during the end step." Attacking is what opens it, and combat
/// ending does not close it again.
#[test]
fn the_boast_outlasts_the_combat_that_opened_it() {
    let (mut game, bombardiers, others) = staged(&[cards::GRIZZLY_BEARS]);
    attack(&mut game, bombardiers);

    // Out of combat entirely, in your own postcombat main phase.
    game.step = Step::EndOfCombat;
    game.advance_step();
    game.finish_rules_procedure();
    assert_eq!(game.step, Step::PostcombatMain);
    assert!(
        !boasts(&game, bombardiers).is_empty(),
        "the attack is what it asks about, and that already happened",
    );

    // And still in the end step, which is the last window the ruling names.
    game.step = Step::End;
    game.priority = PlayerId::One;
    assert!(
        !boasts(&game, bombardiers).is_empty(),
        "the end step is late but not too late",
    );

    boast_throwing(&mut game, bombardiers, others[0]);

    assert_eq!(
        game.players[1].life, 16,
        "two plus the Bears' two, thrown after the combat was over",
    );
}

/// "Any target": every test above throws across the table, and a creature is
/// the other thing the clause names.
#[test]
fn the_throw_may_be_aimed_at_a_creature() {
    let (mut game, bombardiers, ids) = staged(&[cards::SERRA_ANGEL]);
    let titan = game
        .put_onto_battlefield(PlayerId::Two, cards::GRAVE_TITAN)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    attack(&mut game, bombardiers);

    let action = boasts(&game, bombardiers)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                cost_objects,
                targets,
                ..
            } => {
                cost_objects.contains(&ids[0])
                    && targets
                        .iter()
                        .any(|selection| selection.targets().contains(&Target::Permanent(titan)))
            }
            _ => false,
        })
        .expect("a creature is a thing it may throw the Angel at");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(&mut game);
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == titan),
        "two plus the Angel's five is seven, and a 6/6 does not survive it",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        20,
        "and their life was never the target",
    );
}

/// Menace: the Goblin is blocked by two or by nobody. The constraint shows
/// up as being unable to finish the declaration with one blocker on it.
#[test]
fn menace_asks_for_a_second_blocker() {
    let (mut game, bombardiers, _) = staged(&[]);
    let mut blockers = Vec::new();
    for instance in [98_000, 98_001] {
        let blocker = creature(instance, cards::GRIZZLY_BEARS, PlayerId::Two);
        blockers.push(blocker.card.id);
        game.battlefield.push(blocker);
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.priority = PlayerId::One;
    attack(&mut game, bombardiers);
    game.step = Step::DeclareBlockers;
    game.priority = PlayerId::Two;

    let may_finish = |game: &Game| {
        game.legal_actions(PlayerId::Two)
            .iter()
            .any(|action| matches!(action, Action::FinishDeclaringBlockers))
    };
    assert!(may_finish(&game), "blocking with nobody is always allowed");

    game.apply(
        PlayerId::Two,
        Action::DeclareBlocker {
            blocker: blockers[0],
            attacker: bombardiers,
        },
    )
    .expect("one blocker may be declared");
    assert!(
        !may_finish(&game),
        "but the declaration cannot be finished with only one on it",
    );

    game.apply(
        PlayerId::Two,
        Action::DeclareBlocker {
            blocker: blockers[1],
            attacker: bombardiers,
        },
    )
    .expect("the second blocker joins it");

    assert!(
        may_finish(&game),
        "and two of them together are what menace asks for",
    );
}

/// Haste is what makes boast reachable the turn it lands: a Goblin that
/// arrived this turn attacks, and attacking is what opens the throw.
#[test]
fn haste_opens_the_boast_the_turn_it_arrives() {
    let (mut game, bombardiers, others) = staged(&[cards::GRIZZLY_BEARS]);
    let arrived = game.turns_started[PlayerId::One.index()];
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == bombardiers)
        .expect("it is there")
        .entered_controller_turn = arrived;

    assert!(
        boasts(&game, bombardiers).is_empty(),
        "nothing has attacked yet",
    );
    attack(&mut game, bombardiers);

    let life = game.players[PlayerId::Two.index()].life;
    boast_throwing(&mut game, bombardiers, others[0]);

    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        life - 4,
        "two plus a two-drop, thrown on the turn the Goblin arrived",
    );
}

/// A token's mana value is zero, so throwing one is two damage and no more
/// -- which is what makes a Cat a worse missile than a bear.
#[test]
fn a_token_thrown_is_worth_exactly_two() {
    let (mut game, bombardiers, _others) = staged(&[cards::ESIKA_S_CHARIOT]);
    settle(&mut game);
    let cat = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Token)
        .expect("the Chariot brought Cats")
        .card
        .id;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;
    attack(&mut game, bombardiers);
    let life = game.players[PlayerId::Two.index()].life;

    boast_throwing(&mut game, bombardiers, cat);

    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        life - 2,
        "two plus nothing: a token has no mana cost to read",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == cat),
        "and the Cat was the cost",
    );
}
