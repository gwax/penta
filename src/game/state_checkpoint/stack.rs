use super::model::{
    SeatSnapshot, StackAbilitySnapshot, StackSnapshot, TargetSelectionSnapshot, TargetSnapshot,
    TriggerContextSnapshot,
};
use super::{
    AbilityId, AbilityOrigin, BasicLandType, CardDefinitionId, CardPartId, CharacteristicSource,
    DeclarativeAbilityDef, Game, GameObjectId, GameStack, GrantId, PlayerId, StackAbilityPayload,
    StackObject, StackObjectKind, Target, TargetSelection, TriggerContext, Value, ability_locator,
    array, card, catalog_ability, field, optional_id, parse_cast_signature, parse_ids, seat_value,
    str_field, u8_field, u32_field, usize_field,
};

pub(super) fn stack_ability_snapshot(
    game: &Game,
    object: &StackObject,
) -> Option<StackAbilitySnapshot> {
    let payload = object.ability.as_ref()?;
    let locator = ability_locator(&game.catalog, |candidate| {
        stack_payload_matches(payload, candidate)
    });
    Some(StackAbilitySnapshot {
        ability_locator: locator,
        target_selections: payload
            .targets
            .iter()
            .map(target_selection_snapshot)
            .collect(),
        context: trigger_context_snapshot(payload.context),
    })
}

pub(super) fn stack_object_requires_retired(game: &Game, object: &StackObject) -> bool {
    object
        .source
        .into_iter()
        .chain(
            object
                .ability
                .as_ref()
                .and_then(|payload| payload.context.object),
        )
        .chain(object.iter_targets().copied().filter_map(target_object_id))
        .chain(object.chosen_permanents.iter().copied())
        .any(|id| game.retired_objects.contains_key(&id))
}

fn target_object_id(target: Target) -> Option<GameObjectId> {
    match target {
        Target::Card(id) | Target::Permanent(id) | Target::Spell(id) => Some(id),
        Target::Player(_) => None,
    }
}

fn stack_payload_matches(
    payload: &StackAbilityPayload,
    candidate: &crate::card::AbilityDef,
) -> bool {
    if let Some(definition) = payload.definition.as_deref() {
        return definition == candidate;
    }
    let DeclarativeAbilityDef::Triggered(triggered) = candidate.definition else {
        return false;
    };
    payload.text == Some(candidate.text)
        && payload.target_defs == triggered.targets
        && payload.condition == triggered.condition
        && payload.resolver == Game::ability_resolver(payload.origin, candidate)
}

fn target_selection_snapshot(selection: &TargetSelection) -> TargetSelectionSnapshot {
    TargetSelectionSnapshot {
        slot_id: selection.slot().0,
        targets: selection
            .targets()
            .iter()
            .copied()
            .map(target_snapshot)
            .collect(),
        amounts: selection.amounts().to_vec(),
    }
}

pub(super) fn target_snapshot(target: Target) -> TargetSnapshot {
    match target {
        Target::Player(player) => TargetSnapshot::Player {
            seat: if player == PlayerId::One {
                SeatSnapshot::One
            } else {
                SeatSnapshot::Two
            },
        },
        Target::Card(id) => TargetSnapshot::Card { object_id: id.0 },
        Target::Permanent(id) => TargetSnapshot::Permanent { object_id: id.0 },
        Target::Spell(id) => TargetSnapshot::Spell { object_id: id.0 },
    }
}

