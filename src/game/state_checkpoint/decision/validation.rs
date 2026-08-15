fn resolved_effect_payment(
    game: &Game,
    payment: EffectPaymentDef,
    object: &super::super::StackObject,
    context: &super::super::EffectResolutionContext,
    scoped: ScopedEffect,
) -> Option<(PlayerId, super::super::ResolvedEffectPayment)> {
    let payers = game.effect_players(payment.payer, object, context, scoped);
    let [player] = payers.as_slice() else {
        return None;
    };
    let payment = match payment.cost {
        EffectPaymentCostDef::Mana(cost) => super::super::ResolvedEffectPayment::Mana(cost),
        EffectPaymentCostDef::GenericMana(amount) => {
            let amount = game
                .effect_value(amount, object, context, scoped)
                .max(0)
                .try_into()
                .unwrap_or(u16::MAX);
            super::super::ResolvedEffectPayment::Mana(ManaCost::new(amount, 0))
        }
        EffectPaymentCostDef::ColoredMana { color, amount } => {
            let amount = game
                .effect_value(amount, object, context, scoped)
                .max(0)
                .try_into()
                .unwrap_or(u16::MAX);
            super::super::ResolvedEffectPayment::Mana(ManaCost::of_color(color, amount))
        }
        EffectPaymentCostDef::Life(amount) => super::super::ResolvedEffectPayment::Life(amount),
        EffectPaymentCostDef::Mill(amount) => super::super::ResolvedEffectPayment::Mill(amount),
        EffectPaymentCostDef::Discard(amount) => {
            super::super::ResolvedEffectPayment::Discard(amount)
        }
    };
    Some((*player, payment))
}

fn payment_decision_options(
    payment: super::super::ResolvedEffectPayment,
    can_pay: bool,
    decline: &str,
) -> Vec<DecisionOption> {
    let mut options = vec![DecisionOption {
        id: 0,
        label: decline.into(),
        card: None,
        members: Vec::new(),
        ability_text: None,
        zone: DecisionZone::None,
    }];
    if can_pay {
        options.push(DecisionOption {
            id: 1,
            label: Game::effect_payment_label(payment),
            card: None,
            members: Vec::new(),
            ability_text: None,
            zone: DecisionZone::None,
        });
    }
    options
}

