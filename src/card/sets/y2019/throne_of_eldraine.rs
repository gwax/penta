//! Throne of Eldraine cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, ActivationTimingDef, BattlefieldEntryModificationDef, CardArt,
    CardRules, CardSet, ControlDurationDef, CounterKind, EffectDef, EffectRecipientDef,
    ObjectPredicateDef, PlayerRefDef, ReplacementEffectDef, ValueDef, ZoneKind, ZonePlacement,
    cards,
};
use crate::mana_cost;

static WISHCLAW_COSTS: [AbilityCostDef; 3] = [
    AbilityCostDef::Mana(mana_cost!("{1}")),
    AbilityCostDef::TapSource,
    AbilityCostDef::RemoveCountersFromSource {
        kind: CounterKind::Wish,
        amount: 1,
    },
];

/// The tutor and the handover are one clause resolving in order, so the card
/// is in hand before the artifact changes sides -- and the opponent inherits
/// two counters they may spend on their own turn.
static WISHCLAW_GRANTS_A_WISH: [EffectDef; 2] = [
    EffectDef::SearchZone {
        player: EffectRecipientDef::Controller,
        source: ZoneKind::Library,
        object: ObjectPredicateDef::Any,
        minimum: 1,
        maximum: ValueDef::Constant(1),
        reveal: false,
        destination: ZoneKind::Hand,
        placement: ZonePlacement::Top,
        shuffle: true,
        enters_tapped: false,
        binding: None,
        then: None,
    },
    EffectDef::GainControl {
        object: EffectRecipientDef::Source,
        controller: PlayerRefDef::Opponent,
        // Nothing holds the change and no cleanup ends it: the artifact is
        // theirs from here (CR 611.2b).
        duration: ControlDurationDef::Indefinitely,
    },
];

// ELD 110 — Wishclaw Talisman
pub(in crate::card::sets) static WISHCLAW_TALISMAN: CardRecord = CardRecord::new(
    cards::WISHCLAW_TALISMAN,
    "Wishclaw Talisman",
    CardArt::new("07c17b01-ee5d-491a-8403-b3f819b778c4", "Daarken"),
    CardSet::ThroneOfEldraine,
    // Two mana for any card in the deck, and the price is handing the rest of
    // the artifact to the person it will be used against. The decks that play
    // it intend to win before that matters.
    CardRules::new_artifact(mana_cost!("{1}{B}")).with_abilities(&[
        AbilityDef::as_enters(
            "This artifact enters with three wish counters on it.",
            ReplacementEffectDef::ModifyBattlefieldEntry(
                BattlefieldEntryModificationDef::AddCounters {
                    kind: CounterKind::Wish,
                    amount: 3,
                },
            ),
        ),
        AbilityDef::activated(
            "{1}, {T}, Remove a wish counter from this artifact: Search your library for a card, put it into your hand, then shuffle. An opponent gains control of this artifact. Activate only during your turn.",
            &WISHCLAW_COSTS,
            EffectDef::Sequence(&WISHCLAW_GRANTS_A_WISH),
        )
        .with_activation_timing(ActivationTimingDef::YourTurn),
    ]),
);

// ELD 138 — Robber of the Rich
// Audit: blocked — Needs three things. An intervening-if that compares two players' hand sizes rather than a count against a printed number; a permission to cast one exiled card that survives its source leaving the battlefield and is gated on having attacked with a Rogue that turn; and spending mana as though it were mana of any color, which already blocks North Star in Legends.

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&WISHCLAW_TALISMAN];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
