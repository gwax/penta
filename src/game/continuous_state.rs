use crate::action::AbilityOrigin;
use crate::card::{
    AbilityDef, AbilityPredicateDef, AppliedEffectDef, AppliedRuleDef, BasicLandType, CardTypeSet,
    ColorSet, CreatureTypeSetDef, PlayRestrictionDef, SetOperationDef,
};
use crate::ids::{AbilityId, CardDefinitionId, CardPartId, GameObjectId, GrantId, PlayerId};

use super::{AbilitySourceRef, Permanent};

/// Timestamp shared by the continuous-effect slices currently modeled. Static
/// effects use their source permanent's battlefield timestamp; resolving
/// effects receive a fresh timestamp as they are created. Keeping this
/// independent from object identity lets a later layer evaluator preserve the
/// same ordering contract.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuousEffectTimestamp(pub(super) u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ContinuousEffectExpiration {
    EndOfTurn,
    UpkeepOf(PlayerId),
    TurnOf { player: PlayerId, turn: u32 },
    WhileSourceTapped,
    Never,
}

impl ContinuousEffectExpiration {
    pub(super) fn survives_turn_start(
        self,
        active_player: PlayerId,
        turns_started: [u32; 2],
    ) -> bool {
        match self {
            Self::UpkeepOf(player) => player != active_player,
            Self::TurnOf { player, turn } => turns_started[player.index()] < turn,
            Self::EndOfTurn | Self::WhileSourceTapped | Self::Never => true,
        }
    }

    pub(super) const fn survives_cleanup(self) -> bool {
        !matches!(self, Self::EndOfTurn)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub(super) enum ResolvedAbilityOperation {
    Add { ability: AbilityDef, grant: GrantId },
    Remove(AbilityPredicateDef),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ResolvedPowerToughnessOperation {
    SetBase { power: i16, toughness: i16 },
    Modify { power: i16, toughness: i16 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub(super) enum ResolvedContinuousEffectKind {
    Abilities(ResolvedAbilityOperation),
    BasicLandTypes(SetOperationDef<&'static [BasicLandType]>),
    CardTypes(SetOperationDef<CardTypeSet>),
    Colors(SetOperationDef<ColorSet>),
    CreatureTypes(SetOperationDef<CreatureTypeSetDef>),
    PowerToughness(ResolvedPowerToughnessOperation),
    Rule(AppliedRuleDef),
}

/// One resolved, noncopiable continuous-effect component attached to a
/// battlefield object. Compound authored effects share a timestamp and keep
/// their depth-first component order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ResolvedContinuousEffect {
    pub(super) definition: AppliedEffectDef,
    pub(super) source: super::AbilitySourceRef,
    pub(super) timestamp: ContinuousEffectTimestamp,
    pub(super) component_order: u16,
    pub(super) expiration: ContinuousEffectExpiration,
    pub(super) kind: ResolvedContinuousEffectKind,
}

/// One resolving play prohibition after its player recipient has been frozen.
/// Static play prohibitions stay source-derived and are not stored here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ResolvedPlayRestriction {
    pub(super) definition: AppliedEffectDef,
    pub(super) source: AbilitySourceRef,
    pub(super) affected_player: PlayerId,
    pub(super) timestamp: ContinuousEffectTimestamp,
    pub(super) component_order: u16,
    pub(super) expiration: ContinuousEffectExpiration,
    pub(super) restriction: PlayRestrictionDef,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// Ability definitions are immutable catalog values with static references.
// Keeping this operation Copy avoids allocation in the hot ability-layer walk.
#[allow(clippy::large_enum_variant)]
pub(super) enum AbilityLayerOperationKind {
    Add {
        origin: AbilityOrigin,
        ability: AbilityDef,
    },
    Remove(AbilityPredicateDef),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AbilityLayerOperation {
    pub(super) timestamp: ContinuousEffectTimestamp,
    pub(super) order: u16,
    pub(super) kind: AbilityLayerOperationKind,
}

/// An ability granted to one non-battlefield object until cleanup. The object
/// identity naturally makes the grant end if that card changes zones.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TemporaryAbilityGrant {
    pub(super) object: GameObjectId,
    pub(super) ability: AbilityDef,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct StaticAppliedEffect {
    pub(super) source: GameObjectId,
    pub(super) timestamp: ContinuousEffectTimestamp,
    pub(super) source_definition: CardDefinitionId,
    pub(super) source_part: CardPartId,
    pub(super) source_ability: AbilityId,
    pub(super) grant: Option<GrantId>,
    pub(super) component_order: u16,
    pub(super) effect: AppliedEffectDef,
}

/// One rule leaf after static and resolved continuous effects have converged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AppliedRuleEffect {
    pub(super) source: GameObjectId,
    pub(super) timestamp: ContinuousEffectTimestamp,
    pub(super) component_order: u16,
    pub(super) rule: AppliedRuleDef,
}

/// One static or resolved play prohibition after source, recipient, and
/// ordering metadata have converged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct AppliedPlayRestriction {
    pub(super) source: GameObjectId,
    pub(super) timestamp: ContinuousEffectTimestamp,
    pub(super) component_order: u16,
    pub(super) restriction: PlayRestrictionDef,
}

pub(super) struct StaticEffectTraversal<'a> {
    pub(super) source: &'a Permanent,
    pub(super) source_timestamp: ContinuousEffectTimestamp,
    pub(super) source_definition: CardDefinitionId,
    pub(super) source_part: CardPartId,
    pub(super) source_ability: AbilityId,
    pub(super) affected: &'a Permanent,
    pub(super) prospective: Option<&'a Permanent>,
    pub(super) next_grant: usize,
    pub(super) next_component_order: u16,
}