#[allow(clippy::too_many_arguments)]
fn validate_top_card_selection_observation(
    game: &Game,
    observation: &DecisionObservation,
    player: PlayerId,
    revealed: &[super::super::CardInstance],
    selection: &'static crate::card::TopCardSelectionDef,
    object: &super::super::StackObject,
    context: &super::super::EffectResolutionContext,
    scoped: ScopedEffect,
) -> Result<(), String> {
    let requested = game
        .effect_value(selection.count, object, context, scoped)
        .max(0);
    let requested = usize::try_from(requested).unwrap_or(usize::MAX);
    let available_before_inspection = game.players[player.index()]
        .library
        .len()
        .saturating_add(revealed.len());
    if revealed.len() != requested.min(available_before_inspection) {
        return Err("top-card selection inspected count disagrees with its authored effect".into());
    }
    let source = object.source.unwrap_or(object.id);
    let eligible = revealed
        .iter()
        .filter(|card| {
            selection.object.is_none_or(|predicate| {
                game.card_object_matches(predicate, card, crate::card::ZoneKind::Library, source)
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    let inspected = revealed
        .iter()
        .map(|card| (card.id, card.definition))
        .collect::<Vec<_>>();
    let mut expected = game.card_decision_options(&eligible, DecisionZone::Library);
    for option in &mut expected {
        option.members.clone_from(&inspected);
    }
    let no_selection = expected.is_empty();
    if no_selection {
        expected.push(DecisionOption {
            id: 0,
            label: "No inspected card is eligible".into(),
            card: None,
            members: inspected,
            ability_text: None,
            zone: DecisionZone::Library,
        });
    }
    let (minimum, maximum, preference) = if no_selection {
        (0, 0, DecisionPreference::Neutral)
    } else {
        (
            usize::from(selection.minimum).min(expected.len()),
            usize::from(selection.maximum),
            if selection.selected_zone == crate::card::ZoneKind::Hand {
                DecisionPreference::HigherCardValue
            } else {
                DecisionPreference::LowerCardValue
            },
        )
    };
    validate_authored_decision(
        observation,
        player,
        "Choose cards from the top of the library",
        DecisionVisibility::Private,
        preference,
        minimum,
        maximum,
        &expected,
        "top-card selection",
    )
}

#[allow(clippy::too_many_arguments)]
fn validate_authored_decision(
    observation: &DecisionObservation,
    player: PlayerId,
    prompt: &str,
    visibility: DecisionVisibility,
    preference: DecisionPreference,
    minimum: usize,
    maximum: usize,
    options: &[DecisionOption],
    description: &str,
) -> Result<(), String> {
    let expected_minimum = minimum.min(options.len());
    let expected_maximum = maximum.max(expected_minimum);
    let mismatch = if observation.player != player {
        "player"
    } else if observation.kind != DecisionKind::Choice || observation.order_semantics.is_some() {
        "kind"
    } else if observation.prompt != prompt {
        "prompt"
    } else if observation.visibility != visibility {
        "visibility"
    } else if observation.preference != preference {
        "preference"
    } else if observation.minimum != expected_minimum || observation.maximum != expected_maximum {
        "bounds"
    } else if observation.cancellable {
        "cancellability"
    } else if observation.options != options {
        return Err(format!(
            "{description} decision options disagree with its authored effect: observed {:?}, expected {options:?}",
            observation.options,
        ));
    } else {
        return Ok(());
    };
    Err(format!(
        "{description} decision {mismatch} disagrees with its authored effect"
    ))
}

fn validate_exact_partition(
    authored: &[Target],
    first: &[Target],
    second: &[Target],
) -> Result<(), String> {
    let combined = first.iter().chain(second).copied().collect::<Vec<_>>();
    if combined.len() != authored.len()
        || combined
            .iter()
            .enumerate()
            .any(|(index, item)| combined[..index].contains(item))
        || combined.iter().any(|item| !authored.contains(item))
        || authored.iter().any(|item| !combined.contains(item))
    {
        return Err(
            "pile-choice checkpoint is not an exact disjoint partition of authored items".into(),
        );
    }
    let canonical_first = authored
        .iter()
        .filter(|item| first.contains(item))
        .copied()
        .collect::<Vec<_>>();
    let canonical_second = authored
        .iter()
        .filter(|item| second.contains(item))
        .copied()
        .collect::<Vec<_>>();
    if canonical_first != first || canonical_second != second {
        return Err("pile-choice checkpoint changed the authored item order".into());
    }
    Ok(())
}

fn ability_locator_matches_origin(
    locator: &AbilityLocator,
    object: &super::super::StackObject,
) -> bool {
    let Some(payload) = &object.ability else {
        return false;
    };
    let expected = match payload.origin {
        super::super::AbilityOrigin::Printed {
            definition,
            part,
            ability,
        } => (definition.0, part.0, ability.0),
        super::super::AbilityOrigin::Granted {
            source_definition,
            source_part,
            source_ability,
            ..
        } => (source_definition.0, source_part.0, source_ability.0),
        super::super::AbilityOrigin::IntrinsicBasicLand(_) => return false,
    };
    (locator.definition, locator.part_id, locator.ability_id) == expected
}

fn validate_entry_decision_context(
    game: &Game,
    context: ReplacementEffectContext,
    locator: &ReplacementEffectLocator,
) -> Result<(), String> {
    if !replacement_effect_locator_matches_source(locator, context.source) {
        return Err("entry decision locator disagrees with its replacement source".into());
    }
    let pending = game
        .pending_events
        .front()
        .ok_or("entry decision lacks its pending event")?;
    if !pending.applied.contains(&context.source)
        || Game::pending_event_controller(pending) != context.controller
    {
        return Err("entry decision context disagrees with its pending event".into());
    }
    Ok(())
}