fn trigger_context_snapshot(context: TriggerContext) -> TriggerContextSnapshot {
    TriggerContextSnapshot {
        object: context.object.map(|id| id.0),
        object_controller: context.object_controller.map(PlayerId::index),
        event_player: context.event_player.map(PlayerId::index),
        amount: context.amount,
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn parse_stack(
    observation: &Value,
    snapshots: &[StackSnapshot],
    game: &Game,
) -> Result<GameStack, String> {
    let visible = array(field(observation, "stack")?)?;
    if visible.len() != snapshots.len() {
        return Err("checkpoint stack does not match observation".into());
    }
    let mut stack = GameStack::default();
    for (shown, state) in visible.iter().zip(snapshots) {
        if state.has_runtime_overrides {
            return Err(
                "stack object has runtime overrides not yet represented by semantic locators"
                    .into(),
            );
        }
        if state.requires_retired_object {
            return Err(
                "stack object requires retired-object last-known information not yet represented by the checkpoint"
                    .into(),
            );
        }
        let id = GameObjectId(u32_field(shown, "objectId")?);
        if id.0 != state.object_id {
            return Err("checkpoint stack id does not match observation".into());
        }
        let definition = CardDefinitionId(
            u16::try_from(usize_field(shown, "definition")?).map_err(|_| "definition too large")?,
        );
        let owner = seat_index_value(state.owner)?;
        let controller = seat_value(field(shown, "controller")?)?;
        let kind = match str_field(shown, "kind")? {
            "Spell" => StackObjectKind::Spell,
            "ActivatedAbility" => StackObjectKind::ActivatedAbility,
            "TriggeredAbility" => StackObjectKind::TriggeredAbility,
            other => return Err(format!("unknown stack object kind {other}")),
        };
        let (source, ability, signature, card) = match kind {
            StackObjectKind::Spell => {
                let signature = parse_cast_signature(field(shown, "signature")?)?;
                let card = card(id, definition, owner, &game.catalog)?;
                (
                    None,
                    game.frozen_spell_payload(definition, &signature),
                    Some(signature),
                    card,
                )
            }
            StackObjectKind::ActivatedAbility | StackObjectKind::TriggeredAbility => {
                let payload_state = state
                    .ability_payload
                    .as_ref()
                    .ok_or("stack ability is missing its frozen payload")?;
                let origin = parse_ability_origin(field(shown, "ability")?)?;
                let source = optional_id(shown.get("sourceObjectId"));
                let definition_snapshot = payload_state
                    .ability_locator
                    .as_ref()
                    .and_then(|locator| catalog_ability(&game.catalog, locator))
                    .ok_or_else(|| {
                        "stack ability locator is absent from this catalog".to_owned()
                    })?;
                let (target_defs, condition) = match (kind, definition_snapshot.definition) {
                    (
                        StackObjectKind::ActivatedAbility,
                        DeclarativeAbilityDef::Activated(activated),
                    ) => (activated.targets, None),
                    (
                        StackObjectKind::TriggeredAbility,
                        DeclarativeAbilityDef::Triggered(triggered),
                    ) => (triggered.targets, triggered.condition),
                    _ => {
                        return Err(
                            "stack ability locator does not match the observed ability kind".into(),
                        );
                    }
                };
                let targets = payload_state
                    .target_selections
                    .iter()
                    .map(parse_target_selection)
                    .collect::<Result<Vec<_>, _>>()?;
                let context = parse_trigger_context(payload_state.context)?;
                let ability = StackAbilityPayload {
                    origin,
                    definition: (kind == StackObjectKind::ActivatedAbility)
                        .then(|| Box::new(definition_snapshot)),
                    presentation_definition: definition,
                    text: Some(definition_snapshot.text),
                    target_defs: target_defs.to_vec(),
                    targets,
                    context,
                    resolver: Game::ability_resolver(origin, &definition_snapshot),
                    condition,
                    mode_effects: Vec::new(),
                    x: u16::try_from(usize_field(shown, "x")?)
                        .map_err(|_| "ability X is too large")?,
                };
                let mut card = card(id, definition, owner, &game.catalog)?;
                card.characteristics = CharacteristicSource::Ability(definition);
                (source, Some(ability), None, card)
            }
        };
        stack.push(StackObject {
            id,
            kind,
            card,
            source,
            ability,
            controller,
            signature,
            chosen_permanents: parse_ids(field(shown, "chosenPermanents")?)?,
            applied_effects: Vec::new(),
            text_changes: Vec::new(),
            colors: None,
            cast_via_flashback: false,
            is_copy: false,
        });
    }
    Ok(stack)
}

fn parse_trigger_context(value: TriggerContextSnapshot) -> Result<TriggerContext, String> {
    Ok(TriggerContext {
        object: value.object.map(GameObjectId),
        object_controller: value.object_controller.map(seat_index_value).transpose()?,
        event_player: value.event_player.map(seat_index_value).transpose()?,
        amount: value.amount,
        chosen_objects: [None; crate::ChoiceIndex::COUNT],
    })
}

fn parse_target_selection(value: &TargetSelectionSnapshot) -> Result<TargetSelection, String> {
    let slot = crate::TargetSlotId(value.slot_id);
    let targets = value.targets.iter().copied().map(parse_target).collect();
    if value.amounts.is_empty() {
        Ok(TargetSelection::new(slot, targets))
    } else if value.amounts.len() == targets.len() {
        Ok(TargetSelection::divided(
            slot,
            targets,
            value.amounts.clone(),
        ))
    } else {
        Err("divided target amounts do not match targets".into())
    }
}

pub(super) fn parse_target(value: TargetSnapshot) -> Target {
    match value {
        TargetSnapshot::Player {
            seat: SeatSnapshot::One,
        } => Target::Player(PlayerId::One),
        TargetSnapshot::Player {
            seat: SeatSnapshot::Two,
        } => Target::Player(PlayerId::Two),
        TargetSnapshot::Card { object_id } => Target::Card(GameObjectId(object_id)),
        TargetSnapshot::Permanent { object_id } => Target::Permanent(GameObjectId(object_id)),
        TargetSnapshot::Spell { object_id } => Target::Spell(GameObjectId(object_id)),
    }
}

fn player_from_index(index: usize) -> Option<PlayerId> {
    [PlayerId::One, PlayerId::Two].get(index).copied()
}

fn seat_index_value(index: usize) -> Result<PlayerId, String> {
    player_from_index(index).ok_or_else(|| "seat index must be 0 or 1".into())
}

pub(super) fn parse_ability_origin(value: &Value) -> Result<AbilityOrigin, String> {
    match str_field(value, "kind")? {
        "printed" => Ok(AbilityOrigin::Printed {
            definition: CardDefinitionId(
                u16::try_from(usize_field(value, "definition")?)
                    .map_err(|_| "ability definition is too large")?,
            ),
            part: CardPartId(u8_field(value, "partId")?),
            ability: AbilityId(u8_field(value, "abilityId")?),
        }),
        "intrinsicBasicLand" => Ok(AbilityOrigin::IntrinsicBasicLand(
            match str_field(value, "landType")? {
                "plains" => BasicLandType::Plains,
                "island" => BasicLandType::Island,
                "swamp" => BasicLandType::Swamp,
                "mountain" => BasicLandType::Mountain,
                "forest" => BasicLandType::Forest,
                other => return Err(format!("unknown intrinsic basic land type {other}")),
            },
        )),
        "granted" => Ok(AbilityOrigin::Granted {
            source: GameObjectId(u32_field(value, "source")?),
            source_definition: CardDefinitionId(
                u16::try_from(usize_field(value, "sourceDefinition")?)
                    .map_err(|_| "grant source definition is too large")?,
            ),
            source_part: CardPartId(u8_field(value, "sourcePartId")?),
            source_ability: AbilityId(u8_field(value, "sourceAbilityId")?),
            grant: GrantId(u8_field(value, "grantId")?),
        }),
        other => Err(format!("unknown ability origin kind {other}")),
    }
}
