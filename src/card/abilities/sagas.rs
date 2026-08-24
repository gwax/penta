// Saga chapters: the trigger shape that reads one, and the constructors a
// card names them with.
//
// Included textually into `abilities.rs`, so the imports here are that
// module's. What makes a chapter a chapter is this shape -- the rules read
// the number back off it to know when a Saga has been read through.

/// The event one chapter watches: a lore counter arriving that brings the
/// count to this chapter's number.
const fn saga_chapter_event(chapter: u8) -> TriggerEventDef {
    TriggerEventDef::While {
        event: &SAGA_LORE_COUNTER,
        condition: match chapter {
            1 => &SAGA_CHAPTER_ONE,
            2 => &SAGA_CHAPTER_TWO,
            3 => &SAGA_CHAPTER_THREE,
            _ => &SAGA_CHAPTER_FOUR,
        },
    }
}

static SAGA_LORE_COUNTER: TriggerEventDef = TriggerEventDef::CountersPlaced {
    object: ObjectPredicateDef::Source,
    kind: CounterKind::Lore,
};

const fn saga_chapter_condition(chapter: u8) -> TriggerConditionDef {
    TriggerConditionDef::SourceCounters {
        kind: CounterKind::Lore,
        comparison: ComparisonDef::Equal,
        amount: chapter,
    }
}

static SAGA_CHAPTER_ONE: TriggerConditionDef = saga_chapter_condition(1);
static SAGA_CHAPTER_TWO: TriggerConditionDef = saga_chapter_condition(2);
static SAGA_CHAPTER_THREE: TriggerConditionDef = saga_chapter_condition(3);
static SAGA_CHAPTER_FOUR: TriggerConditionDef = saga_chapter_condition(4);

/// One chapter of a Saga (CR 714.2c): the ability that triggers when the
/// lore counter placed makes the count reach `chapter`.
///
/// "While" rather than an intervening if: reaching the number is part of the
/// event, so a later counter -- a proliferate in response -- does not undo a
/// chapter that has already been read.
#[must_use]
pub const fn saga_chapter(chapter: u8, text: &'static str, effect: EffectDef) -> AbilityDef {
    AbilityDef::triggered(text, saga_chapter_event(chapter), effect)
}

/// The same, for a chapter that names a target.
#[must_use]
pub const fn saga_chapter_with_targets(
    chapter: u8,
    text: &'static str,
    targets: &'static [AbilityTargetDef],
    effect: EffectDef,
) -> AbilityDef {
    AbilityDef::triggered_with_targets(text, saga_chapter_event(chapter), targets, effect)
}
