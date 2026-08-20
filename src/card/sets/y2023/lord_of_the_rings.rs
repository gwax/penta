//! The Lord of the Rings: Tales of Middle-earth cards cataloged for the
//! Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::TargetIndex;
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, AddManaEffectDef,
    AppliedEffectDef, AppliedRuleDef, CardArt, CardRules, CardSet, CardSupertype, CardType,
    ConditionDef, EffectDef, EffectRecipientDef, ManaColor, ManaRestrictionDef, ManaSpendEffectDef,
    ObjectPredicateDef, ObjectQueryDef, PlayerRelation, PlayerSetDef, TriggerEventDef, ValueDef,
    ZoneKind, abilities, cards,
};
use crate::mana_cost;

static REPRIEVE_TARGET: [AbilityTargetDef; 1] =
    [AbilityTargetDef::exactly_one_spell(ObjectPredicateDef::Any)];

/// Returning the spell is not countering it, so a spell that cannot be
/// countered is answered all the same -- and its controller keeps the card,
/// which is the price. Drawing pays for the tempo either way.
static REPRIEVE_EFFECTS: [EffectDef; 2] = [
    EffectDef::ReturnSpellToHand {
        object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
    },
    EffectDef::DrawCards {
        recipient: EffectRecipientDef::Controller,
        amount: ValueDef::Constant(1),
    },
];

// LTR 26 — Reprieve
pub(in crate::card::sets) static REPRIEVE: CardRecord = CardRecord::new(
    cards::REPRIEVE,
    "Reprieve",
    CardArt::new("1bd3fa8a-6c50-4f7f-9ae3-0810eec5e3db", "Justyna Dura"),
    CardSet::LordOfTheRings,
    CardRules::new_instant(mana_cost!("{1}{W}")).with_ability(AbilityDef::spell_with_targets(
        "Return target spell to its owner's hand.\nDraw a card.",
        &REPRIEVE_TARGET,
        EffectDef::Sequence(&REPRIEVE_EFFECTS),
    )),
);

/// "Power or toughness 2 or less" is a disjunction, not a pair of bounds: a
/// 5/1 is small enough and a 1/5 is too. Written as "less than 3" because
/// that is the comparison the predicate offers.
static STERN_SCOLDING_TARGET: AbilityTargetDef =
    AbilityTargetDef::exactly_one_spell(ObjectPredicateDef::All(&[
        ObjectPredicateDef::HasType(CardType::Creature),
        ObjectPredicateDef::AnyOf(&[
            ObjectPredicateDef::PowerLessThan(ValueDef::Constant(3)),
            ObjectPredicateDef::ToughnessLessThan(ValueDef::Constant(3)),
        ]),
    ]));

// LTR 60 — Lórien Revealed
pub(in crate::card::sets) static LORIEN_REVEALED: CardRecord = CardRecord::new(
    cards::LORIEN_REVEALED,
    "Lórien Revealed",
    CardArt::new("0ce44270-a684-4489-9077-521456e6dfaa", "Randy Gallegos"),
    CardSet::LordOfTheRings,
    // Five mana is not what this card is for either. One mana from hand for
    // an Island is, and the three cards are what makes the last copy in the
    // deck worth drawing.
    CardRules::new_sorcery(mana_cost!("{3}{U}{U}")).with_abilities(&[
        AbilityDef::spell(
            "Draw three cards.",
            EffectDef::DrawCards {
                recipient: EffectRecipientDef::Controller,
                amount: ValueDef::Constant(3),
            },
        ),
        abilities::typecycling(
            "Islandcycling {1} ({1}, Discard this card: Search your library for an Island card, reveal it, put it into your hand, then shuffle.)",
            mana_cost!("{1}"),
            ObjectPredicateDef::Subtype("Island"),
        ),
    ]),
);

// LTR 71 — Stern Scolding
pub(in crate::card::sets) static STERN_SCOLDING: CardRecord = CardRecord::new(
    cards::STERN_SCOLDING,
    "Stern Scolding",
    CardArt::new("3ca1e1de-b916-445f-b3b2-0f4d0cc7ceeb", "Valera Lutfullina"),
    CardSet::LordOfTheRings,
    CardRules::new_instant(mana_cost!("{U}")).with_ability(AbilityDef::counter_target(
        "Counter target creature spell with power or toughness 2 or less.",
        &STERN_SCOLDING_TARGET,
    )),
);

// LTR 103 — Orcish Bowmasters
// Audit: blocked — Needs two things. A trigger on an opponent drawing a card, which no event here raises, and which this card qualifies further: every draw except the first one in each of that player's draw steps, so the count has to be kept per player per turn. And amass, which is a conditional token creation followed by a chosen Army taking counters and gaining a creature type.

static HALFLING_MANA_RESTRICTIONS: [ManaRestrictionDef; 1] = [ManaRestrictionDef::CastSpell(
    ObjectPredicateDef::Supertype(CardSupertype::Legendary),
)];

/// The rider is the reason the card is played: uncounterable is not a
/// property of the Halfling but of whatever its mana paid for.
static HALFLING_MANA_SPEND_EFFECTS: [ManaSpendEffectDef; 1] =
    [ManaSpendEffectDef::ApplyToPaidSpell(
        AppliedEffectDef::Rule(AppliedRuleDef::CannotBeCountered),
    )];

