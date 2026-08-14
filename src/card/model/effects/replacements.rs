//! The declarative vocabulary for replacement abilities.
//!
//! A replacement ability is described by the event it watches, the condition
//! that gates it, and the operations it performs instead. These live apart
//! from the ordinary effect vocabulary because nothing outside a replacement
//! ability reads them.

use super::{
    ConditionDef, CounterKind, EffectDef, EffectPaymentDef, ObjectPredicateDef, PlayerRelation,
    TurnKindDef, ZoneKind,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReplacementEventDef {
    /// The object carrying this ability would enter the battlefield.
    SourceEntersBattlefield,
    /// A matching object would enter the battlefield.
    ObjectEntersBattlefield {
        object: ObjectPredicateDef,
        controller: PlayerRelation,
    },
    /// This ability's source would move between the named zones for the
    /// specified reason. Matching happens before the object leaves `from`.
    WouldMove {
        from: ZoneKind,
        to: ZoneKind,
        cause: ZoneMoveCauseDef,
    },
    /// A player would gain life, matched relative to the replacement
    /// ability's controller.
    WouldGainLife(PlayerRelation),
    /// A matching player would begin a turn. The turn is still prospective:
    /// none of its turn-based actions, counters, or beginning-of-turn events
    /// have happened yet.
    WouldBeginTurn {
        player: PlayerRelation,
        kind: TurnKindDef,
    },
    /// Any object anywhere would be put into this zone. Unlike
    /// [`Self::WouldMove`] this does not describe the moving object's own
    /// ability: the replacement source watches from the battlefield.
    AnyObjectWouldMove { to: ZoneKind },
    /// A narrow, named event that is not yet part of the shared vocabulary.
    Special(&'static str),
}

/// What is causing a proposed zone move. A controlled effect is matched
/// relative to the replacement ability's controller; rules and costs do not
/// have an effect controller and therefore only match [`Self::Any`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ZoneMoveCauseDef {
    Any,
    EffectControlledBy(PlayerRelation),
}

/// A condition checked while deciding whether a replacement ability applies
/// to its prospective event.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReplacementConditionDef {
    /// The permanent carrying the replacement ability is currently tapped.
    SourceTapped,
    /// A creature died at some point this turn, which is what morbid asks.
    /// Read as the replacement applies, so a creature dying in response
    /// changes the answer.
    CreatureDiedThisTurn,
}

/// A typed modification to the permanent an object would become as it enters
/// the battlefield.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BattlefieldEntryModificationDef {
    Tapped,
    AddCounters { kind: CounterKind, amount: u16 },
}

/// A choice made while applying a source-entry replacement.
///
/// The entering object is implicit: replacement programs operate on the
/// prospective event rather than naming an already-existing game object.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReplacementChoiceDef {
    CardName,
    CreatureType,
    Player(PlayerRelation),
}

/// Declarative operations performed by a replacement ability.
///
/// Branches are slices so complex replacements remain const-friendly and can
/// be resumed around a player choice without baking card names into the game
/// engine.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReplacementEffectDef {
    Sequence(&'static [ReplacementEffectDef]),
    /// Consume the prospective event without committing it.
    ReplaceEventWithNothing,
    /// Change the destination of a prospective zone move. The source object
    /// has not left its current zone while this operation is interpreted.
    MoveToZone(ZoneKind),
    /// Perform an ordinary declarative effect as part of replacing the event.
    /// The replacement source and controller provide the effect context.
    Perform(&'static EffectDef),
    ModifyBattlefieldEntry(BattlefieldEntryModificationDef),
    /// Multiply the amount carried by the prospective event.
    MultiplyEventAmount(u8),
    /// Record a choice on the object that is entering.
    Choose(ReplacementChoiceDef),
    /// Optionally use another permanent's copiable values for the entering
    /// object, retaining the named card types in addition to the copy.
    CopyEntering {
        object: ObjectPredicateDef,
        added_types: super::CardTypeSet,
    },
    Conditional {
        condition: ConditionDef,
        if_true: &'static [ReplacementEffectDef],
        if_false: &'static [ReplacementEffectDef],
    },
    PayOr {
        payment: EffectPaymentDef,
        if_paid: &'static [ReplacementEffectDef],
        if_declined: &'static [ReplacementEffectDef],
    },
}
