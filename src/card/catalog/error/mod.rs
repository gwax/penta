mod display;

use std::fmt;

use crate::card::{
    CardEffectStatus, CardPrintingId, ManaCost, ObjectChoiceBindingDef, PlayActionKind,
    PlayerSetDef, ReplacementEventDef, SpellForm, TargetSlotDef, TriggerEventDef,
};
use crate::{
    AbilityId, AdditionalCostId, AlternativeCostId, CardDefinitionId, CardPartId, GrantId, ModeId,
    ObjectBindingIndex, ObjectSetBindingIndex, PlayOptionId, TargetIndex, TargetSlotId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GrantedAbilityValidationError {
    TooManyGrantSites {
        count: usize,
    },
    EmptyText,
    MissingImplementationExplanation,
    LegacyProcedureRequiresCustomExecution,
    HasNoSourceZone,
    ManaAbilityHasTargets,
    /// Replacement abilities mutate prospective events and therefore require
    /// the replacement-program vocabulary rather than an ordinary resolving
    /// effect program.
    ReplacementAbilityRequiresReplacementProgram,
    /// Only replacement abilities have a prospective event whose state a
    /// replacement program can mutate.
    ReplacementProgramRequiresReplacementAbility,
    /// The replacement program contains an operation that the prospective
    /// event's shared procedure cannot execute.
    UnsupportedReplacementProgram {
        event: ReplacementEventDef,
        operation: &'static str,
    },
    /// Installed triggers currently reuse their installer's target namespace
    /// and the ordinary shared trigger runtime. Reject any nested ability the
    /// runtime would otherwise silently decline to install.
    UnsupportedInstalledTriggerAbility,
    /// The shared trigger publisher cannot produce or match this event shape.
    UnsupportedTriggerEvent {
        event: TriggerEventDef,
    },
    /// A shared triggered mana ability must resolve immediately without
    /// choices or stack-only effects. The runtime supports one or more fixed
    /// `AddMana` leaves and nothing else.
    UnsupportedTriggeredManaProgram,
    TooManyTargets {
        count: usize,
    },
    InvalidTargetBounds {
        target: TargetIndex,
        minimum: u8,
        maximum: u8,
    },
    TargetReferenceOutOfBounds {
        target: TargetIndex,
        target_count: usize,
    },
    InvalidObjectChoiceBounds {
        binding: ObjectChoiceBindingDef,
        minimum: usize,
        maximum: usize,
    },
    InvalidPileRole {
        role: &'static str,
        players: PlayerSetDef,
    },
    ObjectBindingReferenceOutOfScope {
        binding: ObjectBindingIndex,
    },
    ObjectBindingAlreadyInScope {
        binding: ObjectBindingIndex,
    },
    ObjectSetBindingReferenceOutOfScope {
        binding: ObjectSetBindingIndex,
    },
    ObjectSetBindingAlreadyInScope {
        binding: ObjectSetBindingIndex,
    },
    /// Runtime static-effect discovery currently starts from attached printed
    /// or copied clauses. Reject an executable static ability granted by
    /// another ability until continuous effects have guarded fixed-point
    /// evaluation rather than silently claiming support.
    ExecutableStaticAbility,
}

impl fmt::Display for GrantedAbilityValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyGrantSites { count } => write!(
                formatter,
                "defines {count} grant sites, but grant IDs support at most 256"
            ),
            Self::EmptyText => formatter.write_str("has empty rules text"),
            Self::MissingImplementationExplanation => formatter.write_str(
                "has a non-declarative implementation without an explanation",
            ),
            Self::LegacyProcedureRequiresCustomExecution => formatter.write_str(
                "uses the legacy rules procedure without a custom effect executor",
            ),
            Self::HasNoSourceZone => formatter.write_str("has no source zone"),
            Self::ManaAbilityHasTargets => formatter.write_str("is a mana ability that declares targets"),
            Self::ReplacementAbilityRequiresReplacementProgram => formatter.write_str(
                "is a replacement ability but does not define a replacement program",
            ),
            Self::ReplacementProgramRequiresReplacementAbility => formatter.write_str(
                "defines a replacement program but is not a replacement ability",
            ),
            Self::UnsupportedReplacementProgram { event, operation } => write!(
                formatter,
                "uses unsupported replacement operation {operation} for event {event:?}",
            ),
            Self::UnsupportedInstalledTriggerAbility => formatter.write_str(
                "installs an ability that is not a targetless shared declarative triggered ability",
            ),
            Self::UnsupportedTriggerEvent { event } => {
                write!(formatter, "uses unsupported trigger event {event:?}")
            }
            Self::UnsupportedTriggeredManaProgram => formatter.write_str(
                "uses a triggered mana program that cannot resolve immediately",
            ),
            Self::TooManyTargets { count } => write!(
                formatter,
                "defines {count} targets, but positional target indices support at most 256"
            ),
            Self::InvalidTargetBounds {
                target,
                minimum,
                maximum,
            } => write!(
                formatter,
                "defines target {target:?} requiring at least {minimum} targets but allowing at most {maximum}",
            ),
            Self::TargetReferenceOutOfBounds {
                target,
                target_count,
            } => write!(
                formatter,
                "references target {target:?}, but the clause defines only {target_count} target slots"
            ),
            Self::InvalidObjectChoiceBounds {
                binding,
                minimum,
                maximum,
            } => write!(
                formatter,
                "binds {binding:?} from a choice requiring at least {minimum} objects and allowing at most {maximum}"
            ),
            Self::InvalidPileRole { role, players } => write!(
                formatter,
                "uses {players:?} for pile {role}, but that role must select at most one player"
            ),
            Self::ObjectBindingReferenceOutOfScope { binding } => {
                write!(formatter, "references object binding {binding:?} outside its scope")
            }
            Self::ObjectBindingAlreadyInScope { binding } => write!(
                formatter,
                "binds object slot {binding:?}, but that slot is already bound in this scope"
            ),
            Self::ObjectSetBindingReferenceOutOfScope { binding } => write!(
                formatter,
                "references object-set binding {binding:?} outside its scope"
            ),
            Self::ObjectSetBindingAlreadyInScope { binding } => write!(
                formatter,
                "binds object-set slot {binding:?}, but that slot is already bound in this scope"
            ),
            Self::ExecutableStaticAbility => formatter.write_str(
                "is an executable static ability, but granted static abilities are not evaluated yet",
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CatalogError {
    DuplicateId(CardDefinitionId),
    DuplicateName(String),
    DuplicatePrintingId(CardPrintingId),
    MismatchedPrintingDefinition {
        definition: CardDefinitionId,
        printing: CardPrintingId,
    },
    OrphanPrinting(CardPrintingId),
    EmptyAbilityText {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
    },
    MissingImplementationExplanation {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
    },
    LegacyProcedureRequiresCustomExecution {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
    },
    DuplicatePartId {
        definition: CardDefinitionId,
        part: CardPartId,
    },
    IncoherentCardRules {
        definition: CardDefinitionId,
        part: CardPartId,
        explanation: &'static str,
    },
    MismatchedPrimaryRules {
        definition: CardDefinitionId,
        part: CardPartId,
    },
    TooManyAbilities {
        definition: CardDefinitionId,
        part: CardPartId,
        count: usize,
    },
    MultipleSpellAbilities {
        definition: CardDefinitionId,
        part: CardPartId,
        count: usize,
    },
    InvalidModalSpellParent {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
    },
    TooManySpellModes {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        count: usize,
    },
    InvalidModalSpellSelection {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        minimum: u8,
        maximum: u8,
        may_repeat: bool,
        available: usize,
    },
    NonSpellMode {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        mode: ModeId,
    },
    NestedModalSpellMode {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        mode: ModeId,
    },
    CustomSpellModeImplementation {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        mode: ModeId,
    },
    InvalidSpellMode {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        mode: ModeId,
        problem: GrantedAbilityValidationError,
    },
    TooManyAbilityGrantSites {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        count: usize,
    },
    InvalidGrantedAbility {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        grant_path: Vec<GrantId>,
        problem: GrantedAbilityValidationError,
    },
    AbilityHasNoSourceZone {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
    },
    ManaAbilityHasTargets {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
    },
    ReplacementAbilityRequiresReplacementProgram {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
    },
    ReplacementProgramRequiresReplacementAbility {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
    },
    UnsupportedReplacementProgram {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        event: ReplacementEventDef,
        operation: &'static str,
    },
    UnsupportedInstalledTriggerAbility {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
    },
    UnsupportedTriggerEvent {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        event: TriggerEventDef,
    },
    UnsupportedTriggeredManaProgram {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
    },
    TooManyAbilityTargets {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        count: usize,
    },
    InvalidAbilityTargetBounds {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        target: TargetIndex,
        minimum: u8,
        maximum: u8,
    },
    AbilityTargetReferenceOutOfBounds {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        target: TargetIndex,
        target_count: usize,
    },
    InvalidAbilityObjectChoiceBounds {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        binding: ObjectChoiceBindingDef,
        minimum: usize,
        maximum: usize,
    },
    InvalidAbilityPileRole {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        role: &'static str,
        players: PlayerSetDef,
    },
    AbilityObjectBindingReferenceOutOfScope {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        binding: ObjectBindingIndex,
    },
    AbilityObjectBindingAlreadyInScope {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        binding: ObjectBindingIndex,
    },
    AbilityObjectSetBindingReferenceOutOfScope {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        binding: ObjectSetBindingIndex,
    },
    AbilityObjectSetBindingAlreadyInScope {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        binding: ObjectSetBindingIndex,
    },
    DuplicateStructurePart {
        definition: CardDefinitionId,
        part: CardPartId,
    },
    InvalidSplitPartCount {
        definition: CardDefinitionId,
        actual: usize,
    },
    UndefinedStructurePart {
        definition: CardDefinitionId,
        part: CardPartId,
    },
    PartOutsideStructure {
        definition: CardDefinitionId,
        part: CardPartId,
    },
    DuplicatePlayOptionId {
        definition: CardDefinitionId,
        option: PlayOptionId,
    },
    EmptySpellForm {
        definition: CardDefinitionId,
        option: PlayOptionId,
    },
    DuplicateSpellFormPart {
        definition: CardDefinitionId,
        option: PlayOptionId,
        part: CardPartId,
    },
    UndefinedSpellFormPart {
        definition: CardDefinitionId,
        option: PlayOptionId,
        part: CardPartId,
    },
    SpellFormPartOutsideStructure {
        definition: CardDefinitionId,
        option: PlayOptionId,
        part: CardPartId,
    },
    MissingFusedPlayOption {
        definition: CardDefinitionId,
        option: PlayOptionId,
    },
    InvalidFusedPlayOption {
        definition: CardDefinitionId,
        option: PlayOptionId,
        expected: Vec<CardPartId>,
        actual: SpellForm,
        actual_action: PlayActionKind,
    },
    UnexpectedCombinedSpellForm {
        definition: CardDefinitionId,
        option: PlayOptionId,
    },
    CombinedModalSpellUnsupported {
        definition: CardDefinitionId,
        option: PlayOptionId,
        part: CardPartId,
    },
    DuplicateModeId {
        definition: CardDefinitionId,
        option: PlayOptionId,
        mode: ModeId,
    },
    NonPositionalModeId {
        definition: CardDefinitionId,
        option: PlayOptionId,
        expected: ModeId,
        actual: ModeId,
    },
    EmptyModeSet {
        definition: CardDefinitionId,
        option: PlayOptionId,
    },
    TooManyModes {
        definition: CardDefinitionId,
        option: PlayOptionId,
        count: usize,
    },
    InvalidModeBounds {
        definition: CardDefinitionId,
        option: PlayOptionId,
        minimum: u8,
        maximum: u8,
    },
    ZeroModeMaximum {
        definition: CardDefinitionId,
        option: PlayOptionId,
    },
    TooManyModesWithoutRepetition {
        definition: CardDefinitionId,
        option: PlayOptionId,
        maximum: u8,
        available: usize,
    },
    UnexpectedPresentationSpellModes {
        definition: CardDefinitionId,
        option: PlayOptionId,
    },
    MissingPresentationSpellTarget {
        definition: CardDefinitionId,
        option: PlayOptionId,
        target: TargetSlotId,
    },
    MissingSemanticSpellTarget {
        definition: CardDefinitionId,
        option: PlayOptionId,
        target: TargetSlotId,
    },
    MismatchedSpellTargetCardinality {
        definition: CardDefinitionId,
        option: PlayOptionId,
        target: TargetSlotId,
        presentation_minimum: u8,
        presentation_maximum: u8,
        semantic_minimum: u8,
        semantic_maximum: u8,
    },
    UnpresentableSpellTarget {
        definition: CardDefinitionId,
        option: PlayOptionId,
        target: TargetSlotId,
    },
    MismatchedSpellTargetPresentation {
        definition: CardDefinitionId,
        option: PlayOptionId,
        position: usize,
        presentation: TargetSlotDef,
        semantic: TargetSlotDef,
    },
    UnexpectedModalSpellTargets {
        definition: CardDefinitionId,
        option: PlayOptionId,
        count: usize,
    },
    MissingPresentationSpellMode {
        definition: CardDefinitionId,
        option: PlayOptionId,
        mode: ModeId,
    },
    MissingSemanticSpellMode {
        definition: CardDefinitionId,
        option: PlayOptionId,
        mode: ModeId,
    },
    MissingPresentationSpellModeTarget {
        definition: CardDefinitionId,
        option: PlayOptionId,
        mode: ModeId,
        target: TargetSlotId,
    },
    MissingSemanticSpellModeTarget {
        definition: CardDefinitionId,
        option: PlayOptionId,
        mode: ModeId,
        target: TargetSlotId,
    },
    MismatchedSpellModeTargetCardinality {
        definition: CardDefinitionId,
        option: PlayOptionId,
        mode: ModeId,
        target: TargetSlotId,
        presentation_minimum: u8,
        presentation_maximum: u8,
        semantic_minimum: u8,
        semantic_maximum: u8,
    },
    UnpresentableSpellModeTarget {
        definition: CardDefinitionId,
        option: PlayOptionId,
        mode: ModeId,
        target: TargetSlotId,
    },
    MismatchedSpellModeTargetPresentation {
        definition: CardDefinitionId,
        option: PlayOptionId,
        mode: ModeId,
        position: usize,
        presentation: TargetSlotDef,
        semantic: TargetSlotDef,
    },
    MismatchedSpellModeSelection {
        definition: CardDefinitionId,
        option: PlayOptionId,
        presentation_minimum: u8,
        presentation_maximum: u8,
        presentation_may_repeat: bool,
        semantic_minimum: u8,
        semantic_maximum: u8,
        semantic_may_repeat: bool,
    },
    MismatchedSpellModeImplementation {
        definition: CardDefinitionId,
        option: PlayOptionId,
        mode: ModeId,
        presentation: CardEffectStatus,
        semantic: CardEffectStatus,
    },
    MismatchedSpellModeLabel {
        definition: CardDefinitionId,
        option: PlayOptionId,
        mode: ModeId,
        presentation: String,
        semantic: &'static str,
    },
    DuplicateAlternativeCostId {
        definition: CardDefinitionId,
        option: PlayOptionId,
        cost: AlternativeCostId,
    },
    MissingAlternativeCostForAbility {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        cost: AlternativeCostId,
    },
    MismatchedAlternativeCostForAbility {
        definition: CardDefinitionId,
        part: CardPartId,
        ability: AbilityId,
        option: PlayOptionId,
        cost: AlternativeCostId,
        expected_label: String,
        actual_label: String,
        expected_mana_cost: ManaCost,
        actual_mana_cost: ManaCost,
    },
    DuplicateAdditionalCostId {
        definition: CardDefinitionId,
        cost: AdditionalCostId,
    },
    InvalidTargetBounds {
        definition: CardDefinitionId,
        option: PlayOptionId,
        mode: Option<ModeId>,
        slot: TargetSlotId,
        minimum: u8,
        maximum: u8,
    },
    TooManyTargetSlots {
        definition: CardDefinitionId,
        option: PlayOptionId,
        mode: Option<ModeId>,
        count: usize,
    },
    NonPositionalTargetSlot {
        definition: CardDefinitionId,
        option: PlayOptionId,
        mode: Option<ModeId>,
        expected: TargetSlotId,
        actual: TargetSlotId,
    },
    TooManyInstantiatedTargets {
        definition: CardDefinitionId,
        option: PlayOptionId,
        count: usize,
    },
}
