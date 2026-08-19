use crate::action::{ManaColor, Target};
use crate::card::{
    BattlefieldEntryScalarChoiceDef, CardTypeSet, ColorChoiceOperationDef, ColorSet, EffectDef,
    ManaCost, ObjectChoiceBindingDef, ObjectPredicateDef, ReplacementEffectDef,
    TopCardSelectionDef, TurnKindDef, ZoneKind, ZonePlacement,
};
use crate::casting::TargetSelection;
use crate::ids::{CardDefinitionId, GameObjectId, ObjectSetBindingIndex, PlayerId};

use super::{
    AbilitySourceRef, ApplicableReplacement, ApplicableZoneMoveReplacement, CardInstance,
    DecisionObservation, DecisionOption, DecisionZone, DrawReplacement, EffectResolutionContext,
    PendingBattlefieldExitBatch, PendingTrigger, PileChosen, PileSplit, PilesSeparated,
    ReplacementEffectContext, ResolvedEffectDurationDef, SacrificedAmountDef, ScopedEffect,
    StackObject, TriggerPlacementBatch,
};

/// Fork repaints its copy, so the copy is red and nothing else.
pub(super) const FORK_COPY_COLOR: ColorSet = ColorSet::from_colors(&[ManaColor::Red]);

/// What runs once a demanded sacrifice has been chosen and made. The
/// sacrificed permanent's power travels as the trigger amount, so an effect
/// measured by what was sacrificed can read it.
#[derive(Clone, Debug)]
pub(super) struct SacrificeFollowup {
    pub(super) object: Box<StackObject>,
    pub(super) context: EffectResolutionContext,
    pub(super) effect: ScopedEffect,
    /// Which characteristic of the sacrificed permanent this reads. Carried
    /// here because the permanent is gone by the time the follow-up runs.
    pub(super) amount: SacrificedAmountDef,
}

/// The branch an optional sacrifice takes when it is declined or has nothing
/// to take. Carried beside the follow-up because both are frozen when the
/// offer is made, and exactly one of them runs.
#[derive(Clone, Debug)]
pub(super) struct SacrificeDeclined {
    pub(super) object: Box<StackObject>,
    pub(super) context: EffectResolutionContext,
    pub(super) effect: ScopedEffect,
}

/// A payment whose dynamic values and payer have been frozen before a
/// resolving effect suspends behind a decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ResolvedEffectPayment {
    Mana(ManaCost),
    Life(u16),
    Mill(u16),
    Discard(u16),
    /// One card matching the predicate, named as part of the payment
    /// decision rather than after it.
    DiscardMatching(ObjectPredicateDef),
    /// Generic mana in an amount the payer chooses, named the same way.
    ChosenGenericMana,
    /// One matching permanent, returned to its owner's hand.
    ReturnPermanentMatching(ObjectPredicateDef),
    /// One matching permanent, sacrificed.
    SacrificePermanentMatching(ObjectPredicateDef),
}

/// What runs once a discard finishes, and what it counts among the cards
/// that went. Held beside the pending discard rather than inside its
/// continuation: one discard effect produces one follow-up, and a decision
/// per player in between.
#[derive(Clone, Debug)]
pub(super) struct DiscardFollowUp {
    pub(super) counted: ObjectPredicateDef,
    pub(super) effect: ScopedEffect,
    pub(super) object: Box<StackObject>,
    pub(super) context: EffectResolutionContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Pregame {
    Mulligan(PlayerId),
    Bottom(PlayerId),
}

#[derive(Clone, Debug)]
pub(super) struct PendingDecision {
    pub(super) observation: DecisionObservation,
    pub(super) continuation: DecisionContinuation,
}

/// One optional replacement that can consume a prospective turn before it
/// begins. The effective ability identity is frozen with its public
/// presentation so copied, granted, and ability-removed sources participate
/// through the same scheduler procedure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ApplicableBeginTurnReplacement {
    pub(super) source: AbilitySourceRef,
    pub(super) controller: PlayerId,
    pub(super) definition: CardDefinitionId,
    pub(super) text: &'static str,
    pub(super) optional: bool,
    pub(super) effect: ReplacementEffectDef,
}

