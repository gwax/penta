#[allow(clippy::too_many_lines)]
fn parse_continuation(
    value: &DecisionContinuationSnapshot,
    observation: &DecisionObservation,
    hidden: &Value,
    game: &Game,
) -> Result<DecisionContinuation, String> {
    Ok(match value {
        DecisionContinuationSnapshot::BeginTurn {
            player: prospective_player,
            turn_kind,
            applied,
            replacements,
            deferred,
        } => DecisionContinuation::BeginTurn {
            player: player(*prospective_player)?,
            kind: parse_turn_kind(*turn_kind),
            applied: applied.iter().copied().map(parse_ability_source).collect(),
            replacements: replacements
                .iter()
                .map(|replacement| parse_begin_turn_replacement(replacement, game))
                .collect::<Result<Vec<_>, _>>()?,
            deferred: deferred
                .iter()
                .map(|effect| parse_deferred_begin_turn_effect(effect, game))
                .collect::<Result<Vec<_>, _>>()?,
        },
        DecisionContinuationSnapshot::SearchZone {
            controller,
            source,
            destination,
            placement,
            reveal,
            shuffle,
            enters_tapped,
        } => DecisionContinuation::SearchZone {
            controller: player(*controller)?,
            source: parse_zone_kind(*source),
            destination: parse_zone_kind(*destination),
            placement: parse_zone_placement(*placement),
            reveal: *reveal,
            shuffle: *shuffle,
            enters_tapped: *enters_tapped,
        },
        DecisionContinuationSnapshot::ChooseCards {
            controller,
            destination,
            placement,
            reveal,
        } => DecisionContinuation::ChooseCards {
            controller: player(*controller)?,
            destination: parse_zone_kind(*destination),
            placement: parse_zone_placement(*placement),
            reveal: *reveal,
        },
        DecisionContinuationSnapshot::DrawReplacement {
            player: owner,
            replacements,
        } => DecisionContinuation::DrawReplacement {
            player: player(*owner)?,
            replacements: replacements
                .iter()
                .map(|replacement| parse_draw_replacement(replacement, game))
                .collect::<Result<Vec<_>, _>>()?,
        },
        DecisionContinuationSnapshot::DiscardForEffect {
            player: current,
            amount,
            remaining,
            chosen,
            cause,
        } => DecisionContinuation::DiscardForEffect {
            player: player(*current)?,
            amount: *amount,
            remaining: remaining
                .iter()
                .copied()
                .map(player)
                .collect::<Result<Vec<_>, _>>()?,
            chosen: chosen
                .iter()
                .map(|choice| {
                    let owner = player(choice.player)?;
                    let cards = match &choice.cards {
                        Some(cards) => game_ids(cards),
                        None => hidden_discard_choices(hidden, owner, choice.count, game)?,
                    };
                    Ok((owner, cards))
                })
                .collect::<Result<Vec<_>, String>>()?,
            cause: parse_cause(*cause)?,
        },
        DecisionContinuationSnapshot::BasicLandTypeTextChange { target } => {
            DecisionContinuation::BasicLandTypeTextChange {
                target: parse_target(*target),
            }
        }
        DecisionContinuationSnapshot::GrislySalvage {
            player: owner,
            revealed,
        } => DecisionContinuation::GrislySalvage {
            player: player(*owner)?,
            revealed: parse_detached_cards(revealed, game)?,
        },
        DecisionContinuationSnapshot::AugurOfBolas {
            player: owner,
            revealed,
        } => DecisionContinuation::AugurOfBolas {
            player: player(*owner)?,
            revealed: parse_detached_cards(revealed, game)?,
        },
        DecisionContinuationSnapshot::TopCardSelection {
            player: owner,
            revealed,
            continuation,
        } => {
            let owner = player(*owner)?;
            let continuation = parse_effect_continuation(continuation, game)?;
            let EffectDef::LookAtTopAndSelect {
                player: recipient,
                looker,
                selection,
            } = continuation.effect.effect
            else {
                return Err("top-card selection locator is not a top-card selection".into());
            };
            let resolve = |recipient| {
                game.effect_recipients(
                    recipient,
                    &continuation.object,
                    &continuation.context,
                    continuation.effect,
                )
            };
            // The library belongs to one player and the decision is shown to
            // another whenever a spy is looking, so the two are checked
            // against the two authored recipients rather than each other.
            if resolve(recipient).as_slice() != [Target::Player(owner)] {
                return Err("top-card selection player disagrees with its authored effect".into());
            }
            if resolve(looker).as_slice() != [Target::Player(observation.player)] {
                return Err("top-card selection looker disagrees with the visible decision".into());
            }
            let revealed = parse_detached_cards(revealed, game)?;
            validate_top_card_selection_observation(
                game,
                observation,
                owner,
                &revealed,
                selection,
                &continuation.object,
                &continuation.context,
                continuation.effect,
            )?;
            DecisionContinuation::TopCardSelection {
                player: owner,
                revealed,
                selection,
                object: continuation.object,
                context: continuation.context,
                effect: continuation.effect,
            }
        }
        DecisionContinuationSnapshot::ChainLightning {
            player: owner,
            spell,
            targets,
        } => DecisionContinuation::ChainLightning {
            player: player(*owner)?,
            spell: parse_detached_stack(spell, game)?,
            targets: targets.iter().copied().map(parse_target).collect(),
        },
        DecisionContinuationSnapshot::Fork {
            player: owner,
            spell,
            target_lists,
        } => DecisionContinuation::Fork {
            player: player(*owner)?,
            spell: parse_detached_stack(spell, game)?,
            target_lists: target_lists
                .iter()
                .map(|targets| {
                    targets
                        .iter()
                        .map(parse_target_selection)
                        .collect::<Result<Vec<_>, _>>()
                })
                .collect::<Result<Vec<_>, _>>()?,
        },
        DecisionContinuationSnapshot::OptionalEffect {
            object,
            ability,
            context,
            effect,
        } => DecisionContinuation::OptionalEffect {
            object: Box::new(parse_detached_stack(object, game)?),
            context: parse_effect_resolution_context(context.clone())?,
            effect: catalog_scoped_effect(&game.catalog, ability, effect)
                .ok_or("optional effect locator is absent from this catalog")?,
        },
        DecisionContinuationSnapshot::ChooseForEffect {
            continuation: snapshot,
        } => {
            let continuation = parse_effect_continuation(snapshot, game)?;
            if !ability_locator_matches_origin(&snapshot.ability, &continuation.object) {
                return Err("object-choice locator disagrees with its resolving ability".into());
            }
            let EffectDef::Choose(definition) = continuation.effect.effect else {
                return Err("object-choice locator does not identify an authored choice".into());
            };
            let state = game
                .effect_choice_decision_state(
                    definition,
                    &continuation.object,
                    &continuation.context,
                    continuation.effect,
                )
                .ok_or("object-choice authored chooser is not singular")?;
            if definition.minimum > 0 && state.candidates.len() <= definition.minimum {
                return Err(
                    "object-choice checkpoint encodes a choice that would resolve automatically"
                        .into(),
                );
            }
            validate_authored_decision(
                observation,
                state.chooser,
                "Choose objects",
                effect_choice_visibility(definition.visibility),
                state.preference,
                state.minimum,
                state.maximum,
                &state.options,
                "object choice",
            )?;
            DecisionContinuation::ChooseForEffect {
                definition: continuation.effect,
                binding: definition.binding,
                object: continuation.object,
                context: continuation.context,
                candidates: state.candidates,
                effect: continuation.effect.with_effect(*definition.then),
            }
        }
        DecisionContinuationSnapshot::PayOr {
            player: payer,
            payment,
            object,
            ability,
            context,
            definition,
        } => {
            let payer = player(*payer)?;
            if payer != observation.player {
                return Err("pay-or payer disagrees with the visible decision".into());
            }
            let payment = parse_resolved_effect_payment(payment)?;
            let object = Box::new(parse_detached_stack(object, game)?);
            let context = parse_effect_resolution_context(context.clone())?;
            if !ability_locator_matches_origin(ability, &object) {
                return Err("pay-or locator disagrees with its resolving ability".into());
            }
            let scoped = catalog_scoped_effect(&game.catalog, ability, definition)
                .ok_or("pay-or locator is absent from this catalog")?;
            let EffectDef::PayOr(authored) = scoped.effect else {
                return Err("pay-or locator does not identify an optional payment".into());
            };
            let expected =
                resolved_effect_payment(game, authored.payment, &object, &context, scoped)
                    .ok_or("pay-or authored payment no longer has exactly one payer")?;
            if expected != (payer, payment) {
                return Err("pay-or payer or payment disagrees with its authored effect".into());
            }
            let can_pay = game.can_pay_effect_payment(payer, payment);
            if authored.if_paid.is_none() && authored.otherwise.is_none()
                || (!can_pay && authored.otherwise.is_some())
            {
                return Err(
                    "pay-or checkpoint encodes a choice that would resolve automatically".into(),
                );
            }
            let options = payment_decision_options(payment, can_pay, "Decline");
            validate_authored_decision(
                observation,
                payer,
                object.ability_text().unwrap_or("Pay the cost?"),
                effect_choice_visibility(authored.visibility),
                DecisionPreference::Neutral,
                1,
                1,
                &options,
                "pay-or",
            )?;
            DecisionContinuation::PayOr {
                player: payer,
                payment,
                definition: scoped,
                object,
                context,
                if_paid: authored.if_paid.map(|effect| scoped.with_effect(*effect)),
                otherwise: authored.otherwise.map(|effect| scoped.with_effect(*effect)),
            }
        }
        DecisionContinuationSnapshot::SplitForEffect {
            continuation: snapshot,
        } => {
            let continuation = parse_effect_continuation(snapshot, game)?;
            if !ability_locator_matches_origin(&snapshot.ability, &continuation.object) {
                return Err("pile-split locator disagrees with its resolving ability".into());
            }
            let EffectDef::SplitIntoPiles(definition) = continuation.effect.effect else {
                return Err("pile-split locator does not identify an authored partition".into());
            };
            let state = game
                .effect_pile_split_state(
                    definition,
                    &continuation.object,
                    &continuation.context,
                    continuation.effect,
                )
                .ok_or("pile-split authored divider or chooser is not singular")?;
            validate_authored_decision(
                observation,
                state.divider,
                "Separate the objects into two piles",
                DecisionVisibility::Public,
                DecisionPreference::BalancedPartition,
                0,
                state.items.len(),
                &state.options,
                "pile split",
            )?;
            DecisionContinuation::SplitForEffect {
                definition: continuation.effect,
                chooser: state.chooser,
                items: state.items,
                object: continuation.object,
                context: continuation.context,
            }
        }
        DecisionContinuationSnapshot::ChoosePileForEffect {
            first,
            second,
            continuation: snapshot,
        } => {
            let continuation = parse_effect_continuation(snapshot, game)?;
            if !ability_locator_matches_origin(&snapshot.ability, &continuation.object) {
                return Err("pile-choice locator disagrees with its resolving ability".into());
            }
            let EffectDef::SplitIntoPiles(definition) = continuation.effect.effect else {
                return Err("pile-choice locator does not identify an authored partition".into());
            };
            let authored = game
                .effect_pile_split_state(
                    definition,
                    &continuation.object,
                    &continuation.context,
                    continuation.effect,
                )
                .ok_or("pile-choice authored divider or chooser is not singular")?;
            let first = first.iter().copied().map(parse_target).collect::<Vec<_>>();
            let second = second.iter().copied().map(parse_target).collect::<Vec<_>>();
            validate_exact_partition(&authored.items, &first, &second)?;
            let state =
                game.effect_pile_choice_state(&first, &second, definition, continuation.effect);
            validate_authored_decision(
                observation,
                authored.chooser,
                "Choose a pile",
                DecisionVisibility::Public,
                state.preference,
                1,
                1,
                &state.options,
                "pile choice",
            )?;
            DecisionContinuation::ChoosePileForEffect {
                definition: continuation.effect,
                first,
                second,
                chosen: definition.chosen,
                unchosen: definition.unchosen,
                object: continuation.object,
                context: continuation.context,
                effect: continuation.effect.with_effect(*definition.then),
            }
        }
        DecisionContinuationSnapshot::BattlefieldEntryPayment {
            context,
            player: payer,
            payment,
            effect,
        } => {
            let context = parse_replacement_context(*context)?;
            validate_entry_decision_context(game, context, effect)?;
            let definition = catalog_replacement_effect(&game.catalog, effect)
                .ok_or("battlefield entry payment locator is absent from this catalog")?;
            let ReplacementEffectDef::PayOr { .. } = definition else {
                return Err("battlefield entry payment locator is not an optional payment".into());
            };
            let payer = player(*payer)?;
            let payment = parse_resolved_effect_payment(payment)?;
            let pending = game
                .pending_events
                .front()
                .ok_or("battlefield entry payment lacks its pending event")?;
            if payer != observation.player
                || game.pending_resolved_payment(
                    pending,
                    context,
                    match definition {
                        ReplacementEffectDef::PayOr { payment, .. } => payment,
                        _ => unreachable!(),
                    },
                ) != Some((payer, payment))
            {
                return Err(
                    "battlefield entry payer or payment disagrees with its authored effect".into(),
                );
            }
            if !game.can_pay_effect_payment(payer, payment) {
                return Err("battlefield entry payment is no longer payable".into());
            }
            let name = game.pending_entry_name(pending);
            let payment_label = Game::effect_payment_label(payment);
            let options = payment_decision_options(payment, true, "Do not pay");
            validate_authored_decision(
                observation,
                payer,
                &format!("{payment_label} as {name} enters the battlefield?"),
                DecisionVisibility::Public,
                DecisionPreference::Neutral,
                1,
                1,
                &options,
                "battlefield entry payment",
            )?;
            DecisionContinuation::BattlefieldEntryPayment {
                context,
                player: payer,
                payment,
                definition,
            }
        }
        DecisionContinuationSnapshot::BattlefieldEntryReplacement { candidates } => {
            DecisionContinuation::BattlefieldEntryReplacement {
                candidates: candidates
                    .iter()
                    .map(|candidate| parse_applicable_replacement(candidate, &game.catalog))
                    .collect::<Result<Vec<_>, _>>()?,
            }
        }
        DecisionContinuationSnapshot::BattlefieldEntryOptional { context, effect } => {
            let context = parse_replacement_context(*context)?;
            validate_entry_decision_context(game, context, effect)?;
            let definition = catalog_replacement_effect(&game.catalog, effect)
                .ok_or("optional entry replacement locator is absent from this catalog")?;
            let pending = game
                .pending_events
                .front()
                .ok_or("optional entry replacement lacks its pending event")?;
            let mut before_selection = pending.clone();
            before_selection
                .applied
                .retain(|source| *source != context.source);
            let candidate = game
                .applicable_replacements(&before_selection)
                .into_iter()
                .find(|candidate| candidate.context == context && candidate.effect == definition)
                .ok_or("optional entry replacement is not applicable to its pending event")?;
            if !candidate.optional {
                return Err("optional entry replacement locator names a mandatory ability".into());
            }
            let owner = Game::pending_event_controller(pending);
            let name = game.pending_entry_name(pending);
            validate_authored_decision(
                observation,
                owner,
                &format!("Apply the optional replacement for {name}?"),
                DecisionVisibility::Public,
                DecisionPreference::Neutral,
                1,
                1,
                &Game::optional_entry_replacement_options(),
                "optional entry replacement",
            )?;
            DecisionContinuation::BattlefieldEntryOptional {
                context,
                effect: definition,
            }
        }
        DecisionContinuationSnapshot::BattlefieldEntryScalarChoice {
            context,
            effect,
            choices,
        } => {
            let context = parse_replacement_context(*context)?;
            validate_entry_decision_context(game, context, effect)?;
            let ReplacementEffectDef::Choose(ReplacementChoiceDef::Scalar(choice)) =
                catalog_replacement_effect(&game.catalog, effect)
                    .ok_or("entry scalar choice locator is absent from this catalog")?
            else {
                return Err("entry scalar choice locator is not a scalar choice".into());
            };
            let pending = game
                .pending_events
                .front()
                .ok_or("entry scalar choice lacks its pending event")?;
            let owner = Game::pending_event_controller(pending);
            let (prompt, authored_choices) = game.entry_scalar_choices(owner, choice);
            if *choices != authored_choices {
                return Err(
                    "entry scalar choice vocabulary disagrees with its authored choice".into(),
                );
            }
            let options = authored_choices
                .iter()
                .enumerate()
                .map(|(index, label)| DecisionOption {
                    id: u32::try_from(index).unwrap_or(u32::MAX),
                    label: label.clone(),
                    card: None,
                    members: Vec::new(),
                    ability_text: None,
                    zone: DecisionZone::None,
                })
                .collect::<Vec<_>>();
            validate_authored_decision(
                observation,
                owner,
                prompt,
                DecisionVisibility::Public,
                DecisionPreference::Neutral,
                1,
                1,
                &options,
                "entry scalar choice",
            )?;
            DecisionContinuation::BattlefieldEntryScalarChoice {
                context,
                choice,
                choices: choices.clone(),
            }
        }
        DecisionContinuationSnapshot::BattlefieldEntryCopy {
            choices,
            added_types,
        } => DecisionContinuation::BattlefieldEntryCopy {
            choices: game_ids(choices),
            added_types: parse_card_type_set(*added_types),
        },
        DecisionContinuationSnapshot::TriggerOrder { batch, remaining } => {
            DecisionContinuation::TriggerOrder {
                batch: parse_trigger_batch(batch, game)?,
                remaining: remaining
                    .iter()
                    .map(|batch| parse_trigger_batch(batch, game))
                    .collect::<Result<Vec<_>, _>>()?,
            }
        }
        DecisionContinuationSnapshot::TriggerPlacement {
            trigger,
            pending,
            remaining,
            candidates,
        } => DecisionContinuation::TriggerPlacement {
            trigger: parse_pending_trigger(trigger, game)?,
            pending: pending
                .iter()
                .map(|trigger| parse_pending_trigger(trigger, game))
                .collect::<Result<Vec<_>, _>>()?,
            remaining: remaining
                .iter()
                .map(|batch| parse_trigger_batch(batch, game))
                .collect::<Result<Vec<_>, _>>()?,
            candidates: candidates.iter().copied().map(parse_target).collect(),
        },
        DecisionContinuationSnapshot::MiracleReveal { card } => {
            DecisionContinuation::MiracleReveal {
                card: GameObjectId(*card),
            }
        }
        DecisionContinuationSnapshot::SeparateIntoPiles {
            resolving_controller,
            subject,
            items,
            on_complete,
        } => DecisionContinuation::SeparateIntoPiles {
            resolving_controller: player(*resolving_controller)?,
            subject: player(*subject)?,
            items: items.iter().map(parse_decision_option_snapshot).collect(),
            on_complete: crate::card::sets::piles_separated_resolver(on_complete)
                .ok_or("unknown piles-separated resolver")?,
        },
        DecisionContinuationSnapshot::ChoosePile { piles, on_complete } => {
            DecisionContinuation::ChoosePile {
                piles: parse_pile_split_snapshot(piles)?,
                on_complete: crate::card::sets::pile_chosen_resolver(on_complete)
                    .ok_or("unknown pile-chosen resolver")?,
            }
        }
        DecisionContinuationSnapshot::SacrificeOfChoice { followup, optional } => {
            DecisionContinuation::SacrificeOfChoice {
                followup: followup
                    .as_ref()
                    .map(|followup| parse_effect_continuation(followup, game))
                    .transpose()?,
                optional: *optional,
            }
        }
        DecisionContinuationSnapshot::RecallDiscard { player: owner } => {
            DecisionContinuation::RecallDiscard {
                player: player(*owner)?,
            }
        }
        DecisionContinuationSnapshot::RecallReturn { player: owner } => {
            DecisionContinuation::RecallReturn {
                player: player(*owner)?,
            }
        }
        DecisionContinuationSnapshot::Balance {
            controller,
            phase,
            task,
            remaining,
        } => DecisionContinuation::Balance {
            controller: player(*controller)?,
            phase: parse_balance_phase(*phase),
            task: parse_balance_task(task, game)?,
            remaining: remaining
                .iter()
                .map(|task| parse_balance_task(task, game))
                .collect::<Result<Vec<_>, _>>()?,
        },
        DecisionContinuationSnapshot::SylvanOffer { player: owner } => {
            DecisionContinuation::SylvanOffer {
                player: player(*owner)?,
            }
        }
        DecisionContinuationSnapshot::SylvanSelect {
            player: owner,
            candidates,
            choices_left,
        } => DecisionContinuation::SylvanSelect {
            player: player(*owner)?,
            candidates: game_ids(candidates),
            choices_left: *choices_left,
        },
        DecisionContinuationSnapshot::SylvanMode {
            player: owner,
            card,
            candidates,
            choices_left,
        } => DecisionContinuation::SylvanMode {
            player: player(*owner)?,
            card: GameObjectId(*card),
            candidates: game_ids(candidates),
            choices_left: *choices_left,
        },
        DecisionContinuationSnapshot::TetravusDetach { source } => {
            DecisionContinuation::TetravusDetach {
                source: GameObjectId(*source),
            }
        }
        DecisionContinuationSnapshot::TetravusAssemble { source } => {
            DecisionContinuation::TetravusAssemble {
                source: GameObjectId(*source),
            }
        }
    })
}
