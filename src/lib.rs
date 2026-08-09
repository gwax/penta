//! Deterministic engine primitives for supported two-player Magic formats.

pub mod action;
pub mod card;
pub mod casting;
pub mod deck;
pub mod decks;
pub mod format;
pub mod game;
pub mod ids;
pub mod poc;
pub mod policy;
pub mod protocol;
mod rng;
pub mod rules;

pub use action::{AbilityOrigin, Action, ActionError, CombatDamageAssignment, ManaColor, Target};
pub use card::{
    AbilityCostDef, AbilityCostList, AbilityDef, AbilityImplementationDef, AbilityTargetDef,
    AbilityTargetPredicate, ActivatedAbilityDef, ActivatedAbilityText, AddManaEffectDef,
    AdditionalCostDef, AlternateSpellKind, AlternativeCastAbilityDef, AlternativeCastKindDef,
    AlternativeCastManaCostDef, AlternativeCostDef, AppliedEffectDef, AttachedAbilityDef,
    BasicLandType, CREATURE_TYPES, CardAbilityList, CardArt, CardBehavior, CardCatalog,
    CardComposition, CardDefinition, CardEffectStatus, CardPart, CardPrinting, CardPrintingId,
    CardRules, CardSet, CardStructure, CardSupertype, CardType, CardTypeSet, CatalogError,
    CharacteristicContext, CharacteristicError, ColorSet, CreatureStats, DeclarativeAbilityDef,
    DoubleFacedKind, EffectDef, EffectDurationDef, EffectRecipientDef,
    GrantedAbilityValidationError, ImplementationStatus, KeywordAbility, LandEntry, ManaCost,
    ManaCostParseError, ManaCostParseErrorKind, ManaRestrictionDef, ManaSelectionDef,
    ManaSpendEffectDef, MeldComponentDef, MeldRecipeDef, MeldResultDef, ModalSpellDef, ModeDef,
    ModeSetDef, ObjectPredicateDef, PlayActionKind, PlayOptionDef, PlayRestriction, PlayerRelation,
    PrintedManaCost, ReplacementAbilityDef, ReplacementEventDef, SpecialActionDef, SpellAbilityDef,
    SpellForm, StaticAbilityDef, TargetPredicate, TargetSlotDef, TriggerEventDef,
    TriggeredAbilityDef, TurnStepDef, ValueDef, ZoneKind, ZoneMoveCauseDef, applicable_part_ids,
};
pub use casting::{
    CastChoices, CastSignature, CostConfiguration, TargetReplacementError, TargetSelection,
};
pub use deck::{Deck, DeckError, ValidatedDeck};
pub use format::{Format, FormatRules};
pub use game::{
    BattlefieldExit, DecisionObservation, DecisionOption, DecisionPreference, DecisionVisibility,
    DecisionZone, Game, GameError, GameEvent, GameResult, Mana, ManaPool, ManaSource,
    PlayerObservation, StackObjectKind, Step, WinReason, ZoneCard, ZoneChangeOutcome, ZoneError,
};
pub use ids::{
    AbilityId, AdditionalCostId, AlternativeCostId, CardDefinitionId, CardInstanceId, CardPartId,
    GameObjectId, GrantId, MeldRecipeId, ModeId, PhysicalCardId, PlayOptionId, PlayerId,
    StackObjectId, TargetSlotId,
};
pub use policy::{HandcraftedPolicy, PlayError, Policy, RandomPolicy, play_game};
