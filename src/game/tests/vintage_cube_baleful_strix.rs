//! Baleful Strix: two mana that replaces itself and then eats whatever
//! attacks into it.

use super::*;

fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].library.clear();
    game.players[PlayerId::One.index()].library.push(card(
        86_000,
        cards::GIANT_GROWTH,
        PlayerId::One,
    ));
    let strix = game
        .put_onto_battlefield(PlayerId::One, cards::BALEFUL_STRIX)
        .expect("cataloged");
    drain_pending(&mut game);
    (game, strix)
}

/// Flying and deathtouch, and a card on the way in.
#[test]
fn it_flies_touches_and_draws() {
    let (game, strix) = staged();
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == strix)
        .expect("it is there");

    assert!(game.permanent_has_executable_keyword(permanent, KeywordAbility::Flying));
    assert!(game.permanent_has_executable_keyword(permanent, KeywordAbility::Deathtouch));
    assert_eq!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::GIANT_GROWTH],
        "entering draws one",
    );
}

/// It is an artifact as well as a creature, which is what makes it a
/// target for artifact removal and food for artifact synergies.
#[test]
fn it_is_an_artifact_creature() {
    let (game, strix) = staged();
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == strix)
        .expect("it is there");

    assert!(
        game.permanent_types(permanent)
            .is_some_and(|types| types.contains(CardType::Artifact)),
    );
    assert!(
        game.permanent_types(permanent)
            .is_some_and(|types| types.contains(CardType::Creature)),
    );
    assert_eq!(
        (game.power(permanent), game.toughness(permanent)),
        (Some(1), Some(1))
    );
}

/// A 1/1 with deathtouch kills whatever it damages, however large.
#[test]
fn its_damage_is_lethal_whatever_it_hits() {
    let (mut game, strix) = staged();
    let angel = creature(86_100, cards::SERRA_ANGEL, PlayerId::Two);
    let angel_id = angel.card.id;
    game.battlefield.push(angel);

    game.damage_target_from(Some(strix), Some(Target::Permanent(angel_id)), 1);
    game.check_state_based_actions();

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != angel_id),
        "one point from a source with deathtouch is lethal",
    );
}

/// Flying is not just a flag on the permanent: a ground creature is not
/// offered as a blocker for it, and a flier is.
#[test]
fn only_a_flier_may_block_it() {
    let (mut game, strix) = staged();
    let bears = creature(86_200, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let angel = creature(86_201, cards::SERRA_ANGEL, PlayerId::Two);
    let angel_id = angel.card.id;
    game.battlefield.push(angel);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.declare_attacker(strix, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;

    let blockers = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .filter_map(|action| match action {
            Action::DeclareBlocker { blocker, attacker } if attacker == strix => Some(blocker),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        blockers,
        vec![angel_id],
        "the Angel flies and the Bears do not",
    );
    assert!(!blockers.contains(&bears_id));
}

/// The draw is a triggered ability, so it is on the stack in its own right:
/// answering the body before the trigger resolves still leaves the card.
#[test]
fn the_card_is_drawn_even_if_the_bird_is_answered() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(86_300, cards::GIANT_GROWTH, PlayerId::One));
    let strix = game
        .put_onto_battlefield(PlayerId::One, cards::BALEFUL_STRIX)
        .expect("cataloged");
    // The trigger is waiting; the Bird is not.
    game.begin_trigger_placement();
    game.destroy_permanent(strix);
    game.check_state_based_actions();
    drain_pending(&mut game);

    assert!(
        game.battlefield.is_empty(),
        "the Bird was answered before its trigger resolved",
    );
    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::GIANT_GROWTH],
        "and the card it drew is a card it drew",
    );
}

/// Deathtouch and trample together: a Wurm blocked by the Strix needs to
/// assign only one damage as lethal, and the other fourteen go through. The
/// Bird dies for it, and so does the Wurm.
#[test]
fn blocking_a_trampler_kills_it_and_lets_the_rest_through() {
    let (mut game, strix) = staged();
    let wurm = game
        .put_onto_battlefield(PlayerId::Two, cards::WORLDSPINE_WURM)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::Two;
    game.step = Step::DeclareAttackers;
    game.priority = PlayerId::Two;
    let life = game.players[PlayerId::One.index()].life;

    game.declare_attacker(wurm, AttackDefender::Player(PlayerId::One));
    game.finish_declaring_attackers();
    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;
    game.declare_blocker(strix, wurm);
    game.finish_declaring_blockers();
    settle_combat(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == wurm),
        "one point of deathtouch damage is lethal to fifteen toughness",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == strix),
        "and the Wurm was more than enough for a 1/1",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life - 14,
        "the one it had to assign to the blocker stayed there; the rest trampled",
    );
}

/// Resolves combat damage and whatever it sets off.
fn settle_combat(game: &mut Game) {
    game.deal_combat_damage();
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: decision
                        .options
                        .iter()
                        .map(|option| option.id)
                        .take(decision.minimum.max(1))
                        .collect(),
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

/// Deathtouch destroys, and indestructible ignores destruction: a Darksteel
/// Myr the Bird bites takes its one damage and stands there, which is the
/// one board a 1/1 deathtoucher cannot answer.
#[test]
fn indestructible_survives_the_bite() {
    let (mut game, strix) = staged();
    let myr = game
        .put_onto_battlefield(PlayerId::Two, cards::DARKSTEEL_MYR)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;

    game.declare_attacker(strix, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;
    game.declare_blocker(myr, strix);
    game.finish_declaring_blockers();
    settle_combat(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == myr),
        "deathtouch says destroy, and it cannot be destroyed",
    );
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == myr)
            .expect("it is still there")
            .damage,
        1,
        "with the Bird's one point marked on it all the same",
    );
}

/// Deathtouch is about creatures: damage to a planeswalker is loyalty and
/// nothing more, so the Bird takes one counter off Jace rather than the
/// whole of him.
#[test]
fn a_planeswalker_loses_one_loyalty_and_no_more() {
    let (mut game, strix) = staged();
    let jace = game
        .put_onto_battlefield(PlayerId::Two, cards::JACE_THE_MIND_SCULPTOR)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;

    game.declare_attacker(strix, AttackDefender::Planeswalker(jace));
    game.finish_declaring_attackers();
    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;
    game.finish_declaring_blockers();
    settle_combat(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == jace)
            .expect("he is still standing")
            .counters(CounterKind::Loyalty),
        2,
        "one point of deathtouch damage is one loyalty counter",
    );
}
