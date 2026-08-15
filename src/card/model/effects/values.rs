use crate::ids::TargetIndex;

use super::super::{CounterKind, ObjectPredicateDef, PlayerRelation, ZoneKind};
use super::{DamageSourceGroupDef, PlayerSetDef};

/// The two branches of a conditional value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ConditionalValueDef {
    pub then: ValueDef,
    pub otherwise: ValueDef,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LifeConditionDef {
    pub threshold: u16,
    pub then: ValueDef,
    pub otherwise: ValueDef,
}

impl LifeConditionDef {
    #[must_use]
    pub const fn new(threshold: u16, then: ValueDef, otherwise: ValueDef) -> Self {
        Self {
            threshold,
            then,
            otherwise,
        }
    }
}

/// A conditional value that asks how many objects match.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CountConditionDef {
    pub query: ObjectQueryDef,
    pub equals: u8,
    pub then: ValueDef,
    pub otherwise: ValueDef,
}

/// A conditional value that asks what the chosen target is.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TargetConditionDef {
    pub slot: TargetIndex,
    pub object: ObjectPredicateDef,
    pub then: ValueDef,
    pub otherwise: ValueDef,
}

/// A set of objects with independent controller and owner constraints.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ObjectQueryDef {
    pub object: ObjectPredicateDef,
    pub zones: &'static [ZoneKind],
    /// A zone-relative constraint: controller for battlefield and stack
    /// objects, owner for cards in every other zone. This preserves the
    /// ordinary "you control" / "in your graveyard" query vocabulary even
    /// when one query spans both kinds of zone.
    pub related_player: Option<PlayerSetDef>,
    pub controller: Option<PlayerSetDef>,
    pub owner: Option<PlayerSetDef>,
}

impl ObjectQueryDef {
    #[must_use]
    pub const fn new(object: ObjectPredicateDef, zones: &'static [ZoneKind]) -> Self {
        Self {
            object,
            zones,
            related_player: None,
            controller: None,
            owner: None,
        }
    }

    #[must_use]
    pub const fn controlled_by(
        object: ObjectPredicateDef,
        zones: &'static [ZoneKind],
        controller: PlayerSetDef,
    ) -> Self {
        Self {
            object,
            zones,
            related_player: None,
            controller: Some(controller),
            owner: None,
        }
    }

    #[must_use]
    pub const fn owned_by(
        object: ObjectPredicateDef,
        zones: &'static [ZoneKind],
        owner: PlayerSetDef,
    ) -> Self {
        Self {
            object,
            zones,
            related_player: None,
            controller: None,
            owner: Some(owner),
        }
    }

    /// Compatibility constructor for the old zone-relative query spelling:
    /// battlefield/stack objects are related by controller, while cards in
    /// other zones are related by owner.
    #[must_use]
    pub const fn matching(
        object: ObjectPredicateDef,
        zones: &'static [ZoneKind],
        controller_or_owner: PlayerRelation,
    ) -> Self {
        Self {
            object,
            zones,
            related_player: Some(PlayerSetDef::Related(controller_or_owner)),
            controller: None,
            owner: None,
        }
    }
}

