//! Lost Caverns of Ixalan cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingRecord};
use crate::card::{
    AbilityDef, AbilityTargetDef, AbilityTargetPredicate, CardArt, CardRules, CardSet, CardType,
    ChoiceVisibilityDef, ChooseDef, EffectDef, EffectRecipientDef, InstalledTriggerDef,
    ObjectChoiceBindingDef, ObjectPredicateDef, ObjectQueryDef, ObjectRefDef, ObjectSetDef,
    PlayerRefDef, PlayerRelation, PlayerSetDef, TriggerEventDef, ZoneKind, abilities, cards,
};
use crate::ids::ObjectBindingIndex;
use crate::{TargetIndex, mana_cost};

/// "Until this creature leaves the battlefield" is one printed ability, so
/// the return rides on the same resolution as a delayed trigger rather than
/// appearing as a second clause the card does not print.
static BAT_RETURNS_IT: AbilityDef = AbilityDef::triggered(
    "When this creature leaves the battlefield, return the exiled card to its owner's hand.",
    TriggerEventDef::zone_changed(
        ObjectPredicateDef::Source,
        Some(ZoneKind::Battlefield),
        None,
    ),
    EffectDef::ReturnLinkedExiles {
        zone: ZoneKind::Hand,
        grant: None,
        controller: None,
        transformed: false,
    },
);

static BAT_EXILE: [EffectDef; 2] = [
    EffectDef::ExileLinkedToSource {
        object: EffectRecipientDef::object(ObjectRefDef::Binding(ObjectBindingIndex::PRIMARY)),
    },
    EffectDef::InstallTrigger(InstalledTriggerDef::once(&BAT_RETURNS_IT)),
];

static BAT_LOOKS_AND_MAY_TAKE: [EffectDef; 2] = [
    EffectDef::LookAtHand {
        player: EffectRecipientDef::Target(TargetIndex::PRIMARY),
    },
    // "You may exile" -- a minimum of none, so looking and taking nothing is
    // a legal answer. The Sculler and the Freebooter both must take one.
    EffectDef::Choose(ChooseDef {
        binding: ObjectChoiceBindingDef::Object(ObjectBindingIndex::PRIMARY),
        unchosen: None,
        chooser: PlayerRefDef::EffectController,
        candidates: ObjectSetDef::Query(ObjectQueryDef::owned_by(
            ObjectPredicateDef::Not(&ObjectPredicateDef::HasType(CardType::Land)),
            &[ZoneKind::Hand],
            PlayerSetDef::One(PlayerRefDef::Target(TargetIndex::PRIMARY)),
        )),
        exclude: None,
        minimum: 0,
        maximum: 1,
        visibility: ChoiceVisibilityDef::Public,
        then: &EffectDef::Sequence(&BAT_EXILE),
    }),
];

static BAT_TARGET: [AbilityTargetDef; 1] = [AbilityTargetDef::exactly_one(
    AbilityTargetPredicate::Player(PlayerRelation::Opponent),
)];

static DEEP_CAVERN_BAT_ABILITIES: [AbilityDef; 3] = [
    abilities::flying(),
    abilities::lifelink(),
    AbilityDef::triggered_with_targets(
        "When this creature enters, look at target opponent's hand. You may exile a nonland card from it until this creature leaves the battlefield.",
        TriggerEventDef::zone_changed(
            ObjectPredicateDef::Source,
            None,
            Some(ZoneKind::Battlefield),
        ),
        &BAT_TARGET,
        EffectDef::Sequence(&BAT_LOOKS_AND_MAY_TAKE),
    ),
];

// LCI 102 — Deep-Cavern Bat
pub(in crate::card::sets) static DEEP_CAVERN_BAT: CardRecord = CardRecord::new(
    cards::DEEP_CAVERN_BAT,
    "Deep-Cavern Bat",
    CardArt::new("69c68c95-b788-43b1-9f22-1b22c5a00b25", "Campbell White"),
    CardSet::LostCavernsOfIxalan,
    CardRules::new_creature(mana_cost!("{1}{B}"), &["Bat"], 1, 1)
        .with_abilities(&DEEP_CAVERN_BAT_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&DEEP_CAVERN_BAT];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
