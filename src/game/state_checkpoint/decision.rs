use serde_json::Value;

use crate::{CardDefinitionId, GameObjectId, PlayerId};

use super::super::{
    DecisionContinuation, DecisionKind, DecisionObservation, DecisionOption,
    DecisionOrderSemantics, DecisionPreference, DecisionVisibility, DecisionZone, PendingDecision,
};
use super::model::{
    DecisionContinuationSnapshot, DecisionPreferenceSnapshot, DecisionStateSnapshot,
};
use super::stack::{parse_target, target_snapshot};
use super::{array, bool_field, field, seat_value, str_field, u32_field, usize_field};

pub(super) fn decision_snapshot(pending: &PendingDecision) -> Option<DecisionStateSnapshot> {
    Some(DecisionStateSnapshot {
        preference: preference_snapshot(pending.observation.preference),
        continuation: continuation_snapshot(&pending.continuation)?,
    })
}

#[allow(clippy::too_many_lines)]
fn continuation_snapshot(
    continuation: &DecisionContinuation,
) -> Option<DecisionContinuationSnapshot> {
    let value = match continuation {
        DecisionContinuation::BasicLandTypeTextChange { target } => {
            DecisionContinuationSnapshot::BasicLandTypeTextChange {
                target: target_snapshot(*target),
            }
        }
        DecisionContinuation::MiracleReveal { card } => {
            DecisionContinuationSnapshot::MiracleReveal { card: card.0 }
        }
        DecisionContinuation::PileSplit { owner } => DecisionContinuationSnapshot::PileSplit {
            owner: owner.index(),
        },
        DecisionContinuation::PileChoice { first, second } => {
            DecisionContinuationSnapshot::PileChoice {
                first: ids(first),
                second: ids(second),
            }
        }
        DecisionContinuation::SacrificeOfChoice {
            followup: None,
            optional,
        } => DecisionContinuationSnapshot::SacrificeOfChoice {
            optional: *optional,
        },
        DecisionContinuation::DestroyOfChoice { can_regenerate } => {
            DecisionContinuationSnapshot::DestroyOfChoice {
                can_regenerate: *can_regenerate,
            }
        }
        DecisionContinuation::TimeVault {
            permanent,
            remaining,
        } => DecisionContinuationSnapshot::TimeVault {
            permanent: permanent.0,
            remaining: ids(remaining),
        },
        DecisionContinuation::SylvanOffer { player } => DecisionContinuationSnapshot::SylvanOffer {
            player: player.index(),
        },
        DecisionContinuation::SylvanSelect {
            player,
            candidates,
            choices_left,
        } => DecisionContinuationSnapshot::SylvanSelect {
            player: player.index(),
            candidates: ids(candidates),
            choices_left: *choices_left,
        },
        DecisionContinuation::SylvanMode {
            player,
            card,
            candidates,
            choices_left,
        } => DecisionContinuationSnapshot::SylvanMode {
            player: player.index(),
            card: card.0,
            candidates: ids(candidates),
            choices_left: *choices_left,
        },
        DecisionContinuation::TetravusDetach { source } => {
            DecisionContinuationSnapshot::TetravusDetach { source: source.0 }
        }
        DecisionContinuation::TetravusAssemble { source } => {
            DecisionContinuationSnapshot::TetravusAssemble { source: source.0 }
        }
        DecisionContinuation::DiscardForEffect { .. }
        | DecisionContinuation::Tutor
        | DecisionContinuation::LibrarySearch { .. }
        | DecisionContinuation::OptionalManaPayment { .. }
        | DecisionContinuation::ManaPaymentOrElse { .. }
        | DecisionContinuation::ChainLightning { .. }
        | DecisionContinuation::Fork { .. }
        | DecisionContinuation::OptionalEffect { .. }
        | DecisionContinuation::ChoosePermanentForEffect { .. }
        | DecisionContinuation::RevealedPileSplit { .. }
        | DecisionContinuation::RevealedPileChoice { .. }
        | DecisionContinuation::SeparateIntoPiles { .. }
        | DecisionContinuation::ChoosePile { .. }
        | DecisionContinuation::SacrificeOfChoice {
            followup: Some(_), ..
        }
        | DecisionContinuation::CounterUnlessPaid { .. }
        | DecisionContinuation::GrislySalvage { .. }
        | DecisionContinuation::RecallDiscard { .. }
        | DecisionContinuation::RecallReturn { .. }
        | DecisionContinuation::Duress { .. }
        | DecisionContinuation::Balance { .. }
        | DecisionContinuation::ExileFromHand { .. }
        | DecisionContinuation::AugurOfBolas { .. }
        | DecisionContinuation::TopCardSelection { .. }
        | DecisionContinuation::BattlefieldEntryReplacement { .. }
        | DecisionContinuation::BattlefieldEntryPayment { .. }
        | DecisionContinuation::BattlefieldEntryCardName { .. }
        | DecisionContinuation::BattlefieldEntryCopy { .. }
        | DecisionContinuation::BattlefieldEntryCreatureType { .. }
        | DecisionContinuation::TriggerOrder { .. }
        | DecisionContinuation::TriggerPlacement { .. } => return None,
    };
    Some(value)
}

