// The intervening-if conditions a trigger or a resolving effect can read.
//
// Separated from the ability shapes next door because this answers a different
// question: those say how a clause executes, while these are the facts a
// clause can ask about before it does. Included textually into
// `ability_kinds.rs`, so the paths and imports here are the parent module's.

/// An intervening-if condition, the "if ..." clause a trigger reads before it
/// does anything. Rule 603.4 checks such a condition twice: once when the
/// ability would go on the stack, and again as it resolves.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TriggerConditionDef {
    /// Every listed condition holds. A card whose clause names two facts at
    /// once -- "if an opponent controls an Island and you control a Mountain"
    /// -- is one condition made of two rather than two clauses.
    All(&'static [TriggerConditionDef]),
    /// The condition does not hold. Two clauses of one printed sentence --
    /// "if you do" and "if you don't" -- are written as complementary
    /// conditions rather than as an effect with two branches, so that the
    /// pair reads the same way the card does.
    Not(&'static TriggerConditionDef),
    /// Whether the original source object is still on the battlefield.
    SourceOnBattlefield,
    /// Whether two bound objects share a card name. Naming a card and then
    /// revealing one is a comparison of names rather than of identity: a
    /// second copy of the named card is still the named card.
    BoundObjectsShareName {
        first: ObjectBindingIndex,
        second: ObjectBindingIndex,
    },
    /// Whether the source came under its controller's control since the
    /// beginning of that player's previous upkeep -- the condition echo is
    /// written against, and what makes an echo cost come due exactly once.
    SourceArrivedSinceControllersLastUpkeep,
    /// Whether the source permanent is currently untapped.
    SourceUntapped,
    /// Whether the source is soulbonded to another creature. The clause every
    /// soulbond card prints reads "as long as this creature is paired with
    /// another creature", so it is continuous rather than checked once.
    SourceIsPaired,
    /// How many objects the query matches, against a printed number.
    ObjectCount {
        query: ObjectQueryDef,
        comparison: ComparisonDef,
        amount: u8,
    },
    /// Whose turn it is, relative to the ability's controller.
    ActivePlayer(PlayerRelation),
    /// How many spells a matching player cast during the turn before this
    /// one. "No spells were cast last turn" is every player at zero, and "a
    /// player cast two or more" is any player at two.
    SpellsCastLastTurn {
        /// Whether every matching player has to satisfy the comparison or
        /// only one. "No spells were cast last turn" is every player at zero;
        /// "a player cast two or more" is one player at two.
        quantifier: QuantifierDef,
        player: PlayerRelation,
        comparison: ComparisonDef,
        amount: u8,
    },
    /// The same, counted for the turn in progress. Read after the spell that
    /// caused the trigger has already been counted, so "your second spell
    /// each turn" is a comparison against two rather than one.
    SpellsCastThisTurn {
        quantifier: QuantifierDef,
        player: PlayerRelation,
        comparison: ComparisonDef,
        amount: u8,
    },
    /// How the source's own spell was cast. Evoke's sacrifice asks exactly
    /// this: the permanent is here, and the question is which way it was
    /// paid for on the way in. False for anything that never was a spell.
    SourceCastWith(AlternativeCastKindDef),
    /// "If you cast it any time a sorcery couldn't have been cast." Recorded
    /// as the spell was cast, because nothing afterwards can tell.
    SourceCastAtInstantSpeed,
    /// How much loyalty the ability's own source has left.
    SourceLoyalty {
        comparison: ComparisonDef,
        amount: u8,
    },
    /// How many times this ability has been activated from its source this
    /// turn, counting the activation now resolving.
    SourceActivationsThisTurn {
        comparison: ComparisonDef,
        amount: u8,
    },
    /// Whether this ability's own source has dealt damage to an opponent of
    /// its controller at any point this turn, by any means.
    SourceDealtDamageToOpponentThisTurn,
    /// Whether the ability's own source is tapped, using last-known
    /// information if it has left the battlefield.
    SourceIsTapped,
    /// Whether the ability's own source is untapped, using last-known
    /// information if it has left the battlefield. Not the negation of
    /// [`Self::SourceIsTapped`] for an object that was never on the
    /// battlefield, which is neither.
    SourceIsUntapped,
    /// Whether the ability's controller is at or below this life total, for
    /// the fateful-hour clauses. Read live, so a static ability guarded by it
    /// switches on and off as life moves rather than being fixed when the
    /// permanent arrived.
    ControllerLifeAtMost(u16),
    /// Whether the ability's controller is at or below half their starting
    /// life total. Printed that way rather than as a number, so it is read
    /// from the format rather than fixed at ten -- the fateful-hour cards
    /// print the literal instead and use [`Self::ControllerLifeAtMost`].
    ControllerLifeAtMostHalfStartingLife,
    /// Whether this ability's controller controls a creature whose power is
    /// at least every other creature's, which is what "the greatest power or
    /// tied for the greatest power" asks. False when no creature is on the
    /// battlefield at all.
    ControlsGreatestPowerCreature,
    /// Whether a creature has gone to a graveyard this turn. The condition
    /// form of the morbid value, for the intervening-ifs that ask rather than
    /// pick an amount.
    CreatureDiedThisTurn,
    /// Whether the ability's own source matches. The mirror of
    /// [`Self::AttachedPermanentMatches`] pointed at the source itself, for
    /// the intervening-ifs that ask what the permanent has been doing.
    SourceMatches { object: ObjectPredicateDef },
    /// Whether what the ability's source is attached to matches. This is what
    /// "as long as equipped creature is a Human" asks, and it is read live so
    /// the answer follows the Equipment as it moves.
    AttachedPermanentMatches { object: ObjectPredicateDef },
    /// How many counters of one kind the ability's own source carries. This
    /// is what "as long as there are exactly three tide counters on this
    /// creature" asks, and it is read live rather than captured.
    SourceCounters {
        kind: CounterKind,
        comparison: ComparisonDef,
        amount: u8,
    },
    /// Whether what a target slot points at still matches. Read when the
    /// condition is checked, so a delayed effect can ask about the target as
    /// it is then rather than as it was.
    TargetMatches {
        slot: TargetIndex,
        object: ObjectPredicateDef,
    },
}