/// A value evaluated from the resolving spell or ability and its captured
/// event. `SourcePower` and `SourceToughness` deliberately leave current-versus
/// last-known-information selection to the runtime source reference.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ValueDef {
    Constant(i32),
    ChosenX,
    /// The X chosen for the spell that put the ability's source onto the
    /// battlefield. An enters trigger is a new object, so [`Self::ChosenX`]
    /// reads nothing there; this reads it off the permanent instead.
    SourceCastX,
    SourcePower,
    SourceToughness,
    TriggerEventAmount,
    CardsInHandAbove {
        player: PlayerRelation,
        threshold: u8,
    },
    /// How much damage a player has been dealt so far this turn, optionally
    /// only from one named source group. Accumulated as the damage lands,
    /// because a group such as "unblocked creatures" stops being answerable
    /// once combat is over.
    DamageTakenThisTurn {
        player: PlayerRelation,
        source: Option<DamageSourceGroupDef>,
    },
    /// How many objects match, for the "for each" clauses. Held by reference
    /// so that `ValueDef` stays small enough to embed freely.
    CountMatchingObjects(&'static ObjectQueryDef),
    /// One when at least one object matches, zero otherwise. "As long as you
    /// control a Mountain" is a condition rather than a count, so counting
    /// matches would pay a second Mountain twice.
    AnyMatchingObject(&'static ObjectQueryDef),
    /// The negation of another value, so a "for each" penalty can reuse the
    /// same count a bonus would.
    Negate(&'static ValueDef),
    /// Another value multiplied by a constant, for the clauses that pay more
    /// than one per thing counted. Held by reference for the same reason
    /// [`Self::Negate`] is: `ValueDef` stays one word wide.
    Scaled(&'static ScaledValueDef),
    /// Two values added together, for "1 plus the power of ...". Held by
    /// reference like the other compound forms so that `ValueDef` stays one
    /// word wide.
    Sum(&'static SumValueDef),
    /// Half of another value, rounded the way the card says. Rounding is only
    /// visible when a value is divided, so the direction belongs to the
    /// division rather than being a separate step over it.
    Halved(&'static HalvedValueDef),
    /// How many counters of one kind sit on the ability's own source.
    CountersOnSource(CounterKind),
    /// How many creatures have died this turn, for "for each creature that
    /// died this turn". Counted as they die rather than read off a zone,
    /// because a graveyard is not a record of this turn.
    CreaturesDiedThisTurn,
    /// The morbid condition. Held by reference so that `ValueDef` stays one
    /// word wide; a second inline value would grow everything embedding it.
    IfCreatureDiedThisTurn(&'static ConditionalValueDef),
    /// One value while the ability's controller is at or below this life
    /// total, another otherwise. The fateful-hour "instead" clauses, which
    /// replace an amount rather than adding a second effect beside it.
    IfControllerLifeAtMost(&'static LifeConditionDef),
    /// One value when the chosen target matches, another when it does not.
    /// Held by reference for the same reason.
    IfTargetMatches(&'static TargetConditionDef),
    /// One value when exactly that many objects match, another otherwise.
    /// This is how an intervening-if condition becomes an amount.
    IfMatchingObjectCount(&'static CountConditionDef),
    /// How much of a divided total the target being affected takes. Only
    /// meaningful for an effect aimed at a slot the card divides.
    DividedAmongTargets,
    /// The power of what a target slot points at, for "damage equal to its
    /// power".
    /// The triggering object's power, read with last-known information. A
    /// death trigger asks this about a creature that has already left, which
    /// is the only time it is interesting.
    TriggeringObjectPower,
    /// The triggering object's toughness, read the same way and for the same
    /// reason: a death trigger asks about a creature that has already left.
    TriggeringObjectToughness,
    TargetPower(TargetIndex),
    /// The mana value of what a target slot points at, read from last-known
    /// information after a permanent or spell has left its zone.
    TargetManaValue(TargetIndex),
}

/// A value and the constant it is multiplied by, for "+N/+N for each ...".
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ScaledValueDef {
    pub value: ValueDef,
    pub factor: i32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SumValueDef {
    pub left: ValueDef,
    pub right: ValueDef,
}

impl SumValueDef {
    #[must_use]
    pub const fn new(left: ValueDef, right: ValueDef) -> Self {
        Self { left, right }
    }
}

/// Which way a halved value rounds. A card that halves says so explicitly,
/// and the two halves of "half rounded down and half rounded up" are what
/// make a single count into two different numbers.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RoundingDef {
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct HalvedValueDef {
    pub value: ValueDef,
    pub rounding: RoundingDef,
}

impl HalvedValueDef {
    #[must_use]
    pub const fn new(value: ValueDef, rounding: RoundingDef) -> Self {
        Self { value, rounding }
    }

    /// Halves `total` the way this definition says. Rounding is applied
    /// towards the named direction for negative totals too, so a negative
    /// count does not quietly change which way it goes.
    #[must_use]
    pub const fn apply(&self, total: i32) -> i32 {
        match self.rounding {
            RoundingDef::Down => total.div_euclid(2),
            RoundingDef::Up => total.div_euclid(2) + total.rem_euclid(2),
        }
    }
}

impl ScaledValueDef {
    #[must_use]
    pub const fn new(value: ValueDef, factor: i32) -> Self {
        Self { value, factor }
    }
}
