//! Future Sight cards cataloged as cross-format rules-engine test cases.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCoverageDef, AbilityDef, AbilityTargetDef, AppliedEffectDef, CardArt, CardRules,
    CardSet, CardType, CounterKind, CreatureStats, EffectDef, EffectRecipientDef, ManaColor,
    ObjectPredicateDef, ResolvedEffectDurationDef, SpellResolutionDestinationDef, TriggerEventDef,
    ValueDef, ZoneKind, ZonePlacement, abilities, cards,
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

pub(in crate::card::sets) static CARDS: &[&CardRecord] =
    &[&REALITY_STROBE, &DARKSTEEL_GARRISON, &DRYAD_ARBOR];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
