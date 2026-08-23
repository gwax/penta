//! EXO card records required by supported formats.

use super::{CardRecord, PrintingAnchor, PrintingRecord};
use crate::card::{
    AbilityCoverageDef, AbilityDef, CardArt, CardRules, CardSet, CardType, ComparisonDef,
    EffectDef, EffectRecipientDef, ObjectPredicateDef, ObjectQueryDef, PlayerRelation,
    TriggerConditionDef, TriggerEventDef, TurnStepDef, ValueComparisonDef, ValueDef, ZoneKind,
};
use crate::mana_cost;

/// The creatures the player whose upkeep it is controls, and the ones they
/// do not. In a two-player game the second is exactly "their opponent's",
/// which is what the card asks about.
static CREATURES_THE_UPKEEP_PLAYER_CONTROLS: ObjectQueryDef = ObjectQueryDef::matching(
    ObjectPredicateDef::HasType(CardType::Creature),
    &[ZoneKind::Battlefield],
    PlayerRelation::EventPlayer,
);

static CREATURES_THEIR_OPPONENT_CONTROLS: ObjectQueryDef = ObjectQueryDef::matching(
    ObjectPredicateDef::HasType(CardType::Creature),
    &[ZoneKind::Battlefield],
    PlayerRelation::NotEventPlayer,
);

/// "Who controls more creatures than they do": the whole of what the target
/// clause decides in a two-player game, asked as the trigger is placed and
/// again as it resolves.
static THE_UPKEEP_PLAYER_IS_BEHIND: ValueComparisonDef = ValueComparisonDef {
    left: ValueDef::CountMatchingObjects(&CREATURES_THE_UPKEEP_PLAYER_CONTROLS),
    comparison: ComparisonDef::Less,
    right: ValueDef::CountMatchingObjects(&CREATURES_THEIR_OPPONENT_CONTROLS),
};

static OATH_CONDITION: TriggerConditionDef =
    TriggerConditionDef::ValueComparison(&THE_UPKEEP_PLAYER_IS_BEHIND);

/// The dig itself: everything passed over is buried, and the creature it
/// stops on arrives. Nothing is drawn and nothing is chosen -- which is why
/// the card is played to cheat one enormous creature into play rather than
/// to find a fair one.
static OATH_DIGS_FOR_A_CREATURE: EffectDef = EffectDef::MillUntil {
    player: EffectRecipientDef::EventPlayer,
    object: ObjectPredicateDef::HasType(CardType::Creature),
    matched_zone: ZoneKind::Battlefield,
    binding: None,
    then: None,
};

// EXO 53 — Carnophage
// Audit: metadata-only — Card rules have not been implemented.
pub(in crate::card::sets) static CARNOPHAGE: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("d17c057f-cb1b-4895-831a-fb35c75d3845"),
    "Carnophage",
    crate::card::CardArt::new("d17c057f-cb1b-4895-831a-fb35c75d3845", "Pete Venters"),
    crate::card::CardSet::Exodus,
    crate::card::CardRules::unsupported(),
);

// EXO 115 — Oath of Druids
pub(in crate::card::sets) static OATH_OF_DRUIDS: CardRecord = CardRecord::new(
    PrintingAnchor::scryfall("cf14de50-d123-400c-862e-2c95fd2aa23f"),
    "Oath of Druids",
    CardArt::new("cf14de50-d123-400c-862e-2c95fd2aa23f", "Daren Bader"),
    CardSet::Exodus,
    // Two mana that puts something enormous onto the battlefield for free,
    // for the deck that plays no creatures of its own and lets the other
    // player go first.
    CardRules::new_enchantment(mana_cost!("{1}{G}")).with_ability(
        AbilityDef::triggered_if(
            "At the beginning of each player's upkeep, that player chooses target player who \
             controls more creatures than they do and is their opponent. The first player may \
             reveal cards from the top of their library until they reveal a creature card. If the \
             first player does, that player puts that card onto the battlefield and all other \
             cards revealed this way into their graveyard.",
            TriggerEventDef::StepBegins {
                step: TurnStepDef::Upkeep,
                player: PlayerRelation::Any,
            },
            &OATH_CONDITION,
            EffectDef::May {
                player: EffectRecipientDef::EventPlayer,
                effect: &OATH_DIGS_FOR_A_CREATURE,
            },
        )
        .with_coverage(AbilityCoverageDef::partial(
            "The ability does not target. In a two-player game the printed target has exactly one \
             candidate and its legality is the condition checked here, so what happens is the \
             same -- but nothing that answers targeting sees this ability.",
        )),
    ),
);

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[&CARNOPHAGE, &OATH_OF_DRUIDS];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
