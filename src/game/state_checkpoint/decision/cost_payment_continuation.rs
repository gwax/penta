#[allow(clippy::too_many_arguments)]
fn parse_cost_payment_continuation(
    game: &Game,
    observation: &DecisionObservation,
    payer: PlayerId,
    continuation: &super::model::EffectContinuationSnapshot,
    path: &[usize],
    chosen: &[u32],
    cumulative_upkeep_age: Option<u16>,
) -> Result<DecisionContinuation, String> {
    let continuation = parse_effect_continuation(continuation, game)?;
    match (continuation.effect.effect, cumulative_upkeep_age) {
        (EffectDef::PayOr(definition), None) => {
            if game.effect_players(definition.payment.payer, &continuation.object, &continuation.context, continuation.effect) != [payer] {
                return Err("cost-payment payer disagrees with its authored effect".into());
            }
        }
        (EffectDef::CumulativeUpkeep(_), Some(age)) => {
            let source = continuation.object.source.ok_or("cumulative upkeep has no source")?;
            if payer != continuation.object.controller || !game.battlefield.iter().any(|permanent|
                permanent.card.id == source && permanent.counters(crate::CounterKind::named("age")) == age)
            { return Err("cost-payment upkeep age or payer disagrees with the source".into()); }
        }
        _ => return Err("cost-payment locator is not a supported payment procedure".into()),
    }
    let window = crate::game::cost_payment::CostPaymentWindow {
        player: payer, definition: continuation.effect, object: continuation.object,
        context: continuation.context, path: path.to_vec(), chosen: chosen.iter().copied().map(GameObjectId).collect(),
        cumulative_upkeep_age,
    };
    let offer = game.cost_payment_offer(&window).ok_or("cost-payment path is invalid or unpayable")?;
    if observation.cancellable != offer.cancellable {
        return Err("cost-payment cancellation disagrees with its authored offer".into());
    }
    let mut checked = observation.clone();
    checked.cancellable = false;
    validate_authored_decision(&checked, payer, window.object.ability_text().unwrap_or("Pay the cost?"),
        window.visibility(),
        DecisionPreference::Neutral, offer.count, offer.count, &offer.options, "cost-payment")?;
    Ok(DecisionContinuation::CostPayment(Box::new(window)))
}
