use super::*;
use crate::game::tests::{card, choose_decision_by_label, creature, ready_game};
use crate::poc::cards;

fn staged() -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.put_onto_battlefield(PlayerId::One, cards::CORPSEBERRY_CULTIVATOR)
        .unwrap();
    game.players[0].graveyard.clear();
    for id in 160_000..160_004 {
        game.players[0]
            .graveyard
            .push(card(id, cards::LIGHTNING_BOLT, PlayerId::One));
    }
    game.step = crate::game::Step::BeginningOfCombat;
    game.capture_battlefield_triggers(&crate::game::CommittedTriggerEvent::StepBegins {
        step: crate::card::TurnStepDef::BeginningOfCombat,
        player: PlayerId::One,
    });
    game.finish_rules_procedure();
    game.resolve_stack_top();
    game
}

fn wire(game: &Game, viewer: PlayerId) -> Value {
    let observation = game.observe(viewer);
    crate::protocol::observation_json_for_format(
        &game.catalog,
        game.format,
        &observation,
        game.in_pregame(),
        &crate::protocol::protocol_actions(&observation),
    )
}

#[test]
fn named_mechanics_payment_choices_round_trip_without_hidden_information() {
    for leaf in [false, true] {
        let mut game = staged();
        if leaf {
            choose_decision_by_label(
                &mut game,
                PlayerId::One,
                "Exile 3 card(s) from your graveyard",
            );
        }
        super::rare_states::assert_reconstructs(&game, "forage cost choice");
        let snapshot = wire(&game, PlayerId::One);
        let mut rebuilt = Game::from_observation_checkpoint(
            game.catalog.clone(),
            game.format,
            &snapshot,
            &true_hidden_hypothesis(&game, PlayerId::One),
            55,
        )
        .unwrap();
        if !leaf {
            for current in [&mut game, &mut rebuilt] {
                choose_decision_by_label(
                    current,
                    PlayerId::One,
                    "Exile 3 card(s) from your graveyard",
                );
            }
        }
        for current in [&mut game, &mut rebuilt] {
            let decision = current
                .pending_decisions
                .last()
                .unwrap()
                .observation
                .clone();
            current
                .apply(
                    PlayerId::One,
                    Action::ChooseDecision {
                        decision: decision.id,
                        options: decision
                            .options
                            .iter()
                            .take(3)
                            .map(|option| option.id)
                            .collect(),
                    },
                )
                .unwrap();
            crate::game::tests::drain_pending(current);
            assert_eq!(current.players[0].exile.len(), 3);
            let cultivator = current
                .battlefield
                .iter()
                .find(|permanent| permanent.card.definition == cards::CORPSEBERRY_CULTIVATOR)
                .unwrap();
            assert_eq!(
                cultivator.counters(crate::card::CounterKind::PlusOnePlusOne),
                1
            );
        }
    }
}

#[test]
fn named_mechanics_checkpoint_rejects_invalid_cost_path_and_payer() {
    let game = staged();
    let snapshot = wire(&game, PlayerId::One);
    for field in ["path", "player"] {
        let mut invalid = snapshot.clone();
        invalid["checkpoint"]["decisionState"]["continuation"][field] = if field == "path" {
            json!([99])
        } else {
            json!(1)
        };
        assert!(
            Game::from_observation_checkpoint(
                game.catalog.clone(),
                game.format,
                &invalid,
                &true_hidden_hypothesis(&game, PlayerId::One),
                55
            )
            .is_err()
        );
    }
}

