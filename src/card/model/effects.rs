mod likelihood;
mod replacements;
mod triggers;
mod values;

pub use likelihood::*;
pub use replacements::*;
pub use triggers::*;
pub use values::*;

use crate::Format;
use crate::ids::{CardDefinitionId, ObjectBindingIndex, ObjectSetBindingIndex, TargetIndex};

use super::{
    AbilityDef, AddManaEffectDef, BasicLandType, CardTypeSet, ColorSet, CounterKind,
    KeywordAbility, ManaColor, ManaCost, ObjectPredicateDef, PlayActionKind, PlayerRelation,
    TriggerConditionDef, ZoneKind, ZonePlacement,
};

// Effect subjects, lifetimes, and event matchers form the shared vocabulary
// consumed by both resolving and continuously applied effects below.
include!("effects/recipients_and_matchers.rs");
include!("effects/applied.rs");
/// Which characteristic of a sacrificed permanent a follow-up reads.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SacrificedAmountDef {
    Power,
    Toughness,
}

/// A reusable selector for ability-removing continuous effects.
///
/// `Any` supports ordinary "loses all abilities" effects. The keyword form is
/// also the seam needed by text-changing cards that replace one landwalk
/// ability with another without treating the whole rules box as opaque text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AbilityPredicateDef {
    Any,
    Keyword(KeywordAbility),
    /// Every "bands with other" ability, whatever quality it names. Two cards
    /// strip them all at once, and neither says which qualities it means.
    AnyBandsWithOther,
}
/// An event that a replacement ability can modify before it is committed.
///
/// Replacement events deliberately have their own vocabulary rather than
/// reusing [`TriggerEventDef`]: triggers observe events that have already
/// happened, while replacement abilities inspect and modify prospective
/// events.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TurnKindDef {
    /// Match a regular or extra turn.
    Any,
    /// Match only the next turn in the ordinary turn order.
    Regular,
    /// Match only a turn created by a spell or ability.
    Extra,
}

impl TurnKindDef {
    #[must_use]
    pub const fn matches(self, turn: Self) -> bool {
        matches!(
            (self, turn),
            (Self::Any, _) | (Self::Regular, Self::Regular) | (Self::Extra, Self::Extra)
        )
    }
}

/// One place an effect may choose an owned card from.
///
/// Outside the game is deliberately not a [`ZoneKind`]: Magic's zones include
/// exile, while a tournament sideboard remains outside the game until an
/// effect brings one of its cards in.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CardChoiceSourceDef {
    Zone(ZoneKind),
    OutsideGame,
}

/// A reusable condition evaluated in an effect's source and event context.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ConditionDef {
    /// At least one object matches this zone, controller, and object query.
    Exists(ObjectQueryDef),
    /// Every listed condition holds. An empty list is vacuously true, which
    /// is what makes it safe to build one from a card's own list of named
    /// permanents without a special case for the shortest one.
    All(&'static [ConditionDef]),
}

/// How cards are selected for a discard effect.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiscardSelectionDef {
    /// Each affected player chooses cards from their own hand.
    RecipientChooses,
    /// The engine selects cards using the recorded random seed.
    Random,
    /// The same, but only from the cards in hand that match. "Discards a
    /// creature card at random" leaves everything else where it is, and
    /// discards nothing when the hand holds none.
    RandomMatching(&'static ObjectPredicateDef),
}

/// A private look at the top of a library followed by one bounded card
/// selection. Selected and unselected cards can go to different zones; an
/// optional follow-up resumes only after the choice is complete. This covers
/// both selection spells such as Impulse and scry-then-draw sequencing.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TopCardSelectionDef {
    pub count: ValueDef,
    /// Restrict the cards that may be selected while still looking at and
    /// placing every card in the inspected group.
    pub object: Option<ObjectPredicateDef>,
    pub minimum: u8,
    pub maximum: u8,
    /// Reveal selected cards before moving them, for effects that instruct
    /// the player to reveal what they took.
    pub reveal_selected: bool,
    pub selected_zone: ZoneKind,
    pub selected_placement: ZonePlacement,
    pub rest_zone: ZoneKind,
    pub rest_placement: ZonePlacement,
    pub then: Option<&'static EffectDef>,
}

/// Who may observe a pending choice and its available options.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ChoiceVisibilityDef {
    Public,
    Private,
}

/// The context slot populated by an object choice.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ObjectChoiceBindingDef {
    Object(ObjectBindingIndex),
    Objects(ObjectSetBindingIndex),
}