/// An action appended to a skipped prospective turn. CR 614.10b carries it
/// forward until a turn actually begins, when it happens before the turn's
/// ordinary turn-based actions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct DeferredBeginTurnEffect {
    pub(super) replacement: ApplicableBeginTurnReplacement,
    pub(super) effect: EffectDef,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum BalanceAction {
    Sacrifice,
    Discard,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BalancePhase {
    Lands,
    Hands,
    Creatures,
}

impl BalancePhase {
    pub(super) const fn next(self) -> Option<Self> {
        match self {
            Self::Lands => Some(Self::Hands),
            Self::Hands => Some(Self::Creatures),
            Self::Creatures => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ZoneMoveCause {
    Rules,
    Effect { controller: PlayerId },
}

#[derive(Clone, Debug)]
pub(super) struct BalanceTask {
    pub(super) player: PlayerId,
    pub(super) prompt: String,
    pub(super) zone: DecisionZone,
    pub(super) cards: Vec<CardInstance>,
    pub(super) count: usize,
    pub(super) action: BalanceAction,
    pub(super) cause: ZoneMoveCause,
}

/// Where a countered spell ends up.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CounteredSpellZone {
    Graveyard,
    Exile,
}

#[derive(Clone, Debug)]
pub(super) enum DecisionContinuation {
    /// A prospective turn is suspended before any part of it is committed.
    /// When every replacement is optional, option zero begins it. Every other
    /// option applies one replacement; replacing the event asks the scheduler
    /// for the next proposal, while modifying it resumes this same proposal.
    BeginTurn {
        player: PlayerId,
        kind: TurnKindDef,
        applied: Vec<AbilitySourceRef>,
        replacements: Vec<ApplicableBeginTurnReplacement>,
        deferred: Vec<DeferredBeginTurnEffect>,
    },
    /// One of several players choosing cards for an effect before any chosen
    /// card changes zones.
    DiscardForEffect {
        player: PlayerId,
        amount: usize,
        remaining: Vec<PlayerId>,
        chosen: Vec<(PlayerId, Vec<GameObjectId>)>,
        cause: ZoneMoveCause,
    },
    SearchZone {
        controller: PlayerId,
        source: ZoneKind,
        destination: ZoneKind,
        placement: ZonePlacement,
        reveal: bool,
        /// A search shuffles whether or not it found anything. Looking at the
        /// top card does not: the rest of the library was never disturbed.
        shuffle: bool,
        enters_tapped: bool,
    },
    ChooseCards {
        controller: PlayerId,
        destination: ZoneKind,
        placement: ZonePlacement,
        reveal: bool,
    },
    /// The affected player chooses which of several applicable next-draw
    /// replacements consumes this draw. Unchosen replacements remain live.
    DrawReplacement {
        player: PlayerId,
        replacements: Vec<DrawReplacement>,
    },
    BasicLandTypeTextChange {
        target: Target,
    },
    /// A player naming a colour, with everything the answer will be applied
    /// to already settled. The resolving object travels with it for the same
    /// reason a sacrifice follow-up carries one: the effect it produces has
    /// to be attributed to the same source it would have been without the
    /// question in the middle.
    ChooseColor {
        object: Box<StackObject>,
        context: EffectResolutionContext,
        scoped: ScopedEffect,
        targets: Vec<Target>,
        operation: ColorChoiceOperationDef,
        duration: ResolvedEffectDurationDef,
    },
    ChainLightning {
        player: PlayerId,
        spell: StackObject,
        targets: Vec<Target>,
    },
    Fork {
        /// Fork repaints what it copies; a card copying itself does not. The
        /// colours travel with the decision so one continuation serves both.
        colors: Option<ColorSet>,
        /// Copies still to offer after this one, for storm.
        remaining: u16,
        player: PlayerId,
        spell: StackObject,
        target_lists: Vec<Vec<TargetSelection>>,
    },
    RecallDiscard {
        player: PlayerId,
    },
    RecallReturn {
        player: PlayerId,
    },
    /// An effect the controller was offered and may decline.
    OptionalEffect {
        object: Box<StackObject>,
        context: EffectResolutionContext,
        effect: ScopedEffect,
    },
    /// A generic bounded non-targeting object choice. `candidates` is kept
    /// typed because a spell and a permanent are different objects even
    /// though both are addressed by `GameObjectId`.
    ChooseForEffect {
        definition: ScopedEffect,
        binding: ObjectChoiceBindingDef,
        object: Box<StackObject>,
        context: EffectResolutionContext,
        candidates: Vec<Target>,
        effect: ScopedEffect,
    },
    /// A mana payment offered during effect resolution, with either branch
    /// able to continue the same effect program.
    PayOr {
        player: PlayerId,
        payment: ResolvedEffectPayment,
        definition: ScopedEffect,
        object: Box<StackObject>,
        context: EffectResolutionContext,
        if_paid: Option<ScopedEffect>,
        otherwise: Option<ScopedEffect>,
    },
    /// The divider has selected the first pile. The chooser still has to
    /// choose between the two typed groups before the nested effect runs.
    SplitForEffect {
        definition: ScopedEffect,
        chooser: PlayerId,
        items: Vec<Target>,
        object: Box<StackObject>,
        context: EffectResolutionContext,
    },
    /// The divider's two piles, waiting for the chooser to name one.
    ChoosePileForEffect {
        definition: ScopedEffect,
        first: Vec<Target>,
        second: Vec<Target>,
        chosen: ObjectSetBindingIndex,
        unchosen: ObjectSetBindingIndex,
        object: Box<StackObject>,
        context: EffectResolutionContext,
        effect: ScopedEffect,
    },
    /// The card just drawn, offered to its controller to reveal.
    MiracleReveal {
        card: GameObjectId,
    },
    /// A card-owned resolver has separated object-backed options into two
    /// piles. The shared runtime owns choice mechanics; the card owns what a
    /// chosen pile means.
    SeparateIntoPiles {
        resolving_controller: PlayerId,
        subject: PlayerId,
        items: Vec<DecisionOption>,
        on_complete: PilesSeparated,
    },
    ChoosePile {
        piles: PileSplit,
        on_complete: PileChosen,
    },
    /// A sacrifice an effect demanded, chosen by the sacrificing player.
    SacrificeOfChoice {
        followup: Option<SacrificeFollowup>,
        declined: Option<SacrificeDeclined>,
        optional: bool,
    },
    /// Holds the revealed cards while the caster decides which to keep; they
    /// have already left the library, so the continuation must place them all.
    GrislySalvage {
        player: PlayerId,
        revealed: Vec<CardInstance>,
    },
    Balance {
        controller: PlayerId,
        phase: BalancePhase,
        task: BalanceTask,
        remaining: Vec<BalanceTask>,
    },
    SylvanOffer {
        player: PlayerId,
    },
    SylvanSelect {
        player: PlayerId,
        candidates: Vec<GameObjectId>,
        choices_left: usize,
    },
    SylvanMode {
        player: PlayerId,
        card: GameObjectId,
        candidates: Vec<GameObjectId>,
        choices_left: usize,
    },
    /// How many +1/+1 counters Tetravus is trading for Tetravites. Every
    /// option stands for one counter, so the count selected is the answer.
    TetravusDetach {
        source: GameObjectId,
    },
    /// Which of Tetravus's own Tetravites it is exiling to take the counters
    /// back. The options are the tokens themselves.
    TetravusAssemble {
        source: GameObjectId,
    },
    /// Augur of Bolas holding the three cards it looked at; they have already
    /// left the library, so the continuation must place all of them.
    AugurOfBolas {
        player: PlayerId,
        revealed: Vec<CardInstance>,
    },
    /// A generic private top-of-library selection. The cards have already
    /// left the library, so both groups and any deferred follow-up live here.
    TopCardSelection {
        player: PlayerId,
        revealed: Vec<CardInstance>,
        selection: &'static TopCardSelectionDef,
        object: Box<StackObject>,
        context: EffectResolutionContext,
        effect: ScopedEffect,
    },
    /// The affected object's controller chooses which currently applicable
    /// replacement effect to apply next.
    BattlefieldEntryReplacement {
        candidates: Vec<ApplicableReplacement>,
    },
    /// A replacement its controller may decline as the permanent enters. The
    /// exact authored operation is retained so accepting resumes the same
    /// program that was offered; checkpoint import authenticates it against
    /// the source ability before rebuilding this continuation.
    BattlefieldEntryOptional {
        context: ReplacementEffectContext,
        effect: ReplacementEffectDef,
    },
    /// A simultaneous battlefield-exit batch suspended while the affected
    /// object's controller orders two or more applicable replacement effects.
    BattlefieldExitReplacement {
        batch: PendingBattlefieldExitBatch,
        candidates: Vec<ApplicableZoneMoveReplacement>,
    },
    /// A replacement effect suspended while its controller chooses whether to
    /// pay. The prospective event itself remains at the front of the queue.
    BattlefieldEntryPayment {
        context: ReplacementEffectContext,
        player: PlayerId,
        payment: ResolvedEffectPayment,
        definition: ReplacementEffectDef,
    },
    BattlefieldEntryScalarChoice {
        context: ReplacementEffectContext,
        choice: BattlefieldEntryScalarChoiceDef,
        choices: Vec<String>,
    },
    /// The permanents an entering copy effect could imitate, plus the option
    /// of entering as itself.
    BattlefieldEntryCopy {
        choices: Vec<GameObjectId>,
        added_types: CardTypeSet,
    },
    TriggerOrder {
        batch: TriggerPlacementBatch,
        remaining: Vec<TriggerPlacementBatch>,
    },
    TriggerPlacement {
        trigger: PendingTrigger,
        pending: Vec<PendingTrigger>,
        remaining: Vec<TriggerPlacementBatch>,
        candidates: Vec<Target>,
    },
}
