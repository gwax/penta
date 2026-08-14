use crate::CardCatalog;
use crate::card::{AppliedEffectDef, AppliedRuleDef};

use super::model::ResolvedPlayRestrictionSnapshot;
use super::{
    AbilitySourceRef, ContinuousEffectTimestamp, GameObjectId, ResolvedPlayRestriction,
    ability_origin_from_snapshot, event, expiration_snapshot, parse_expiration,
    resolved_applied_effect_locator, semantics, wire,
};

pub(super) fn resolved_play_restriction_snapshot(
    catalog: &CardCatalog,
    restriction: ResolvedPlayRestriction,
) -> Option<ResolvedPlayRestrictionSnapshot> {
    let AppliedEffectDef::Rule(AppliedRuleDef::CannotPlay(authored)) = restriction.definition
    else {
        return None;
    };
    if authored != restriction.restriction {
        return None;
    }
    Some(ResolvedPlayRestrictionSnapshot {
        definition: resolved_applied_effect_locator(
            catalog,
            restriction.source,
            restriction.definition,
        )?,
        source: event::ability_source_snapshot(restriction.source),
        affected_seat: restriction.affected_player.index(),
        timestamp: restriction.timestamp.0,
        component_order: restriction.component_order,
        expiration: expiration_snapshot(restriction.expiration),
    })
}

pub(super) fn parse_resolved_play_restriction(
    catalog: &CardCatalog,
    snapshot: &ResolvedPlayRestrictionSnapshot,
) -> Result<ResolvedPlayRestriction, String> {
    let source = AbilitySourceRef {
        object: GameObjectId(snapshot.source.object),
        ability: ability_origin_from_snapshot(snapshot.source.ability),
    };
    if !semantics::applied_effect_locator_matches_source(&snapshot.definition, source) {
        return Err(
            "checkpoint play-restriction locator disagrees with its source ability".to_owned(),
        );
    }
    let definition = semantics::catalog_applied_effect(catalog, &snapshot.definition)
        .ok_or("checkpoint play-restriction locator is absent from this catalog")?;
    let AppliedEffectDef::Rule(AppliedRuleDef::CannotPlay(restriction)) = definition else {
        return Err("checkpoint play-restriction locator does not name CannotPlay".to_owned());
    };
    Ok(ResolvedPlayRestriction {
        definition,
        source,
        affected_player: wire::player_from_index(snapshot.affected_seat)?,
        timestamp: ContinuousEffectTimestamp(snapshot.timestamp),
        component_order: snapshot.component_order,
        expiration: parse_expiration(snapshot.expiration)?,
        restriction,
    })
}