/// Choose a bounded number of non-targeted objects, save them in the resolving
/// context, then continue the effect.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ChooseDef {
    pub binding: ObjectChoiceBindingDef,
    pub chooser: PlayerRefDef,
    pub candidates: ObjectSetDef,
    pub exclude: Option<ObjectRefDef>,
    pub minimum: usize,
    pub maximum: usize,
    pub visibility: ChoiceVisibilityDef,
    pub then: &'static EffectDef,
}

/// The supported cost of an optional effect payment.
///
/// This is deliberately narrower than casting and activation costs: those
/// procedures can plan compound costs atomically, while a resolving effect
/// currently offers exactly one mana or life payment.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EffectPaymentCostDef {
    Mana(ManaCost),
    /// A generic mana payment whose amount is evaluated at resolution.
    GenericMana(ValueDef),
    Life(u16),
    /// Mill this many cards. Never impossible: a library shorter than the
    /// amount mills what it has (CR 701.13b), so this branch of an "unless"
    /// is always open and the choice is a real one even at one card left.
    Mill(u16),
}

/// A payment offered while an effect or replacement procedure resolves.
///
/// The payer uses the same compositional player-set vocabulary as the rest of
/// the effect model. Payment procedures require that it resolve to exactly one
/// player; a missing or non-singleton payer cannot pay and takes the declined
/// branch.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EffectPaymentDef {
    pub payer: PlayerSetDef,
    pub cost: EffectPaymentCostDef,
}

impl EffectPaymentDef {
    #[must_use]
    pub const fn mana(payer: PlayerSetDef, cost: ManaCost) -> Self {
        Self {
            payer,
            cost: EffectPaymentCostDef::Mana(cost),
        }
    }

    #[must_use]
    pub const fn generic_mana(payer: PlayerSetDef, amount: ValueDef) -> Self {
        Self {
            payer,
            cost: EffectPaymentCostDef::GenericMana(amount),
        }
    }

    #[must_use]
    pub const fn life(payer: PlayerSetDef, amount: u16) -> Self {
        Self {
            payer,
            cost: EffectPaymentCostDef::Life(amount),
        }
    }

    #[must_use]
    pub const fn mill(payer: PlayerSetDef, amount: u16) -> Self {
        Self {
            payer,
            cost: EffectPaymentCostDef::Mill(amount),
        }
    }
}

/// Offer a payment and continue through the branch selected by its result.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PayOrDef {
    pub payment: EffectPaymentDef,
    pub if_paid: Option<&'static EffectDef>,
    pub otherwise: Option<&'static EffectDef>,
    pub visibility: ChoiceVisibilityDef,
}

impl PayOrDef {
    /// Offer an optional payment and continue only when it is paid.
    #[must_use]
    pub const fn optional(payment: EffectPaymentDef, if_paid: &'static EffectDef) -> Self {
        Self {
            payment,
            if_paid: Some(if_paid),
            otherwise: None,
            visibility: ChoiceVisibilityDef::Private,
        }
    }

    /// Continue unless the resolving effect's controller pays a fixed mana
    /// cost.
    #[must_use]
    pub const fn unless_mana(cost: ManaCost, otherwise: &'static EffectDef) -> Self {
        Self {
            payment: EffectPaymentDef::mana(
                PlayerSetDef::One(PlayerRefDef::EffectController),
                cost,
            ),
            if_paid: None,
            otherwise: Some(otherwise),
            visibility: ChoiceVisibilityDef::Private,
        }
    }
}

/// The objects divided by a pile-splitting procedure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PartitionItemsDef {
    Objects(ObjectSetDef),
    TopOfLibrary {
        player: PlayerRefDef,
        count: ValueDef,
    },
}

/// Divide objects into two piles, choose one pile, bind both results, and then
/// continue the effect.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SplitIntoPilesDef {
    pub items: PartitionItemsDef,
    pub divider: PlayerSetDef,
    pub chooser: PlayerSetDef,
    pub chosen: ObjectSetBindingIndex,
    pub unchosen: ObjectSetBindingIndex,
    pub then: &'static EffectDef,
}

/// How long an effect-created triggered ability listens from outside every
/// zone.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InstalledTriggerLifetimeDef {
    Once,
    UntilNextTurn(PlayerRefDef),
}

/// A triggered ability installed by a resolving effect.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct InstalledTriggerDef {
    pub ability: &'static AbilityDef,
    pub lifetime: InstalledTriggerLifetimeDef,
}

