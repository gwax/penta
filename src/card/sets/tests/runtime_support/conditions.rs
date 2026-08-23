//! Which conditions the shared runtime can evaluate.
//!
//! Split by where the question is asked rather than by what it asks: a
//! trigger's intervening-if is read once as the trigger is placed, and a
//! static clause's is re-read on every layer walk -- so the second list is
//! the smaller one, and everything on it has to be state the walk already
//! holds.

use super::*;

pub(in super::super) fn shared_trigger_condition(condition: TriggerConditionDef) -> bool {
    match condition {
        TriggerConditionDef::All(conditions) | TriggerConditionDef::AnyOf(conditions) => {
            conditions.iter().copied().all(shared_trigger_condition)
        }
        TriggerConditionDef::Not(condition) => shared_trigger_condition(*condition),
        TriggerConditionDef::ObjectCount { query, .. } => shared_object_predicate(query.object),
        TriggerConditionDef::TargetMatches { object, .. }
        | TriggerConditionDef::BoundObjectMatches { object, .. }
        | TriggerConditionDef::SourceMatches { object }
        | TriggerConditionDef::LinkedExilesMatch { object }
        | TriggerConditionDef::AttachedPermanentMatches { object } => {
            shared_object_predicate(object)
        }
        TriggerConditionDef::ControllerHadPermanentLeaveThisTurn
        | TriggerConditionDef::ControllerHadCardLeaveGraveyardThisTurn
        | TriggerConditionDef::ControllerHasCitysBlessing
        | TriggerConditionDef::ControllerGainedLifeThisTurn
        | TriggerConditionDef::CreatureDiedThisTurn
        | TriggerConditionDef::BoundObjectsShareName { .. }
        | TriggerConditionDef::SourceArrivedSinceControllersLastUpkeep
        | TriggerConditionDef::SourceOnBattlefield
        | TriggerConditionDef::SourceUntapped
        | TriggerConditionDef::SourceIsPaired
        | TriggerConditionDef::ActivePlayer(_)
        | TriggerConditionDef::SourceCastWith(_)
        | TriggerConditionDef::SourceCastFrom(_)
        | TriggerConditionDef::SourceCastAtInstantSpeed
        | TriggerConditionDef::ValueComparison(_)
        | TriggerConditionDef::SourceLoyalty { .. }
        | TriggerConditionDef::SourceCounters { .. }
        | TriggerConditionDef::ControlsGreatestPowerCreature
        | TriggerConditionDef::SourceActivationsThisTurn { .. }
        | TriggerConditionDef::SourceResolutionsThisTurn { .. }
        | TriggerConditionDef::SourceDealtDamageToOpponentThisTurn
        | TriggerConditionDef::SourceIsTapped
        | TriggerConditionDef::SourceIsUntapped
        | TriggerConditionDef::ControllerLifeAtMost(_)
        | TriggerConditionDef::ControllerLifeAtMostHalfStartingLife
        | TriggerConditionDef::SpellsCastThisTurn { .. }
        | TriggerConditionDef::SpellsCastLastTurn { .. } => true,
    }
}

/// Static effects have a battlefield source but no captured trigger event,
/// resolving ability, or stack-target scope. Keep their condition boundary to
/// the source-state predicates that can be evaluated from exactly that input.
pub(in super::super) fn shared_static_trigger_condition(condition: TriggerConditionDef) -> bool {
    // Read live off the battlefield, exactly like the attached-permanent form
    // below, so a static clause tracks the source as it changes.
    if let TriggerConditionDef::SourceMatches { object } = condition {
        return shared_object_predicate(object);
    }
    // A battlefield count is re-read on every walk, so it tracks the board the
    // way "as long as" asks. The predicate still has to be one that does not
    // read back into the layer being computed.
    if let TriggerConditionDef::ObjectCount { query, .. } = condition {
        return shared_object_predicate(query.object);
    }
    matches!(
        condition,
        // Counters live on the source, so a static clause can read them from
        // exactly the input it has.
        TriggerConditionDef::CreatureDiedThisTurn
        | TriggerConditionDef::BoundObjectsShareName { .. }
        | TriggerConditionDef::SourceArrivedSinceControllersLastUpkeep
        | TriggerConditionDef::SourceOnBattlefield
            | TriggerConditionDef::SourceUntapped
        | TriggerConditionDef::SourceIsPaired
            | TriggerConditionDef::SourceCounters { .. }
            // Reachable from the source by following its attachment, which
            // is exactly the input a static clause has.
            | TriggerConditionDef::AttachedPermanentMatches { .. }
            // The controller's life is read from the same input, and a
            // fateful-hour clause switches off again when life goes back up.
            | TriggerConditionDef::ControllerLifeAtMost(_)
            | TriggerConditionDef::ControllerLifeAtMostHalfStartingLife
            // Whose turn it is comes off the game rather than out of the
            // layer being computed, so a static clause may gate on it:
            // "during your turn" is a condition, not a recipient.
            | TriggerConditionDef::ActivePlayer(_)
            // How the permanent was cast is recorded on it as the spell
            // resolved, so a static clause reads plain state rather than
            // anything the layer being computed could change. Dash's haste
            // is the clause that asks.
            | TriggerConditionDef::SourceCastWith(_)
    )
}