pub(super) fn parse_pending_decision(
    observation: &Value,
    state: Option<&DecisionStateSnapshot>,
) -> Result<Option<PendingDecision>, String> {
    let Some(visible) = observation.get("decision").filter(|value| !value.is_null()) else {
        if state.is_some() {
            return Err("checkpoint decision is not visible to its viewer".into());
        }
        return Ok(None);
    };
    let state = state.ok_or("decision continuation lacks a semantic checkpoint encoding")?;
    Ok(Some(PendingDecision {
        observation: parse_decision_observation(visible, &state.preference)?,
        continuation: parse_continuation(&state.continuation)?,
    }))
}

fn parse_decision_observation(
    value: &Value,
    preference: &DecisionPreferenceSnapshot,
) -> Result<DecisionObservation, String> {
    Ok(DecisionObservation {
        id: u32_field(value, "id")?,
        player: seat_value(field(value, "seat")?)?,
        kind: match str_field(value, "kind")? {
            "Choice" => DecisionKind::Choice,
            "TriggerOrder" => DecisionKind::TriggerOrder,
            "TriggerPlacement" => DecisionKind::TriggerPlacement,
            other => return Err(format!("unknown decision kind {other}")),
        },
        order_semantics: value
            .get("orderSemantics")
            .filter(|value| !value.is_null())
            .map(|value| match value.as_str() {
                Some("resolution") => Ok(DecisionOrderSemantics::Resolution),
                _ => Err("unknown decision order semantics".to_owned()),
            })
            .transpose()?,
        prompt: str_field(value, "prompt")?.to_owned(),
        visibility: match str_field(value, "visibility")? {
            "Public" => DecisionVisibility::Public,
            "Private" => DecisionVisibility::Private,
            other => return Err(format!("unknown decision visibility {other}")),
        },
        preference: parse_preference(preference)?,
        minimum: usize_field(value, "minimum")?,
        maximum: usize_field(value, "maximum")?,
        cancellable: bool_field(value, "cancellable")?,
        options: array(field(value, "options")?)?
            .iter()
            .map(parse_option)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn parse_option(value: &Value) -> Result<DecisionOption, String> {
    let parse_card = |value: &Value| {
        Ok((
            GameObjectId(u32_field(value, "objectId")?),
            CardDefinitionId(
                u16::try_from(usize_field(value, "definition")?)
                    .map_err(|_| "decision card definition is too large")?,
            ),
        ))
    };
    Ok(DecisionOption {
        id: u32_field(value, "id")?,
        label: str_field(value, "label")?.to_owned(),
        card: value
            .get("card")
            .filter(|value| !value.is_null())
            .map(parse_card)
            .transpose()?,
        members: array(field(value, "members")?)?
            .iter()
            .map(parse_card)
            .collect::<Result<Vec<_>, String>>()?,
        ability_text: value
            .get("abilityText")
            .and_then(Value::as_str)
            .map(str::to_owned),
        zone: parse_decision_zone(str_field(value, "zone")?)?,
    })
}

fn parse_continuation(
    value: &DecisionContinuationSnapshot,
) -> Result<DecisionContinuation, String> {
    Ok(match value {
        DecisionContinuationSnapshot::BasicLandTypeTextChange { target } => {
            DecisionContinuation::BasicLandTypeTextChange {
                target: parse_target(*target),
            }
        }
        DecisionContinuationSnapshot::MiracleReveal { card } => {
            DecisionContinuation::MiracleReveal {
                card: GameObjectId(*card),
            }
        }
        DecisionContinuationSnapshot::PileSplit { owner } => DecisionContinuation::PileSplit {
            owner: player(*owner)?,
        },
        DecisionContinuationSnapshot::PileChoice { first, second } => {
            DecisionContinuation::PileChoice {
                first: game_ids(first),
                second: game_ids(second),
            }
        }
        DecisionContinuationSnapshot::SacrificeOfChoice { optional } => {
            DecisionContinuation::SacrificeOfChoice {
                followup: None,
                optional: *optional,
            }
        }
        DecisionContinuationSnapshot::DestroyOfChoice { can_regenerate } => {
            DecisionContinuation::DestroyOfChoice {
                can_regenerate: *can_regenerate,
            }
        }
        DecisionContinuationSnapshot::TimeVault {
            permanent,
            remaining,
        } => DecisionContinuation::TimeVault {
            permanent: GameObjectId(*permanent),
            remaining: game_ids(remaining),
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

fn preference_snapshot(preference: DecisionPreference) -> DecisionPreferenceSnapshot {
    match preference {
        DecisionPreference::HigherCardValue => {
            DecisionPreferenceSnapshot::Name("higherCardValue".into())
        }
        DecisionPreference::LowerCardValue => {
            DecisionPreferenceSnapshot::Name("lowerCardValue".into())
        }
        DecisionPreference::BalancedPartition => {
            DecisionPreferenceSnapshot::Name("balancedPartition".into())
        }
        DecisionPreference::LinkedExileTargets => {
            DecisionPreferenceSnapshot::Name("linkedExileTargets".into())
        }
        DecisionPreference::RemovalChoice => {
            DecisionPreferenceSnapshot::Name("removalChoice".into())
        }
        DecisionPreference::PreferOption(prefer_option) => {
            DecisionPreferenceSnapshot::PreferOption { prefer_option }
        }
        DecisionPreference::Neutral => DecisionPreferenceSnapshot::Name("neutral".into()),
    }
}

fn parse_preference(value: &DecisionPreferenceSnapshot) -> Result<DecisionPreference, String> {
    match value {
        DecisionPreferenceSnapshot::Name(name) => match name.as_str() {
            "higherCardValue" => Ok(DecisionPreference::HigherCardValue),
            "lowerCardValue" => Ok(DecisionPreference::LowerCardValue),
            "balancedPartition" => Ok(DecisionPreference::BalancedPartition),
            "linkedExileTargets" => Ok(DecisionPreference::LinkedExileTargets),
            "removalChoice" => Ok(DecisionPreference::RemovalChoice),
            "neutral" => Ok(DecisionPreference::Neutral),
            other => Err(format!("unknown decision preference {other}")),
        },
        DecisionPreferenceSnapshot::PreferOption { prefer_option } => {
            Ok(DecisionPreference::PreferOption(*prefer_option))
        }
    }
}

fn parse_decision_zone(value: &str) -> Result<DecisionZone, String> {
    match value {
        "Hand" => Ok(DecisionZone::Hand),
        "Graveyard" => Ok(DecisionZone::Graveyard),
        "Battlefield" => Ok(DecisionZone::Battlefield),
        "Stack" => Ok(DecisionZone::Stack),
        "Library" => Ok(DecisionZone::Library),
        "Exile" => Ok(DecisionZone::Exile),
        "Command" => Ok(DecisionZone::Command),
        "DrawnThisStep" => Ok(DecisionZone::DrawnThisStep),
        "None" => Ok(DecisionZone::None),
        other => Err(format!("unknown decision zone {other}")),
    }
}

fn ids(ids: &[GameObjectId]) -> Vec<u32> {
    ids.iter().map(|id| id.0).collect()
}

fn game_ids(ids: &[u32]) -> Vec<GameObjectId> {
    ids.iter().copied().map(GameObjectId).collect()
}

fn player(index: usize) -> Result<PlayerId, String> {
    match index {
        0 => Ok(PlayerId::One),
        1 => Ok(PlayerId::Two),
        _ => Err("seat index must be 0 or 1".into()),
    }
}