impl InstalledTriggerDef {
    #[must_use]
    pub const fn once(ability: &'static AbilityDef) -> Self {
        Self {
            ability,
            lifetime: InstalledTriggerLifetimeDef::Once,
        }
    }

    #[must_use]
    pub const fn until_next_turn(ability: &'static AbilityDef, player: PlayerRefDef) -> Self {
        Self {
            ability,
            lifetime: InstalledTriggerLifetimeDef::UntilNextTurn(player),
        }
    }
}

/// Declarative effect primitives interpreted by the rules engine.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EffectDef {
    AddCounters {
        object: EffectRecipientDef,
        kind: CounterKind,
        amount: ValueDef,
    },
    AddMana(AddManaEffectDef),
    /// Adds mana of one colour, however much a value says. Mana abilities use
    /// [`Self::AddMana`] with a fixed amount so the mana planner can read
    /// them without resolving anything; this is for the effects that cannot
    /// know their amount until they resolve.
    AddManaEqualTo {
        color: ManaColor,
        amount: ValueDef,
    },
    /// Poison counters given to a player. Ten of them is a state-based loss,
    /// which is why this is not expressible as life loss.
    AddPoisonCounters {
        recipient: EffectRecipientDef,
        amount: ValueDef,
    },
    Apply {
        recipient: EffectRecipientDef,
        effect: AppliedEffectDef,
        duration: ResolvedEffectDurationDef,
    },
    /// An Aura spell attaching itself to what it enchants. The permanent the
    /// spell becomes is what attaches, so this is only meaningful on the spell
    /// clause of an Aura.
    Attach {
        object: EffectRecipientDef,
    },
    /// Reconfigure's paired attach/unattach procedure. A selected creature
    /// becomes the new host; selecting none ends this attachment incarnation.
    Reconfigure {
        object: EffectRecipientDef,
    },
    /// Replaces the source permanent's copiable values with the target's.
    /// Some copy effects, such as Thespian's Stage, retain the resolving
    /// ability as an exception to the copied values.
    BecomeCopyOf {
        object: EffectRecipientDef,
        retain_source_ability: bool,
    },
    /// A static attack restriction: this creature cannot be declared as an
    /// attacker unless the query matches. The query carries its own controller
    /// relation, so "unless defending player controls an Island" is an
    /// opponent-relative battlefield query rather than a special case.
    CannotAttackUnless(&'static ObjectQueryDef),
    /// The mirror of [`Self::CannotAttackUnless`]: the source cannot attack
    /// while anything matches. The negation is over the existential rather
    /// than the object, which is why it is its own clause rather than a
    /// negated query.
    CannotAttackIf(&'static ObjectQueryDef),
    /// A static prohibition: no spell or ability an opponent controls can
    /// make this ability's controller sacrifice a permanent.
    CannotBeForcedToSacrifice,
    /// On resolution, choose two different basic land-type words and apply
    /// the resulting indefinite, noncopiable text change to the object.
    ChangeTextBasicLandType {
        object: EffectRecipientDef,
    },
    Choose(ChooseDef),
    /// Choose owned cards from one or more places without performing the
    /// keyword action "search." Ring of Ma'rûf uses this for outside-game
    /// cards, and Old School expands the same choice to exile.
    ChooseCards {
        player: EffectRecipientDef,
        sources: &'static [CardChoiceSourceDef],
        object: ObjectPredicateDef,
        minimum: usize,
        maximum: usize,
        reveal: bool,
        destination: ZoneKind,
        placement: ZonePlacement,
    },
    /// Counter a spell and put its card into `zone`. Ordinary counters use
    /// the graveyard; replacement-style counters such as Dissipate use exile.
    Counter {
        object: EffectRecipientDef,
        zone: ZoneKind,
    },
    /// Gives its controller an emblem, an object that sits outside every
    /// zone and does nothing but carry its abilities.
    CreateEmblem {
        emblem: CardDefinitionId,
    },
    /// Puts token copies of `token` onto the battlefield under the resolving
    /// object's controller.
    CreateToken {
        token: CardDefinitionId,
        count: ValueDef,
        /// Whether the created token arrives tapped.
        tapped: bool,
    },
    /// Creates one token, then attaches the resolving permanent to that
    /// exact battlefield incarnation before state-based actions run. This is
    /// the common living-weapon shape; the delayed entry completion keeps it
    /// correct through replacement effects and zone-change identity.
    CreateAttachedToken {
        token: CardDefinitionId,
    },
    /// Creates a token copying the recipient's copiable values. Populate uses
    /// this after its generic choice has selected a creature token.
    CreateTokenCopyOf {
        object: EffectRecipientDef,
    },
    DealDamage {
        recipient: EffectRecipientDef,
        amount: ValueDef,
    },
    Destroy {
        object: EffectRecipientDef,
        can_regenerate: bool,
    },
    /// The object is destroyed when this combat phase ends.
    DestroyAtEndOfCombat {
        object: EffectRecipientDef,
    },
    /// Until the resolving controller's next turn, the permanent cannot
    /// attack, block, or activate its activated abilities.
    Detain {
        object: EffectRecipientDef,
    },
    /// Each recipient discards that many cards selected in the specified way.
    /// A player holding fewer cards discards their whole hand.
    Discard {
        recipient: EffectRecipientDef,
        amount: ValueDef,
        selection: DiscardSelectionDef,
    },
    /// Discard the named card objects from their owners' hands. Selection is
    /// expressed separately (usually with [`Self::Choose`]); this leaf is the
    /// rules action that moves the chosen cards and emits discard events.
    DiscardCards {
        object: EffectRecipientDef,
    },
    /// Deals damage and gains its controller that much life, but no more
    /// than the recipient had to give: a player's life total, a
    /// planeswalker's loyalty, or a creature's toughness, each read before
    /// the damage. Draining an almost-dead target gains only what was there.
    DrainLife {
        recipient: EffectRecipientDef,
        amount: ValueDef,
    },
    DrawCards {
        recipient: EffectRecipientDef,
        amount: ValueDef,
    },
    /// The affected player loses all unspent mana without invoking the
    /// turn-based mana-pool emptying procedure (and therefore without mana
    /// burn in formats that use it).
    EmptyManaPool {
        player: EffectRecipientDef,
    },
    /// Exiles, remembering which object sent it there so a later clause can
    /// bring it back. This is the Oblivion Ring shape.
    ExileLinkedToSource {
        object: EffectRecipientDef,
    },
    /// Gain control of the recipient for the stated duration. Source-bound
    /// durations also remember whether the source must remain tapped.
    GainControl {
        object: EffectRecipientDef,
        duration: ControlDurationDef,
    },
    GainLife {
        recipient: EffectRecipientDef,
        amount: ValueDef,
    },
    /// Lets the next sorcery its controller casts this turn be cast as
    /// though it had flash.
    GrantFlashToNextSorcery,
    /// Runs `then` only if the condition holds where this effect resolves.
    /// A condition on a triggered ability is an intervening-if and is checked
    /// twice; this one is part of the effect and is checked once.
    IfCondition {
        condition: &'static TriggerConditionDef,
        then: &'static EffectDef,
    },
    /// Resolve one branch under a particular per-game format profile. Card
    /// definitions remain format-neutral; only the rules procedure varies.
    IfFormat {
        format: Format,
        then: &'static EffectDef,
        otherwise: &'static EffectDef,
    },
    /// Installs a triggered ability that listens from outside every zone.
    InstallTrigger(InstalledTriggerDef),
    /// A static effect that turns off one landwalk for blocking purposes:
    /// creatures with it can be blocked as though they did not have it. The
    /// keyword is untouched -- anything else reading it still sees it -- so
    /// this is a blocking rule rather than an ability-removing one.
    LandwalkCanBeBlocked(BasicLandType),
    /// One player looks at another's hand. Nothing changes zones and no
    /// decision follows; the looking player simply knows.
    LookAtHand {
        player: EffectRecipientDef,
    },
    /// Look privately at the top cards of a library, choose a bounded subset,
    /// place both groups, optionally reveal the selected cards, then continue
    /// resolving. A predicate restricts what may be selected without hiding
    /// the rest of the inspected group.
    LookAtTopAndSelect {
        /// Whose library is inspected.
        player: EffectRecipientDef,
        /// Who looks at the cards and makes the choice. Digging through your
        /// own library names the same player twice, which is the ordinary
        /// case; a spy names someone else's library and keeps the looking.
        looker: EffectRecipientDef,
        selection: &'static TopCardSelectionDef,
    },
    LoseLife {
        recipient: EffectRecipientDef,
        amount: ValueDef,
    },
    /// A state-based loss with no life total involved (CR 104.3b). Nothing
    /// can be done about it once the effect resolves.
    LoseTheGame {
        player: EffectRecipientDef,
    },
    /// An effect the named player may decline. Held by reference so that
    /// `EffectDef` does not grow a recursive inline copy of itself.
    May {
        player: EffectRecipientDef,
        effect: &'static EffectDef,
    },
    /// Put that many cards from the top of a library into its owner's
    /// graveyard.
    Mill {
        player: EffectRecipientDef,
        amount: ValueDef,
    },
    MoveToZone {
        object: EffectRecipientDef,
        zone: ZoneKind,
        /// Which end of a library the card lands on. Meaningless for every
        /// other destination.
        placement: ZonePlacement,
        /// Who controls the permanent when the destination is the
        /// battlefield. `None` is the ordinary case, where a card arrives
        /// under its owner's control; reanimation that steals names a
        /// relation instead.
        controller: Option<PlayerRelation>,
    },
    /// Every card in the named player's hand is revealed to everyone.
    ///
    /// Nothing moves; what changes is what the table knows. It is a separate
    /// step from whatever reads the hand afterwards, because the reveal
    /// happens even when the clause that follows finds nothing to do -- and
    /// unlike [`Self::LookAtHand`], one player learning is not enough.
    RevealHand {
        player: EffectRecipientDef,
    },
    /// CR 506.4: the permanent stops attacking or blocking, and anything
    /// blocking it stops. An attacker removed this way was still blocked, so
    /// it deals no damage rather than getting through.
    RemoveFromCombat {
        object: EffectRecipientDef,
    },
    None,
    PayOr(PayOrDef),
    /// Install a resolved damage-prevention rule for the named duration.
    PreventDamage {
        prevention: DamagePreventionDef,
        duration: ResolvedEffectDurationDef,
    },
    /// Select one branch using the game's replay-stable seeded RNG.
    Randomized {
        likelihood: LikelihoodDef,
        on_success: &'static EffectDef,
        on_failure: &'static EffectDef,
    },
    /// This card costs that much less generic mana to cast. A static ability
    /// that works from the hand, where casting reads it.
    ReduceGenericCostBy(ValueDef),
    /// Matching spells cost that much less generic mana to cast.
    ///
    /// The difference from [`Self::ReduceGenericCostBy`] is who is
    /// discounted: that one is a card in hand cutting its own cost and needs
    /// to name nothing, while this is read off a permanent and so has to say
    /// which spells, and cast by whom.
    ReduceMatchingSpellCostBy {
        spell: ObjectPredicateDef,
        caster: PlayerRelation,
        amount: ValueDef,
    },
    /// Creates a regeneration shield (CR 701.15). The shield is not the
    /// regeneration: it waits, and the next destruction this turn is replaced
    /// by tapping the permanent, removing it from combat, and removing all
    /// damage from it. Shields that go unused are cleared in cleanup, so a
    /// creature that was never destroyed keeps nothing.
    Regenerate {
        object: EffectRecipientDef,
    },
    /// Removes every counter of one kind from the recipient.
    RemoveAllCounters {
        object: EffectRecipientDef,
        kind: CounterKind,
    },
    /// Removes that many counters of one kind, or as many as are there. The
    /// mirror of [`Self::AddCounters`], for the clauses that take some back
    /// rather than all of them.
    RemoveCounters {
        object: EffectRecipientDef,
        kind: CounterKind,
        amount: ValueDef,
    },
    /// Replace the named player's next draw this turn with another effect.
    /// The replacement is frozen with the resolving object and consumed even
    /// when its instructions cannot move a card.
    ReplaceNextDrawThisTurn {
        player: EffectRecipientDef,
        effect: &'static EffectDef,
    },
    /// Returns everything this ability's source exiled, to the named zone.
    /// A returned permanent keeps `grant` until end of turn, which is how
    /// Obzedat comes back ready to attack.
    ReturnLinkedExiles {
        zone: ZoneKind,
        grant: Option<KeywordAbility>,
    },
    Sacrifice {
        object: EffectRecipientDef,
    },
    /// Each recipient player chooses one permanent they control that matches,
    /// and sacrifices it. This remains a dedicated simultaneous procedure:
    /// every affected player's APNAP-ordered choice is frozen before any
    /// permanent moves, forced-sacrifice prohibitions are applied, and an
    /// optional follow-up can read the sacrificed permanent's last-known
    /// power. A generic [`Self::Choose`] followed by [`Self::Sacrifice`] cannot
    /// preserve those multiplayer and LKI semantics.
    SacrificeOfChoice {
        player: EffectRecipientDef,
        object: ObjectPredicateDef,
        /// Run after the sacrifice, with the characteristic named by
        /// `amount` as [`ValueDef::TriggerEventAmount`]. A sacrifice of
        /// choice waits on a decision, so anything reading what was
        /// sacrificed has to be part of the same continuation rather than the
        /// next effect in sequence.
        then: Option<&'static EffectDef>,
        /// Which of the sacrificed permanent's characteristics the follow-up
        /// reads. Both are last-known by the time it runs, so neither is
        /// harder to reach than the other -- the card simply has to say.
        amount: SacrificedAmountDef,
        /// Whether the player may decline. An optional sacrifice runs `then`
        /// only when something was actually sacrificed, which is what "if a
        /// player does" means; a compulsory one runs it either way, so an
        /// amount read off nothing is zero rather than skipped.
        optional: bool,
    },
    /// Schedules these additional phases after the current phase. Later
    /// schedules at the same boundary happen before earlier ones, while the
    /// order inside one schedule is preserved.
    ScheduleTurnPhases(&'static [TurnPhaseDef]),
    /// Search one player's card zone for matching cards and move the chosen
    /// cards. `minimum` and `maximum` model the stated quantity independently
    /// from whether the predicate describes a quality: a search for simply
    /// "a card" is compulsory when one exists, while a qualified hidden-zone
    /// search may legally fail to find and therefore uses a minimum of zero.
    SearchZone {
        player: EffectRecipientDef,
        source: ZoneKind,
        object: ObjectPredicateDef,
        minimum: usize,
        maximum: usize,
        reveal: bool,
        destination: ZoneKind,
        placement: ZonePlacement,
        shuffle: bool,
        /// Whether a permanent this search puts onto the battlefield arrives
        /// tapped.
        enters_tapped: bool,
    },
    Sequence(&'static [EffectDef]),
    /// Randomizes each recipient player's library. Effects that shuffle
    /// cards from other zones into a library first express those zone moves
    /// with [`Self::MoveToZone`], then use this shared operation.
    ShuffleLibrary {
        player: EffectRecipientDef,
    },
    /// The object sits out this many of its controller's untap steps.
    SkipNextUntapSteps {
        object: EffectRecipientDef,
        count: u8,
    },
    /// A descriptive marker for an effect portion the shared vocabulary does
    /// not yet represent. The surrounding costs, targets, and timing can still
    /// remain declarative; clause coverage records whether and how it executes.
    Special(&'static str),
    SplitIntoPiles(SplitIntoPilesDef),
    /// A continuous or rules-modifying effect derived live from a static
    /// ability. Its lifetime is the ability's own applicability rather than a
    /// stored duration.
    StaticApply {
        recipient: EffectRecipientDef,
        effect: AppliedEffectDef,
    },
    /// Gives each affected player an extra turn after the current one. Extra
    /// turns are queued by the turn engine, so a later-created turn happens
    /// before an earlier-created one.
    TakeExtraTurn {
        player: EffectRecipientDef,
    },
    Tap {
        object: EffectRecipientDef,
    },
    /// Turns a double-faced permanent over to its other face.
    Transform {
        object: EffectRecipientDef,
    },
    Untap {
        object: EffectRecipientDef,
    },
}

impl EffectDef {
    #[must_use]
    pub const fn counter_target(target: TargetIndex) -> Self {
        Self::Counter {
            object: EffectRecipientDef::Target(target),
            zone: ZoneKind::Graveyard,
        }
    }

    #[must_use]
    pub const fn destroy_target(target: TargetIndex, can_regenerate: bool) -> Self {
        Self::Destroy {
            object: EffectRecipientDef::Target(target),
            can_regenerate,
        }
    }
}

/// A major turn phase that a resolving effect can insert.
///
/// This is intentionally narrower than [`TurnStepDef`]. Steps remain trigger
/// labels inside a phase, and the untap procedure remains part of ordinary
/// turn startup rather than an independently scheduled step.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TurnPhaseDef {
    Combat,
    PostcombatMain,
}

/// Observable turn steps used by beginning/end-of-step trigger declarations.
/// Untap is an engine procedure before upkeep, and cleanup has no ordinary
/// priority window, so neither is an authored trigger label.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TurnStepDef {
    Upkeep,
    Draw,
    PrecombatMain,
    BeginningOfCombat,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    EndOfCombat,
    PostcombatMain,
    End,
}
