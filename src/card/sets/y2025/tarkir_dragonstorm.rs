//! Tarkir: Dragonstorm cards cataloged for the Vintage Cube pool.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityDef, AppliedEffectDef, AppliedRuleDef, CardArt, CardRules, CardSet, CardSupertype,
    ChoiceVisibilityDef, ChooseDef, ComparisonDef, CreatedTokensDef, EffectDef, EffectPaymentDef,
    EffectRecipientDef, InstalledTriggerDef, ManaColor, ObjectChoiceBindingDef, ObjectPredicateDef,
    ObjectQueryDef, ObjectSetDef, PayOrDef, PlayActionMatcherDef, PlayRestrictionDef, PlayerRefDef,
    PlayerRelation, PlayerSetDef, TriggerConditionDef, TriggerEventDef, TurnStepDef, ValueDef,
    ZoneKind, abilities,
};
use crate::ids::ObjectSetBindingIndex;
use crate::mana_cost;

/// The tokens go away at the next end step, and it has to be exactly the
/// ones this attack made: by then nothing about the board could tell them
/// apart from the pair the last attack made, or from a Warrior that arrived
/// some other way. So they are bound as they are created and the delayed
/// clause names the binding.
static MOBILIZE_SACRIFICE: EffectDef =
    EffectDef::InstallTrigger(InstalledTriggerDef::once(&AbilityDef::triggered(
        "At the beginning of the next end step, sacrifice those tokens.",
        TriggerEventDef::StepBegins {
            step: TurnStepDef::End,
            player: PlayerRelation::Any,
        },
        EffectDef::Sacrifice {
            object: EffectRecipientDef::objects(ObjectSetDef::Binding(
                ObjectSetBindingIndex::PRIMARY,
            )),
        },
    )));

/// Mobilize 2 (CR 702.180a). Written out rather than abbreviated: the
/// keyword is a shorthand for a triggered ability, and this is that ability.
static MOBILIZE_TWO: AbilityDef = AbilityDef::triggered(
    "Mobilize 2 (Whenever this creature attacks, create two tapped and attacking 1/1 red Warrior \
     creature tokens. Sacrifice them at the beginning of the next end step.)",
    TriggerEventDef::attacks(ObjectPredicateDef::Source),
    EffectDef::create_creature_token(&["Warrior"], &[ManaColor::Red], 1, 1)
        .with_art(CardArt::new(
            "7edc0515-a130-45a7-aa09-0e23bba41587",
            "Forrest Imel",
        ))
        .with_amount(2)
        .entering_tapped()
        .entering_attacking()
        .with_created_tokens(CreatedTokensDef {
            binding: ObjectSetBindingIndex::PRIMARY,
            then: &MOBILIZE_SACRIFICE,
        }),
);

static NO_SPELLS: PlayRestrictionDef =
    PlayRestrictionDef::new(PlayActionMatcherDef::CastSpell, ObjectPredicateDef::Any);

/// "During your turn" is the whole of the clause's timing, and it gates the
/// restriction rather than narrowing who it names: on their own turn the
/// same opponents may cast whatever they like.
static SILENCE_ON_YOUR_TURN: EffectDef = EffectDef::StaticApply {
    recipient: EffectRecipientDef::players(PlayerSetDef::Related(PlayerRelation::Opponent)),
    effect: AppliedEffectDef::Rule(AppliedRuleDef::CannotPlay(NO_SPELLS)),
};

static VOICE_OF_VICTORY_ABILITIES: [AbilityDef; 2] = [
    MOBILIZE_TWO,
    AbilityDef::static_ability(
        "Your opponents can't cast spells during your turn.",
        EffectDef::IfCondition {
            condition: &TriggerConditionDef::ActivePlayer(PlayerRelation::You),
            then: &SILENCE_ON_YOUR_TURN,
        },
    ),
];

/// "It endures 1": the counter or the Spirit, and the attacking body is
/// what either one is about.
static DESCENDANT_ENDURES: EffectDef = EffectDef::Endure {
    object: EffectRecipientDef::Source,
    amount: ValueDef::Constant(1),
};

// TDM 8 — Descendant of Storms
pub(in crate::card::sets) static DESCENDANT_OF_STORMS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("f632be90-9e7f-41f8-a52e-a2952354d730"),
    "Descendant of Storms",
    CardArt::new("f632be90-9e7f-41f8-a52e-a2952354d730", "Lie Setiawan"),
    CardSet::TarkirDragonstorm,
    // A one-mana 2/1 that attacks well early and has somewhere to put mana
    // late. Which half of endure you want changes with the board: the
    // counter makes the attack bigger, the Spirit makes the next one wider.
    CardRules::new_creature(mana_cost!("{W}"), &["Human", "Soldier"], 2, 1).with_ability(
        AbilityDef::triggered(
            "Whenever this creature attacks, you may pay {1}{W}. If you do, it endures 1.",
            TriggerEventDef::attacks(ObjectPredicateDef::Source),
            EffectDef::PayOr(PayOrDef::optional(
                EffectPaymentDef::mana(
                    PlayerSetDef::Related(PlayerRelation::You),
                    mana_cost!("{1}{W}"),
                ),
                &DESCENDANT_ENDURES,
            )),
        ),
    ),
);

