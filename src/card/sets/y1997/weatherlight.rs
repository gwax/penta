//! Weatherlight cards used by the staged Premodern deck tranche.

use super::{CardRecord, PrintingRecord};
use crate::card::cards;
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AddManaEffectDef,
    AppliedEffectDef, AppliedRuleDef, BattlefieldEntryModificationDef, CardArt, CardRules, CardSet,
    CardType, CounterKind, EffectDef, EffectPaymentDef, EffectRecipientDef, ObjectPredicateDef,
    PayOrDef, PlayerRelation, PlayerSetDef, ReplacementEffectDef, ResolvedEffectDurationDef,
    TriggerEventDef, ZoneKind,
};
use crate::{TargetIndex, mana_cost};

/// The tax names spells an opponent casts, so it never touches your own.
static OPPONENTS_ARTIFACTS_AND_ENCHANTMENTS: ObjectPredicateDef = ObjectPredicateDef::AnyOf(&[
    ObjectPredicateDef::HasType(CardType::Artifact),
    ObjectPredicateDef::HasType(CardType::Enchantment),
]);

// WTH 7 — Aura of Silence
pub(in crate::card::sets) static AURA_OF_SILENCE: CardRecord = CardRecord::new(
    cards::AURA_OF_SILENCE,
    "Aura of Silence",
    CardArt::new(
        "57e6c366-b8c7-4f66-b8e1-82dc69c0081c",
        "D. Alexander Gregory",
    ),
    CardSet::Weatherlight,
    // It taxes while it sits and answers something on the way out, so the
    // opponent pays either way.
    CardRules::new_enchantment(mana_cost!("{1}{W}{W}")).with_abilities(&[
        AbilityDef::static_ability(
            "Artifact and enchantment spells your opponents cast cost {2} more to cast.",
            EffectDef::IncreaseMatchingSpellCostBy {
                spell: OPPONENTS_ARTIFACTS_AND_ENCHANTMENTS,
                caster: PlayerRelation::Opponent,
                amount: mana_cost!("{2}"),
            },
        ),
        AbilityDef::activated_with_targets(
            "Sacrifice this enchantment: Destroy target artifact or enchantment.",
            &[AbilityCostDef::SacrificeSource],
            &[AbilityTargetDef::exactly_one_permanent(
                OPPONENTS_ARTIFACTS_AND_ENCHANTMENTS,
            )],
            EffectDef::destroy_target(TargetIndex::PRIMARY, true),
        ),
    ]),
);

/// The artifact has to belong to the player being attacked, which in a
/// two-player game is the only opponent there is.
static DEFENDERS_ARTIFACT: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::HasType(CardType::Artifact),
        zones: &[ZoneKind::Battlefield],
        controller: Some(PlayerRelation::Opponent),
        owner: None,
    },
)];

/// Paying trades the hit for the artifact: the Vandal connects, and then
/// deals nothing because it spent the swing breaking something instead.
static VANDAL_TRADE: EffectDef = EffectDef::Sequence(&[
    EffectDef::Destroy {
        object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
        can_regenerate: true,
    },
    EffectDef::Apply {
        recipient: EffectRecipientDef::Source,
        effect: AppliedEffectDef::Rule(AppliedRuleDef::AssignsNoCombatDamage),
        duration: ResolvedEffectDurationDef::UntilEndOfTurn,
    },
]);

// WTH 105 — Goblin Vandal
pub(in crate::card::sets) static GOBLIN_VANDAL: CardRecord = CardRecord::new(
    cards::GOBLIN_VANDAL,
    "Goblin Vandal",
    CardArt::new("b7ad3b81-f706-4b33-b1ec-7600182a5232", "Franz Vohwinkel"),
    CardSet::Weatherlight,
    CardRules::new_creature(mana_cost!("{R}"), &["Goblin", "Rogue"], 1, 1).with_ability(
        AbilityDef::triggered_with_targets(
            "Whenever this creature attacks and isn't blocked, you may pay {R}. If you do, destroy target artifact defending player controls and this creature assigns no combat damage this turn.",
            TriggerEventDef::AttacksAndIsNotBlocked {
                attacker: ObjectPredicateDef::Source,
            },
            &DEFENDERS_ARTIFACT,
            EffectDef::PayOr(PayOrDef::optional(
                EffectPaymentDef::mana(
                    PlayerSetDef::Related(PlayerRelation::You),
                    mana_cost!("{R}"),
                ),
                &VANDAL_TRADE,
            )),
        ),
    ),
);

// WTH 164 — Gemstone Mine
pub(in crate::card::sets) static GEMSTONE_MINE: CardRecord = CardRecord::new(
    cards::GEMSTONE_MINE,
    "Gemstone Mine",
    CardArt::new("09507f7f-c58f-4f57-b878-b39811a5b619", "Brom"),
    CardSet::Weatherlight,
    // Three activations of perfect mana, and then nothing: the deck that
    // plays four of these is buying the first three turns, not the tenth.
    CardRules::new_land(&[]).with_abilities(&[
        AbilityDef::as_enters(
            "This land enters with three mining counters on it.",
            ReplacementEffectDef::ModifyBattlefieldEntry(
                BattlefieldEntryModificationDef::AddCounters {
                    kind: CounterKind::Mining,
                    amount: 3,
                },
            ),
        ),
        AbilityDef::activated_mana(
            "{T}, Remove a mining counter from this land: Add one mana of any color. If there are no mining counters on this land, sacrifice it.",
            &GEMSTONE_MINE_COSTS,
            EffectDef::AddMana(
                AddManaEffectDef::any_color().sacrificing_source_when_out_of(CounterKind::Mining),
            ),
        ),
    ]),
);

static GEMSTONE_MINE_COSTS: [AbilityCostDef; 2] = [
    AbilityCostDef::TapSource,
    AbilityCostDef::RemoveCountersFromSource {
        kind: CounterKind::Mining,
        amount: 1,
    },
];

pub(in crate::card::sets) static CARDS: &[&CardRecord] =
    &[&AURA_OF_SILENCE, &GOBLIN_VANDAL, &GEMSTONE_MINE];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
