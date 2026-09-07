fn parse_cost_payment_continuation(
    game: &Game,
    observation: &DecisionObservation,
    payer: PlayerId,
    continuation: &super::model::EffectContinuationSnapshot,
    path: &[usize],
) -> Result<DecisionContinuation, String> {
    let continuation = parse_effect_continuation(continuation, game)?;
    let EffectDef::PayOr(definition) = continuation.effect.effect else {
        return Err("cost-payment locator is not a pay-or effect".into());
    };
    if game.effect_players(definition.payment.payer, &continuation.object, &continuation.context, continuation.effect) != [payer] {
        return Err("cost-payment payer disagrees with its authored effect".into());
    }
    let window = crate::game::cost_payment::CostPaymentWindow {
        player: payer, definition: continuation.effect, object: continuation.object,
        context: continuation.context, path: path.to_vec(),
    };
    let offer = game.cost_payment_offer(&window).ok_or("cost-payment path is invalid or unpayable")?;
    if observation.cancellable != offer.cancellable {
        return Err("cost-payment cancellation disagrees with its authored offer".into());
    }
    let mut checked = observation.clone();
    checked.cancellable = false;
    validate_authored_decision(&checked, payer, window.object.ability_text().unwrap_or("Pay the cost?"),
        crate::game::decision_offers::effect_choice_visibility(definition.visibility),
        DecisionPreference::Neutral, offer.count, offer.count, &offer.options, "cost-payment")?;
    Ok(DecisionContinuation::CostPayment(Box::new(window)))
}
