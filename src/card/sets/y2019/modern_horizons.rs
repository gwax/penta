//! Modern Horizons cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityCostDef, AbilityDef, AbilityTargetDef, AbilityTargetPredicate, ActivationTimingDef,
    AlternativeCastKindDef, CardArt, CardRules, CardSet, CardSupertype, CardType, EffectDef,
    EffectRecipientDef, ManaColor, ObjectPredicateDef, ObjectQueryDef, ObjectRefDef, ObjectSetDef,
    PlayerRefDef, PlayerRelation, SpellAdditionalCostDef, SpendModeDef, TriggerConditionDef,
    TriggerEventDef, ValueDef, ZoneKind, ZonePlacement, cards,
};
use crate::ids::ObjectSetBindingIndex;
use crate::{TargetIndex, mana_cost};

/// "You don't control" is a constraint on the slot rather than on the object:
/// a spell being cast is not a permanent, so a predicate that compares
/// controllers has nothing to compare against yet.
static WINDS_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::HasType(CardType::Creature),
        zones: &[ZoneKind::Battlefield],
        controller: Some(PlayerRelation::Opponent),
        owner: None,
    },
)];

static WINDS_SINGLE: [EffectDef; 2] = [
    EffectDef::MoveToZone {
        object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
        zone: ZoneKind::Exile,
        controller: None,
        placement: ZonePlacement::Top,
        arrival_effect: None,
        attachment: None,
    },
    // The searcher is the creature's controller, read from the announced
    // target: by now the creature is in exile and cannot be asked.
    EffectDef::SearchZone {
        player: EffectRecipientDef::player(PlayerRefDef::ControllerOf(ObjectRefDef::Target(
            TargetIndex::PRIMARY,
        ))),
        source: ZoneKind::Library,
        object: ObjectPredicateDef::All(&[
            ObjectPredicateDef::HasType(CardType::Land),
            ObjectPredicateDef::Supertype(CardSupertype::Basic),
        ]),
        minimum: 0,
        maximum: ValueDef::Constant(1),
        reveal: false,
        destination: ZoneKind::Battlefield,
        placement: ZonePlacement::Top,
        shuffle: true,
        enters_tapped: true,
        binding: None,
        then: None,
    },
];

static WINDS_OVERLOADED_CREATURES: ObjectQueryDef = ObjectQueryDef::matching(
    ObjectPredicateDef::HasType(CardType::Creature),
    &[ZoneKind::Battlefield],
    PlayerRelation::Opponent,
);

/// "For each creature exiled this way" counts what the exile actually took,
/// so the set is bound before it is emptied and the search reads the count
/// off that binding rather than off a board the creatures have left.
static WINDS_OVERLOADED_STEPS: [EffectDef; 2] = [
    EffectDef::MoveToZone {
        object: EffectRecipientDef::objects(ObjectSetDef::Binding(ObjectSetBindingIndex::PRIMARY)),
        zone: ZoneKind::Exile,
        controller: None,
        placement: ZonePlacement::Top,
        arrival_effect: None,
        attachment: None,
    },
    EffectDef::SearchZone {
        player: EffectRecipientDef::Opponent,
        source: ZoneKind::Library,
        object: ObjectPredicateDef::All(&[
            ObjectPredicateDef::HasType(CardType::Land),
            ObjectPredicateDef::Supertype(CardSupertype::Basic),
        ]),
        minimum: 0,
        maximum: ValueDef::BoundObjectCount(ObjectSetBindingIndex::PRIMARY),
        reveal: false,
        destination: ZoneKind::Battlefield,
        placement: ZonePlacement::Top,
        shuffle: true,
        enters_tapped: true,
        binding: None,
        then: None,
    },
];

static WINDS_OVERLOADED: EffectDef = EffectDef::BindMatching {
    objects: ObjectSetDef::Query(WINDS_OVERLOADED_CREATURES),
    binding: ObjectSetBindingIndex::PRIMARY,
    then: &EffectDef::Sequence(&WINDS_OVERLOADED_STEPS),
};

// MH1 37 — Winds of Abandon
pub(in crate::card::sets) static WINDS_OF_ABANDON: CardRecord = CardRecord::new(
    cards::WINDS_OF_ABANDON,
    "Winds of Abandon",
    CardArt::new("3bb17913-fe4d-4acd-9b75-71f5a90f898b", "Noah Bradley"),
    CardSet::ModernHorizons1,
    // Two mana answers one creature and six answers the board, and neither
    // half leaves anything behind to rebuild from -- exile rather than
    // destruction is the whole reason the card ends games.
    CardRules::new_sorcery(mana_cost!("{1}{W}")).with_abilities(&[
        AbilityDef::spell_with_targets(
            "Exile target creature you don't control. For each creature exiled this way, its controller searches their library for a basic land card. Those players put those cards onto the battlefield tapped, then shuffle.",
            &WINDS_TARGET,
            EffectDef::Sequence(&WINDS_SINGLE),
        ),
        AbilityDef::alternative_cast(
            mana_cost!("{4}{W}{W}"),
            AlternativeCastKindDef::Overload,
            Some("Exile each creature you don't control. For each creature exiled this way, its controller searches their library for a basic land card. Those players put those cards onto the battlefield tapped, then shuffle."),
            WINDS_OVERLOADED,
        ),
    ]),
);

