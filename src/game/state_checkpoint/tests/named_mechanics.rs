use super::*;
use crate::game::tests::{card, choose_decision_by_label, ready_game};
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