#[test]
fn object_costs_upkeep_checkpoint_preserves_age_and_cancel_does_not_discard() {
    let mut game = ready_game();
    let source = game
        .put_onto_battlefield(PlayerId::One, cards::VEXING_SPHINX)
        .unwrap();
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == source)
        .unwrap()
        .set_counters(crate::CounterKind::named("age"), 1);
    for id in 160_020..160_023 {
        game.players[0]
            .hand
            .push(card(id, cards::ISLAND, PlayerId::One));
    }
    game.step = crate::game::Step::Upkeep;
    game.capture_battlefield_triggers(&crate::game::CommittedTriggerEvent::StepBegins {
        step: crate::card::TurnStepDef::Upkeep,
        player: PlayerId::One,
    });
    game.finish_rules_procedure();
    game.resolve_stack_top();
    let snapshot = wire(&game, PlayerId::One);
    super::rare_states::assert_reconstructs(&game, "unpaid cumulative-upkeep object selection");
    let mut rebuilt = Game::from_observation_checkpoint(
        game.catalog.clone(),
        game.format,
        &snapshot,
        &true_hidden_hypothesis(&game, PlayerId::One),
        55,
    )
    .unwrap();
    for field in ["cumulativeUpkeepAge", "chosen"] {
        let mut invalid = snapshot.clone();
        invalid["checkpoint"]["decisionState"]["continuation"][field] = if field == "chosen" {
            json!([160_020])
        } else {
            json!(1)
        };
        assert!(
            Game::from_observation_checkpoint(
                game.catalog.clone(),
                game.format,
                &invalid,
                &true_hidden_hypothesis(&game, PlayerId::One),
                55
            )
            .is_err()
        );
    }
    for current in [&mut game, &mut rebuilt] {
        assert_eq!(
            current
                .battlefield
                .iter()
                .find(|permanent| permanent.card.id == source)
                .unwrap()
                .counters(crate::CounterKind::named("age")),
            2
        );
        let decision = current.pending_decisions[0].observation.clone();
        assert_eq!((decision.minimum, decision.maximum), (2, 2));
        current
            .apply(
                PlayerId::One,
                Action::CancelDecision {
                    decision: decision.id,
                },
            )
            .unwrap();
        assert_eq!(
            current.players[0].hand.len(),
            3,
            "cancellation discards nothing"
        );
        assert!(
            !current
                .battlefield
                .iter()
                .any(|permanent| permanent.card.id == source),
            "the unpaid upkeep sacrifices its source"
        );
    }
}

/// Tentative aggregate choices survive reconstruction without sacrificing anything.
#[test]
#[allow(clippy::too_many_lines)]
fn object_costs_tentative_aggregate_selection_reconstructs_and_rejects_tampering() {
    let mut game = ready_game();
    for index in 0..3 {
        game.battlefield.push(creature(
            10_010 + index,
            crate::card::cards::SERRA_ANGEL,
            PlayerId::One,
        ));
    }
    let dreadnought = card(
        10_000,
        crate::card::cards::PHYREXIAN_DREADNOUGHT,
        PlayerId::One,
    );
    let dreadnought_id = dreadnought.id;
    game.players[PlayerId::One.index()].hand.push(dreadnought);
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        crate::game::tests::cast_action(dreadnought_id, Vec::new(), Vec::new(), 0),
    )
    .expect("one mana casts it");
    crate::game::tests::pass_until_decision(&mut game);

    let offer = game
        .observe(PlayerId::One)
        .decision
        .expect("the payer is asked whether to pay");
    let pay = offer
        .options
        .iter()
        .find(|option| option.id != 0)
        .expect("paying is on offer")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: offer.id,
            options: vec![pay],
        },
    )
    .expect("paying is legal");

    let (wire, mut rebuilt) = super::rebuild_current_checkpoint(&game, PlayerId::One, 11);
    let chosen = match rebuilt
        .pending_decisions
        .first()
        .map(|pending| &pending.continuation)
    {
        Some(DecisionContinuation::CostPayment(window)) => &window.chosen,
        other => panic!("the tentative selection came back as {other:?}"),
    };
    assert_eq!(
        chosen.len(),
        1,
        "the first choice was preserved without sacrifice"
    );
    assert!(
        rebuilt
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == chosen[0])
    );
    let chosen_id = chosen[0];
    for invalid_chosen in [
        serde_json::json!([chosen_id.0, chosen_id.0]),
        serde_json::json!([999_999]),
    ] {
        let mut invalid = wire.clone();
        invalid["checkpoint"]["decisionState"]["continuation"]["chosen"] = invalid_chosen;
        assert!(
            Game::from_observation_checkpoint(
                game.catalog.clone(),
                game.format,
                &invalid,
                &super::true_hidden_hypothesis(&game, PlayerId::One),
                11
            )
            .is_err()
        );
    }
    for current in [&mut game, &mut rebuilt] {
        let decision = current.pending_decisions[0].observation.clone();
        current
            .apply(
                PlayerId::One,
                Action::CancelDecision {
                    decision: decision.id,
                },
            )
            .unwrap();
        assert!(
            current
                .battlefield
                .iter()
                .any(|permanent| permanent.card.id == chosen_id),
            "cancelling did not sacrifice a tentative member"
        );
        assert_eq!(
            current.battlefield.len(),
            3,
            "only the unpaid Dreadnought leaves"
        );
    }
}
