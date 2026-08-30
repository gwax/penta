//! Form of the Dragon's three independent player-facing rules.

use super::*;

fn form_game() -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.put_onto_battlefield(PlayerId::Two, cards::FORM_OF_THE_DRAGON)
        .expect("cataloged");
    drain_pending(&mut game);
    game
}

fn begin_step(game: &mut Game, step: TurnStepDef, player: PlayerId) {
    game.capture_battlefield_triggers(&CommittedTriggerEvent::StepBegins { step, player });
    game.finish_rules_procedure();
}

#[test]
fn every_end_step_sets_its_controllers_life_total_to_five() {
    let mut game = form_game();
    for (active, before) in [(PlayerId::One, 11), (PlayerId::Two, 2)] {
        game.players[PlayerId::Two.index()].life = before;
        begin_step(&mut game, TurnStepDef::End, active);
        drain_pending(&mut game);
        assert_eq!(
            game.players[PlayerId::Two.index()].life,
            5,
            "the trigger both loses and gains life to reach five",
        );
    }
}

#[test]
fn its_upkeep_trigger_deals_five_to_the_chosen_target() {
    let mut game = form_game();
    let before = game.players[PlayerId::One.index()].life;
    begin_step(&mut game, TurnStepDef::Upkeep, PlayerId::Two);

    let pending = game
        .pending_decisions
        .first()
        .expect("the trigger asks for its target");
    let option = match &pending.continuation {
        DecisionContinuation::TriggerPlacement { candidates, .. } => candidates
            .iter()
            .position(|candidate| *candidate == Target::Player(PlayerId::One))
            .expect("the opponent is an any-target candidate"),
        other => panic!("expected trigger placement, found {other:?}"),
    };
    let decision = pending.observation.clone();
    game.apply(
        PlayerId::Two,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![decision.options[option].id],
        },
    )
    .expect("the target is legal");
    drain_pending(&mut game);

    assert_eq!(game.players[PlayerId::One.index()].life, before - 5);
}

#[test]
fn only_fliers_can_attack_its_controller_but_ground_creatures_can_attack_a_planeswalker() {
    let mut game = form_game();
    let ground = creature(10_200, cards::SAVANNAH_LIONS, PlayerId::One);
    let ground_id = ground.card.id;
    let flier = creature(10_201, cards::SERRA_ANGEL, PlayerId::One);
    let flier_id = flier.card.id;
    let mut planeswalker = creature(10_202, cards::VRASKA_THE_UNSEEN, PlayerId::Two);
    planeswalker.set_counters(CounterKind::Loyalty, 5);
    let walker_id = planeswalker.card.id;
    game.battlefield.extend([ground, flier, planeswalker]);
    game.active_player = PlayerId::One;
    game.priority = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.turns_started[PlayerId::One.index()] = 2;
    let actions = game.legal_actions(PlayerId::One);

    assert!(!actions.contains(&Action::DeclareAttacker {
        attacker: ground_id,
        defender: AttackDefender::Player(PlayerId::Two),
    }));
    assert!(actions.contains(&Action::DeclareAttacker {
        attacker: ground_id,
        defender: AttackDefender::Planeswalker(walker_id),
    }));
    assert!(actions.contains(&Action::DeclareAttacker {
        attacker: flier_id,
        defender: AttackDefender::Player(PlayerId::Two),
    }));
}
