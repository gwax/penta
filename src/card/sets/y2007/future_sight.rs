//! Future Sight cards cataloged as cross-format rules-engine test cases.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityCoverageDef, AbilityDef, AbilityTargetDef, AddManaEffectDef,
    AppliedEffectDef, CardArt, CardRules, CardSet, CardType, CounterKind, CreatureStats, EffectDef,
    EffectRecipientDef, ManaColor, ObjectPredicateDef, PlayerRelation, ResolvedEffectDurationDef,
    SpellResolutionDestinationDef, TriggerEventDef, TurnStepDef, ValueDef, ZoneKind, ZonePlacement,
    abilities, cards,
};
use crate::{TargetIndex, mana_cost};

static REALITY_STROBE_TIME_COUNTERS: [(CounterKind, u16); 1] = [(CounterKind::Time, 3)];

// FUT 43 — Reality Strobe
// Audit: partial — Its spell effect and self-exile with time counters are executable, but suspend's upkeep counter removal and free cast from exile need the shared exile-casting lifecycle.
pub(in crate::card::sets) static REALITY_STROBE: CardRecord = CardRecord::new(
    cards::REALITY_STROBE,
    "Reality Strobe",
    CardArt::new("8e6d881a-f7b1-471f-bc0b-64a79bb491c9", "Dan Murayama Scott"),
    CardSet::FutureSight,
    CardRules::new_sorcery(mana_cost!("{4}{U}{U}")).with_ability(
        AbilityDef::spell_with_targets(
            "Return target permanent to its owner's hand. Exile Reality Strobe with three time counters on it.\n\nSuspend 3—{2}{U} (Rather than cast this card from your hand, you may pay {2}{U} and exile it with three time counters on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost.)",
            &[AbilityTargetDef::exactly_one_permanent(ObjectPredicateDef::Any)],
            EffectDef::MoveToZone {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                zone: ZoneKind::Hand,
                placement: ZonePlacement::Top,
                arrival_effect: None,
                attach_source: false,
                controller: None,
            },
        )
        .with_resolution_destination(SpellResolutionDestinationDef::ExileWithCounters(
            &REALITY_STROBE_TIME_COUNTERS,
        ))
        .with_coverage(AbilityCoverageDef::partial(
            "Suspend's upkeep counter removal and free cast from exile need the shared exile-casting lifecycle.",
        )),
    ),
);

/// The printed clause removes the counters and then adds the mana, but the
/// amount is read off the counters, so the two steps are written the other
/// way round. One resolution, no priority in between, and nothing else in
/// the pool watches a charge counter leave: what is observable is that the
/// counters are gone and that many mana arrived.
static RELIC_CASHES_IN: [EffectDef; 2] = [
    EffectDef::AddMana(
        AddManaEffectDef::any_color()
            .with_variable_amount(ValueDef::CountersOnSource(CounterKind::Charge)),
    ),
    EffectDef::RemoveAllCounters {
        object: EffectRecipientDef::Source,
        kind: CounterKind::Charge,
    },
];

// FUT 161 — Coalition Relic
pub(in crate::card::sets) static COALITION_RELIC: CardRecord = CardRecord::new(
    cards::COALITION_RELIC,
    "Coalition Relic",
    CardArt::new("7a7c98b0-d64d-4d0a-b284-1187a8e7095e", "Donato Giancola"),
    CardSet::FutureSight,
    // Three mana that fixes on the turn it lands and ramps on every one
    // after, provided nothing needs the Relic tapped for mana that turn.
    CardRules::new_artifact(mana_cost!("{3}")).with_abilities(&[
        AbilityDef::activated_mana(
            "{T}: Add one mana of any color.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddMana(AddManaEffectDef::any_color()),
        ),
        AbilityDef::activated(
            "{T}: Put a charge counter on this artifact.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddCounters {
                object: EffectRecipientDef::Source,
                kind: CounterKind::Charge,
                amount: ValueDef::Constant(1),
            },
        ),
        AbilityDef::triggered(
            "At the beginning of your first main phase, remove all charge counters from this artifact. Add one mana of any color for each charge counter removed this way.",
            TriggerEventDef::StepBegins {
                step: TurnStepDef::PrecombatMain,
                player: PlayerRelation::You,
            },
            EffectDef::Sequence(&RELIC_CASHES_IN),
        ),
    ]),
);

// FUT 167 — Darksteel Garrison
pub(in crate::card::sets) static DARKSTEEL_GARRISON: CardRecord = CardRecord::new(
    cards::DARKSTEEL_GARRISON,
    "Darksteel Garrison",
    CardArt::new("e77eaaa0-40f9-40e4-b0ba-5a8addd764d3", "David Martin"),
    CardSet::FutureSight,
    CardRules::new_artifact(mana_cost!("{2}"))
        .with_subtypes(&["Fortification"])
        .with_abilities(&[
            AbilityDef::static_ability(
                "Fortified land has indestructible.",
                EffectDef::StaticApply {
                    recipient: EffectRecipientDef::AttachedPermanent,
                    effect: AppliedEffectDef::add_ability(&abilities::indestructible()),
                },
            ),
            AbilityDef::triggered_with_targets(
                "Whenever fortified land becomes tapped, target creature gets +1/+1 until end of turn.",
                TriggerEventDef::tapped(ObjectPredicateDef::AttachedToSource),
                &[AbilityTargetDef::exactly_one_permanent(
                    ObjectPredicateDef::HasType(CardType::Creature),
                )],
                EffectDef::Apply {
                    recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                    effect: AppliedEffectDef::modify_power_toughness(
                        ValueDef::Constant(1),
                        ValueDef::Constant(1),
                    ),
                    duration: ResolvedEffectDurationDef::UntilEndOfTurn,
                },
            ),
            abilities::fortify(
                mana_cost!("{3}"),
                "Fortify {3} ({3}: Attach to target land you control. Fortify only as a sorcery. This card enters unattached and stays on the battlefield if the land leaves.)",
            ),
        ]),
);

// FUT 174 — Dryad Arbor
pub(in crate::card::sets) static DRYAD_ARBOR: CardRecord = CardRecord::new(
    cards::DRYAD_ARBOR,
    "Dryad Arbor",
    CardArt::new("8cee476d-42e1-4997-87af-73e18f542167", "Eric Fortune"),
    CardSet::FutureSight,
    CardRules::new_land(&[])
        .with_type(CardType::Creature)
        .with_subtypes(&["Forest", "Dryad"])
        .with_creature_stats(CreatureStats {
            power: 1,
            toughness: 1,
        })
        .printed_colors(&[ManaColor::Green]),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &REALITY_STROBE,
    &COALITION_RELIC,
    &DARKSTEEL_GARRISON,
    &DRYAD_ARBOR,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