/// Exiled rather than discarded: the card is spent without ever becoming a
/// graveyard card, which is what "exile a green card" means.
static EXILE_A_GREEN_CARD: SpellAdditionalCostDef = SpellAdditionalCostDef::new(
    ObjectPredicateDef::Color(ManaColor::Green),
    ZoneKind::Hand,
    1,
)
.spent(SpendModeDef::Exile);

/// "If it's not your turn" gates only the free cast. The printed cost is
/// always available, which is why this is a condition on the alternative
/// rather than a restriction on the card.
static NOT_YOUR_TURN: TriggerConditionDef =
    TriggerConditionDef::ActivePlayer(PlayerRelation::Opponent);

static FORCE_OF_VIGOR_TARGETS: [AbilityTargetDef; 1] = [AbilityTargetDef::up_to(
    AbilityTargetPredicate::Object {
        object: ObjectPredicateDef::AnyOf(&[
            ObjectPredicateDef::HasType(CardType::Artifact),
            ObjectPredicateDef::HasType(CardType::Enchantment),
        ]),
        zones: &[ZoneKind::Battlefield],
        controller: None,
        owner: None,
    },
    2,
)];

// MH1 164 — Force of Vigor
pub(in crate::card::sets) static FORCE_OF_VIGOR: CardRecord = CardRecord::new(
    cards::FORCE_OF_VIGOR,
    "Force of Vigor",
    CardArt::new("017c415b-d635-43c6-92b8-8c95d1c4ff8d", "Randy Vargas"),
    CardSet::ModernHorizons1,
    CardRules::new_instant(mana_cost!("{2}{G}{G}")).with_abilities(&[
        AbilityDef::alternative_cast(
            mana_cost!("{0}"),
            AlternativeCastKindDef::AlternativeCost,
            Some(
                "If it's not your turn, you may exile a green card from your hand rather than pay this spell's mana cost.",
            ),
            EffectDef::None,
        )
        .with_alternative_additional_cost(&EXILE_A_GREEN_CARD)
        .with_alternative_condition(&NOT_YOUR_TURN),
        AbilityDef::spell_with_targets(
            "Destroy up to two target artifacts and/or enchantments.",
            &FORCE_OF_VIGOR_TARGETS,
            EffectDef::Destroy {
                object: EffectRecipientDef::Target(TargetIndex::PRIMARY),
                can_regenerate: true,
            },
        ),
    ]),
);

static SHINOBI_NINJUTSU_COST: [AbilityCostDef; 2] = [
    AbilityCostDef::Mana(mana_cost!("{2}{U}{B}")),
    AbilityCostDef::ReturnUnblockedAttackerToHand,
];

static SHINOBI_ABILITIES: [AbilityDef; 2] = [
    AbilityDef::activated(
        "Ninjutsu {2}{U}{B} ({2}{U}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)",
        &SHINOBI_NINJUTSU_COST,
        EffectDef::PutSourceOntoBattlefieldAttacking,
    )
    .with_source_zones(&[ZoneKind::Hand])
    .with_activation_timing(ActivationTimingDef::AfterAttackersDeclared),
    AbilityDef::triggered(
        "Whenever this creature deals combat damage to a player, that player exiles the top two cards of their library. Until end of turn, you may play those cards without paying their mana costs.",
        TriggerEventDef::combat_damage_to_player(ObjectPredicateDef::Source),
        EffectDef::ExileTopOfLibraryToPlay {
            player: EffectRecipientDef::EventPlayer,
            amount: ValueDef::Constant(2),
        },
    ),
];

// MH1 199 — Fallen Shinobi
pub(in crate::card::sets) static FALLEN_SHINOBI: CardRecord = CardRecord::new(
    cards::FALLEN_SHINOBI,
    "Fallen Shinobi",
    CardArt::new("900c9dfd-ece1-4b09-a801-0fa05e1994b9", "Tomasz Jedruszek"),
    CardSet::ModernHorizons1,
    // Ninjutsu is what makes a five-mana 5/4 connect on turn three, and
    // connecting is the whole card: two cards off the top of their deck,
    // free, every time.
    CardRules::new_creature(mana_cost!("{3}{U}{B}"), &["Zombie", "Ninja"], 5, 4)
        .with_abilities(&SHINOBI_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] =
    &[&WINDS_OF_ABANDON, &FORCE_OF_VIGOR, &FALLEN_SHINOBI];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