// TDM 33 — Voice of Victory
pub(in crate::card::sets) static VOICE_OF_VICTORY: CardRecord = CardRecord::new_with_legacy_id(
    2282,
    "Voice of Victory",
    CardArt::new("ec3de5f4-bb55-4ab9-995f-f3e0dc22c1bb", "Joshua Cairos"),
    CardSet::TarkirDragonstorm,
    // Two mana that adds two power to every attack and turns off every
    // instant your opponent was holding for the turn you attack.
    CardRules::new_creature(mana_cost!("{1}{W}"), &["Human", "Bard"], 1, 3)
        .with_abilities(&VOICE_OF_VICTORY_ABILITIES),
);

/// "Discard up to two cards, then draw that many." The size is the player's
/// to choose, so the discard is a choice with a floor of none rather than a
/// fixed number, and what is drawn is however many that turned out to be.
static TERSA_REFILL: [EffectDef; 2] = [
    EffectDef::DiscardCards {
        object: EffectRecipientDef::objects(ObjectSetDef::Binding(ObjectSetBindingIndex::PRIMARY)),
    },
    EffectDef::DrawCards {
        recipient: EffectRecipientDef::Controller,
        amount: ValueDef::BoundObjectCount(ObjectSetBindingIndex::PRIMARY),
    },
];

static TERSA_LOOT: EffectDef = EffectDef::Choose(ChooseDef {
    binding: ObjectChoiceBindingDef::Objects(ObjectSetBindingIndex::PRIMARY),
    unchosen: None,
    chooser: PlayerRefDef::EffectController,
    candidates: ObjectSetDef::Query(ObjectQueryDef::owned_by(
        ObjectPredicateDef::Any,
        &[ZoneKind::Hand],
        PlayerSetDef::One(PlayerRefDef::EffectController),
    )),
    exclude: None,
    minimum: 0,
    maximum: 2,
    visibility: ChoiceVisibilityDef::Private,
    then: &EffectDef::Sequence(&TERSA_REFILL),
});

/// Seven cards is a real threshold rather than a formality: the attack that
/// turns it on is the one that has already spent a hand.
static SEVEN_IN_YOUR_GRAVEYARD: TriggerConditionDef = TriggerConditionDef::ObjectCount {
    query: ObjectQueryDef::matching(
        ObjectPredicateDef::Any,
        &[ZoneKind::Graveyard],
        PlayerRelation::You,
    ),
    comparison: ComparisonDef::GreaterOrEqual,
    amount: 7,
};

static TERSA_ABILITIES: [AbilityDef; 3] = [
    abilities::haste(),
    AbilityDef::triggered(
        "When Tersa Lightshatter enters, discard up to two cards, then draw that many cards.",
        TriggerEventDef::zone_changed(
            ObjectPredicateDef::Source,
            None,
            Some(ZoneKind::Battlefield),
        ),
        TERSA_LOOT,
    ),
    AbilityDef::triggered_if(
        "Whenever Tersa Lightshatter attacks, if there are seven or more cards in your graveyard, \
         exile a card at random from your graveyard. You may play that card this turn.",
        TriggerEventDef::attacks(ObjectPredicateDef::Source),
        &SEVEN_IN_YOUR_GRAVEYARD,
        EffectDef::ExileAtRandomFromGraveyardToPlay {
            player: EffectRecipientDef::Controller,
        },
    ),
];

// TDM 127 — Tersa Lightshatter
pub(in crate::card::sets) static TERSA_LIGHTSHATTER: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("39f07b5b-d764-4c88-920b-36b0ba1c62b0"),
    "Tersa Lightshatter",
    CardArt::new("39f07b5b-d764-4c88-920b-36b0ba1c62b0", "Olivier Bernard"),
    CardSet::TarkirDragonstorm,
    // Three mana for a 3/3 that attacks immediately and turns a spent hand
    // into a card a turn. What she asks for is the graveyard the deck was
    // filling anyway.
    CardRules::new_creature(mana_cost!("{2}{R}"), &["Orc", "Wizard"], 3, 3)
        .with_supertype(CardSupertype::Legendary)
        .with_abilities(&TERSA_ABILITIES),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[
    &DESCENDANT_OF_STORMS,
    &VOICE_OF_VICTORY,
    &TERSA_LIGHTSHATTER,
];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