// LTR 158 — Delighted Halfling
pub(in crate::card::sets) static DELIGHTED_HALFLING: CardRecord = CardRecord::new(
    cards::DELIGHTED_HALFLING,
    "Delighted Halfling",
    CardArt::new("71384418-173a-4f77-adab-56e52fa23692", "Inka Schulz"),
    CardSet::LordOfTheRings,
    CardRules::new_creature(mana_cost!("{G}"), &["Halfling", "Citizen"], 1, 2).with_abilities(&[
        abilities::tap_for(ManaColor::Colorless),
        AbilityDef::activated_mana(
            "{T}: Add one mana of any color. Spend this mana only to cast a legendary spell, and that spell can't be countered.",
            &[AbilityCostDef::TapSource],
            EffectDef::AddMana(
                AddManaEffectDef::any_color()
                    .with_restrictions(&HALFLING_MANA_RESTRICTIONS)
                    .with_spend_effects(&HALFLING_MANA_SPEND_EFFECTS),
            ),
        ),
    ]),
);

// LTR 169 — Generous Ent
pub(in crate::card::sets) static GENEROUS_ENT: CardRecord = CardRecord::new(
    cards::GENEROUS_ENT,
    "Generous Ent",
    CardArt::new("85d22d5d-3875-42ff-b51e-c6e21db201f5", "Simon Dominic"),
    CardSet::LordOfTheRings,
    CardRules::new_creature(mana_cost!("{5}{G}"), &["Treefolk"], 5, 7).with_abilities(&[
        abilities::reach(),
        AbilityDef::triggered(
            "When this creature enters, create a Food token.",
            TriggerEventDef::zone_changed(
                ObjectPredicateDef::Source,
                None,
                Some(ZoneKind::Battlefield),
            ),
            EffectDef::CreateToken {
                token: cards::FOOD_TOKEN,
                count: ValueDef::Constant(1),
                tapped: false,
            },
        ),
        // Six mana is not what this card is for. Forestcycling is: one mana
        // from hand, and the Ent becomes the land the draw did not give you.
        abilities::typecycling(
            "Forestcycling {1} ({1}, Discard this card: Search your library for a Forest card, reveal it, put it into your hand, then shuffle.)",
            mana_cost!("{1}"),
            ObjectPredicateDef::Subtype("Forest"),
        ),
    ]),
);

/// The condition is read as the spell is cast, not as it resolves, so a
/// Wizard that dies in response has already done its work.
static FLAME_OF_ANOR_WIZARD: ConditionDef = ConditionDef::Exists(ObjectQueryDef::controlled_by(
    ObjectPredicateDef::Subtype("Wizard"),
    &[ZoneKind::Battlefield],
    PlayerSetDef::Related(PlayerRelation::You),
));

static FLAME_OF_ANOR_MODES: [AbilityDef; 3] = [
    AbilityDef::spell_with_targets(
        "Target player draws two cards.",
        &FLAME_OF_ANOR_DRAW_TARGET,
        EffectDef::DrawCards {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::Constant(2),
        },
    ),
    AbilityDef::destroy_target(
        "Destroy target artifact.",
        &FLAME_OF_ANOR_ARTIFACT_TARGET,
        true,
    ),
    AbilityDef::spell_with_targets(
        "This spell deals 5 damage to target creature.",
        &FLAME_OF_ANOR_CREATURE_TARGET,
        EffectDef::DealDamage {
            recipient: EffectRecipientDef::Target(TargetIndex::PRIMARY),
            amount: ValueDef::Constant(5),
        },
    ),
];

static FLAME_OF_ANOR_DRAW_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Player(PlayerRelation::Any),
)];

static FLAME_OF_ANOR_ARTIFACT_TARGET: AbilityTargetDef =
    AbilityTargetDef::exactly_one_permanent(ObjectPredicateDef::HasType(CardType::Artifact));

static FLAME_OF_ANOR_CREATURE_TARGET: [AbilityTargetDef; 1] =
    [AbilityTargetDef::exactly_one_permanent(
        ObjectPredicateDef::HasType(CardType::Creature),
    )];

// LTR 203 — Flame of Anor
pub(in crate::card::sets) static FLAME_OF_ANOR: CardRecord = CardRecord::new(
    cards::FLAME_OF_ANOR,
    "Flame of Anor",
    CardArt::new("04779a7e-b453-48b9-b392-6d6fd0b8d283", "Yigit Koroglu"),
    CardSet::LordOfTheRings,
    CardRules::new_instant(mana_cost!("{1}{U}{R}")).with_ability(
        AbilityDef::modal_spell(
            "Choose one. If you control a Wizard as you cast this spell, you may choose two instead.\n• Target player draws two cards.\n• Destroy target artifact.\n• This spell deals 5 damage to target creature.",
            &FLAME_OF_ANOR_MODES,
            1,
            1,
            false,
        )
        .with_conditional_mode_maximum(FLAME_OF_ANOR_WIZARD, 2),
    ),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &REPRIEVE,
    &LORIEN_REVEALED,
    &STERN_SCOLDING,
    &DELIGHTED_HALFLING,
    &GENEROUS_ENT,
    &FLAME_OF_ANOR,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
