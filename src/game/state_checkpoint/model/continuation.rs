//! The pending question a checkpoint was taken in the middle of.
//!
//! A decision is the one place a resolving effect stops with work still to
//! do, so every continuation has to say enough for the rest of that work to
//! be found again in the catalog rather than carried as executable state.

use serde::{Deserialize, Serialize};

use super::{
    AbilityLocator, AbilitySourceSnapshot, ApplicableBeginTurnReplacementSnapshot,
    ApplicableReplacementSnapshot, BalancePhaseSnapshot, BalanceTaskSnapshot,
    DecisionOptionSnapshot, DeferredBeginTurnEffectSnapshot, DetachedCardSnapshot,
    DetachedStackSnapshot, DiscardChoiceSnapshot, DrawReplacementSnapshot,
    EffectContinuationSnapshot, EffectResolutionContextSnapshot, PendingTriggerSnapshot,
    PileSplitSnapshot, ReplacementEffectContextSnapshot, ReplacementEffectLocator,
    ResolvedEffectPaymentSnapshot, ScopedEffectSnapshot, TargetSelectionSnapshot, TargetSnapshot,
    TriggerPlacementBatchSnapshot, TurnKindSnapshot, ZoneKindSnapshot, ZoneMoveCauseSnapshot,
    ZonePlacementSnapshot,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(in crate::game::state_checkpoint) enum DecisionContinuationSnapshot {
    BeginTurn {
        player: usize,
        turn_kind: TurnKindSnapshot,
        applied: Vec<AbilitySourceSnapshot>,
        replacements: Vec<ApplicableBeginTurnReplacementSnapshot>,
        deferred: Vec<DeferredBeginTurnEffectSnapshot>,
    },
    SearchZone {
        controller: usize,
        source: ZoneKindSnapshot,
        destination: ZoneKindSnapshot,
        placement: ZonePlacementSnapshot,
        reveal: bool,
        shuffle: bool,
        /// Additive: a checkpoint written before fetch lands existed carries
        /// no flag and reconstructs as an untapped arrival.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        enters_tapped: bool,
    },
    ChooseCards {
        controller: usize,
        destination: ZoneKindSnapshot,
        placement: ZonePlacementSnapshot,
        reveal: bool,
    },
    DrawReplacement {
        player: usize,
        replacements: Vec<DrawReplacementSnapshot>,
    },
    BasicLandTypeTextChange {
        target: TargetSnapshot,
    },
    DiscardForEffect {
        player: usize,
        amount: usize,
        remaining: Vec<usize>,
        chosen: Vec<DiscardChoiceSnapshot>,
        cause: ZoneMoveCauseSnapshot,
    },
    GrislySalvage {
        player: usize,
        revealed: Vec<DetachedCardSnapshot>,
    },
    CardNameChoice {
        /// The names on offer. A card name is stable catalog data, so the
        /// list is written down rather than recomputed: which names were
        /// offered is part of the pending question.
        choices: Vec<String>,
        searched: usize,
        zone: ZoneKindSnapshot,
        binding: usize,
        continuation: EffectContinuationSnapshot,
    },
    AugurOfBolas {
        player: usize,
        revealed: Vec<DetachedCardSnapshot>,
    },
    TopCardSelection {
        player: usize,
        revealed: Vec<DetachedCardSnapshot>,
        continuation: EffectContinuationSnapshot,
    },
    ChainLightning {
        player: usize,
        spell: DetachedStackSnapshot,
        targets: Vec<TargetSnapshot>,
    },
    Fork {
        player: usize,
        spell: DetachedStackSnapshot,
        target_lists: Vec<Vec<TargetSelectionSnapshot>>,
        /// Absent for a card copying itself, which keeps its own colours.
        #[serde(default)]
        repainted: bool,
        /// Copies still to offer after this one, for storm.
        #[serde(default)]
        remaining: u16,
    },
    OptionalEffect {
        object: DetachedStackSnapshot,
        ability: AbilityLocator,
        context: EffectResolutionContextSnapshot,
        effect: ScopedEffectSnapshot,
    },
    ChooseForEffect {
        continuation: EffectContinuationSnapshot,
    },
    PayOr {
        player: usize,
        payment: ResolvedEffectPaymentSnapshot,
        object: DetachedStackSnapshot,
        ability: AbilityLocator,
        context: EffectResolutionContextSnapshot,
        definition: ScopedEffectSnapshot,
    },
    SplitForEffect {
        continuation: EffectContinuationSnapshot,
    },
    ChoosePileForEffect {
        first: Vec<TargetSnapshot>,
        second: Vec<TargetSnapshot>,
        continuation: EffectContinuationSnapshot,
    },
    BattlefieldEntryPayment {
        context: ReplacementEffectContextSnapshot,
        player: usize,
        payment: ResolvedEffectPaymentSnapshot,
        effect: ReplacementEffectLocator,
    },
    BattlefieldEntryReplacement {
        candidates: Vec<ApplicableReplacementSnapshot>,
    },
    BattlefieldEntryOptional {
        context: ReplacementEffectContextSnapshot,
        effect: ReplacementEffectLocator,
    },
    BattlefieldEntryScalarChoice {
        context: ReplacementEffectContextSnapshot,
        effect: ReplacementEffectLocator,
        choices: Vec<String>,
    },
    BattlefieldEntryCopy {
        choices: Vec<u32>,
        added_types: [bool; crate::card::CardType::COUNT],
    },
    TriggerOrder {
        batch: TriggerPlacementBatchSnapshot,
        remaining: Vec<TriggerPlacementBatchSnapshot>,
    },
    TriggerPlacement {
        trigger: PendingTriggerSnapshot,
        pending: Vec<PendingTriggerSnapshot>,
        remaining: Vec<TriggerPlacementBatchSnapshot>,
        candidates: Vec<TargetSnapshot>,
    },
    MiracleReveal {
        card: u32,
    },
    SeparateIntoPiles {
        resolving_controller: usize,
        subject: usize,
        items: Vec<DecisionOptionSnapshot>,
        on_complete: String,
    },
    ChoosePile {
        piles: PileSplitSnapshot,
        on_complete: String,
    },
    SacrificeOfChoice {
        followup: Option<Box<EffectContinuationSnapshot>>,
        /// The branch a declined offer takes. Appended after the follow-up,
        /// so a checkpoint written before this existed still reads. Boxed
        /// alongside it to keep this variant off the enum's size.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        declined: Option<Box<EffectContinuationSnapshot>>,
        optional: bool,
    },
    /// A colour choice waiting to be answered. Only the recipients are
    /// stored: what to do with the answer and how long it lasts are read
    /// back off the effect the continuation already locates.
    ChooseColor {
        continuation: Box<EffectContinuationSnapshot>,
        targets: Vec<TargetSnapshot>,
    },
    RecallDiscard {
        player: usize,
    },
    RecallReturn {
        player: usize,
    },
    Balance {
        controller: usize,
        phase: BalancePhaseSnapshot,
        task: BalanceTaskSnapshot,
        remaining: Vec<BalanceTaskSnapshot>,
    },
    SylvanOffer {
        player: usize,
    },
    SylvanSelect {
        player: usize,
        candidates: Vec<u32>,
        choices_left: usize,
    },
    SylvanMode {
        player: usize,
        card: u32,
        candidates: Vec<u32>,
        choices_left: usize,
    },
    TetravusDetach {
        source: u32,
    },
    TetravusAssemble {
        source: u32,
    },
}
