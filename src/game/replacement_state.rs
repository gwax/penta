use crate::card::{
    CardTypeSet, ObjectPredicateDef, PlayerRelation, ReplacementEffectDef, ZoneKind,
};
use crate::ids::{CardDefinitionId, GameObjectId, PlayerId};

use super::{AbilitySourceRef, Permanent};

/// One replacement effect that currently applies to a prospective event.
///
/// The source and ability origin form the per-event identity required by
/// rule 614.5: after this instance changes the event, it cannot apply to that
/// same event again even if re-evaluation still finds it applicable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ReplacementEffectContext {
    pub(super) source: AbilitySourceRef,
    pub(super) controller: PlayerId,
}

/// The procedures the battlefield-entry engine can order and apply. Most are
/// declarative modifications; choosing a creature type is still a dedicated
/// decision procedure, but participates in the same replacement ordering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BattlefieldEntryReplacementEffect {
    Declarative(ReplacementEffectDef),
    ChooseCreatureType,
    ChooseCardName,
    ChoosePlayer(PlayerRelation),
    CopyAsItEnters {
        object: ObjectPredicateDef,
        added_types: CardTypeSet,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ApplicableReplacement {
    pub(super) context: ReplacementEffectContext,
    pub(super) definition: CardDefinitionId,
    pub(super) text: &'static str,
    pub(super) effect: BattlefieldEntryReplacementEffect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PendingReplacementEffect {
    pub(super) context: ReplacementEffectContext,
    pub(super) effect: BattlefieldEntryReplacementEffect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum EntryCompletion {
    LandPlayed {
        player: PlayerId,
    },
    SpellResolved {
        card: GameObjectId,
        definition: CardDefinitionId,
    },
    /// The development setup surface minted this object's battlefield
    /// identity directly, so committing it must not reincarnate it again.
    Setup,
    None,
}

/// Mutable state for an object that would enter the battlefield. The object
/// deliberately remains outside every public zone until replacement effects
/// finish and `commit_battlefield_entry` gives it its destination object ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PendingBattlefieldEntry {
    pub(super) permanent: Permanent,
    pub(super) from: ZoneKind,
    pub(super) completion: EntryCompletion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ReplaceableEvent {
    BattlefieldEntry(PendingBattlefieldEntry),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PendingEvent {
    pub(super) event: ReplaceableEvent,
    pub(super) applied: Vec<AbilitySourceRef>,
    /// A LIFO program of event-local modifications. Replacement clauses can
    /// suspend this program for a choice and resume it without committing.
    pub(super) effects: Vec<PendingReplacementEffect>,
}
