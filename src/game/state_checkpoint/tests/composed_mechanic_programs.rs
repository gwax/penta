use super::super::*;
use super::rare_states::assert_reconstructs;
use super::true_hidden_hypothesis;
use crate::game::tests::card;

#[test]
fn composed_mechanic_programs_reconstruct_each_lexical_cost_scope() {
    use crate::game::tests::choose_decision_by_label;
    use crate::game::tests::composed_mechanic_programs::{TWO_PAYMENTS, staged, start};
    let (mut game, _) = staged(&TWO_PAYMENTS);
    start(&mut game);
    assert_reconstructs(&game, "first cost parameter");
    choose_decision_by_label(&mut game, PlayerId::One, "Pay 2 life");
    assert_reconstructs(&game, "second cost parameter sharing the same program body");
    let observation = game.observe(PlayerId::One);
    let actions = crate::protocol::protocol_actions(&observation);
    let wire = crate::protocol::observation_json_for_format(
        &game.catalog,
        game.format,
        &observation,
        game.in_pregame(),
        &actions,
    );
    let hidden = true_hidden_hypothesis(&game, PlayerId::One);
    let mut restored =
        Game::from_observation_checkpoint(game.catalog.clone(), game.format, &wire, &hidden, 4242)
            .unwrap();
    choose_decision_by_label(&mut restored, PlayerId::One, "Pay 4 life");
    assert_eq!(restored.players[0].life, 20);
    let mut tampered = wire.clone();
    tampered["checkpoint"]["decisionState"]["continuation"]["paymentProvenance"] =
        serde_json::json!("cycling");
    assert!(
        Game::from_observation_checkpoint(
            game.catalog.clone(),
            game.format,
            &tampered,
            &hidden,
            4242
        )
        .is_err()
    );
}

#[test]
fn composed_mechanic_programs_payment_completion_waits_for_replaced_draws() {
    use crate::game::tests::choose_decision_by_label;
    use crate::game::tests::composed_mechanic_programs::{DRAW_PAYMENT, staged, start};
    let (mut game, _) = staged(&DRAW_PAYMENT);
    game.put_onto_battlefield(PlayerId::One, crate::card::cards::ISLAND_SANCTUARY)
        .unwrap();
    game.players[0].library = vec![
        card(31_111, crate::card::cards::PLAINS, PlayerId::One),
        card(31_112, crate::card::cards::ISLAND, PlayerId::One),
    ];
    start(&mut game);
    game.step = Step::Draw;
    choose_decision_by_label(&mut game, PlayerId::One, "Draw 2 card(s), Pay 2 life");
    assert_eq!(
        game.players[0].life, 18,
        "the paid branch has not run while a draw awaits a choice"
    );
    assert!(
        game.pending_procedures
            .iter()
            .any(|p| matches!(p, crate::game::PendingProcedure::CompletePayment { .. }))
    );
    assert_reconstructs(
        &game,
        "a mixed cost-list payment suspended inside its first draw",
    );
    let observation = game.observe(PlayerId::One);
    let actions = crate::protocol::protocol_actions(&observation);
    let wire = crate::protocol::observation_json_for_format(
        &game.catalog,
        game.format,
        &observation,
        game.in_pregame(),
        &actions,
    );
    let hidden = true_hidden_hypothesis(&game, PlayerId::One);
    let mut restored =
        Game::from_observation_checkpoint(game.catalog.clone(), game.format, &wire, &hidden, 4242)
            .unwrap();
    choose_decision_by_label(&mut restored, PlayerId::One, "Draw the card");
    assert_eq!(restored.players[0].life, 18);
    choose_decision_by_label(&mut restored, PlayerId::One, "Draw the card");
    assert_eq!(
        restored.players[0].life, 21,
        "the paid branch follows both completed draws"
    );
    assert_eq!(restored.players[0].hand.len(), 2);
}

#[test]
fn composed_mechanic_programs_reconstruct_batch_choices() {
    use crate::game::tests::choose_decision_by_label;
    use crate::game::tests::composed_mechanic_programs::{CHOICE_BATCH, staged, start};
    let (mut game, _) = staged(&CHOICE_BATCH);
    game.players[0].life = 3;
    game.add_unrestricted_mana(PlayerId::One, crate::card::ManaColor::Colorless, 1);
    start(&mut game);
    assert_reconstructs(&game, "a complete mixed batch payment");
    let observation = game.observe(PlayerId::One);
    let actions = crate::protocol::protocol_actions(&observation);
    let wire = crate::protocol::observation_json_for_format(
        &game.catalog,
        game.format,
        &observation,
        game.in_pregame(),
        &actions,
    );
    let hidden = true_hidden_hypothesis(&game, PlayerId::One);
    let mut restored =
        Game::from_observation_checkpoint(game.catalog.clone(), game.format, &wire, &hidden, 4242)
            .unwrap();
    choose_decision_by_label(&mut restored, PlayerId::One, "Pay {1}, Pay 2 life");
    assert_eq!(restored.players[0].life, 4);
}

#[test]
fn composed_mechanic_programs_reconstruct_cost_local_repetition_without_a_label() {
    use crate::game::tests::choose_decision_by_label;
    use crate::game::tests::composed_mechanic_programs::{PARTIALLY_REPEATED, staged, start};
    let (mut game, id) = staged(&PARTIALLY_REPEATED);
    game.battlefield
        .iter_mut()
        .find(|p| p.card.id == id)
        .unwrap()
        .set_counters(crate::card::CounterKind::named("age"), 2);
    start(&mut game);
    assert_reconstructs(&game, "a repeated sub-list and a once-only sibling");
    let observation = game.observe(PlayerId::One);
    let actions = crate::protocol::protocol_actions(&observation);
    let wire = crate::protocol::observation_json_for_format(
        &game.catalog,
        game.format,
        &observation,
        game.in_pregame(),
        &actions,
    );
    let hidden = true_hidden_hypothesis(&game, PlayerId::One);
    let mut restored =
        Game::from_observation_checkpoint(game.catalog.clone(), game.format, &wire, &hidden, 4242)
            .unwrap();
    choose_decision_by_label(&mut restored, PlayerId::One, "Pay 5 life");
    assert_eq!(restored.players[0].life, 18);
}
