use crate::ids::{AbilityId, AlternativeCostId, ModeId, TargetIndex};

use super::{
    AbilityCostDef, AbilityCostList, AbilityDef, AbilityTargetDef, AlternativeCostDef,
    BasicLandType, CardBehavior, CardSupertype, CardType, CounterKind, EffectDef,
    ImplementationStatus, ManaColor, ManaCost, ObjectPredicateDef, ObjectQueryDef, PlayerRelation,
    ReplacementConditionDef, ReplacementEffectDef, ReplacementEventDef, TriggerEventDef, ZoneKind,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SpellAbilityDef {
    Nonmodal {
        targets: &'static [AbilityTargetDef],
        /// A nonmana cost paid as the spell is cast, chosen from the objects
        /// it names. Unlike a target this is spent rather than pointed at, so
        /// it is not checked again on resolution.
        additional_cost: Option<SpellAdditionalCostDef>,
        /// Where the card goes after a successful resolution. This is part of
        /// a spell's shared stack procedure rather than an instruction that
        /// can move the resolving object while it is off the stack.
        resolution_destination: SpellResolutionDestinationDef,
    },
    Modal(ModalSpellDef),
}

/// The card's normal post-resolution destination after it has successfully
/// completed its instructions. Countered spells never use this: they follow
/// the countering effect's destination instead. A destination can also carry
/// an instruction that remains meaningful when the spell is a copy, such as
/// shuffling its owner's library.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SpellResolutionDestinationDef {
    Graveyard,
    Exile,
    /// Exile the card and put these counters on its new object. A zone change
    /// happens before the counters are added, so prior-zone counters cannot
    /// leak into exile.
    ExileWithCounters(&'static [(CounterKind, u16)]),
    /// Move the card to its owner's library, then shuffle it. The shuffle is
    /// still part of the resolution when another effect replaces the move, or
    /// when this resolving spell is a copy with no card to move.
    LibraryShuffled,
}

/// An additional cost that selects objects to spend. The zone decides what
/// spending means: a permanent on the battlefield is sacrificed, a card in a
/// graveyard is exiled, and a card in hand is discarded.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SpellAdditionalCostDef {
    pub object: ObjectPredicateDef,
    pub zone: ZoneKind,
    pub count: u8,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ModalSpellDef {
    /// Each mode is an ordinary spell ability. Its positional index supplies
    /// the stable [`ModeId`] used by casting and presentation.
    pub modes: &'static [AbilityDef],
    pub minimum: u8,
    pub maximum: u8,
    /// Some spells explicitly allow the same mode to be chosen more than once.
    pub may_repeat: bool,
}

impl ModalSpellDef {
    #[must_use]
    pub const fn new(
        modes: &'static [AbilityDef],
        minimum: u8,
        maximum: u8,
        may_repeat: bool,
    ) -> Self {
        Self {
            modes,
            minimum,
            maximum,
            may_repeat,
        }
    }

    #[must_use]
    pub const fn choose_one(modes: &'static [AbilityDef]) -> Self {
        Self::new(modes, 1, 1, false)
    }
}

impl SpellAbilityDef {
    #[must_use]
    pub const fn new() -> Self {
        Self::Nonmodal {
            targets: &[],
            additional_cost: None,
            resolution_destination: SpellResolutionDestinationDef::Graveyard,
        }
    }

    /// Adds targets to an ordinary, nonmodal spell definition.
    ///
    /// # Panics
    ///
    /// Panics for a modal wrapper because each mode declares its own targets.
    #[must_use]
    pub const fn with_targets(self, targets: &'static [AbilityTargetDef]) -> Self {
        match self {
            Self::Nonmodal {
                additional_cost, ..
            } => Self::Nonmodal {
                targets,
                additional_cost,
                resolution_destination: self.resolution_destination(),
            },
            Self::Modal(_) => panic!("targets belong on modal spell branches"),
        }
    }

    /// # Panics
    ///
    /// Panics for a modal wrapper, which has no single cost to attach.
    #[must_use]
    pub const fn with_additional_cost(self, cost: SpellAdditionalCostDef) -> Self {
        match self {
            Self::Nonmodal {
                targets,
                resolution_destination,
                ..
            } => Self::Nonmodal {
                targets,
                additional_cost: Some(cost),
                resolution_destination,
            },
            Self::Modal(_) => panic!("an additional cost belongs to a whole spell"),
        }
    }

    #[must_use]
    pub const fn additional_cost(self) -> Option<SpellAdditionalCostDef> {
        match self {
            Self::Nonmodal {
                additional_cost, ..
            } => additional_cost,
            Self::Modal(_) => None,
        }
    }

    /// Changes the ordinary destination used after this spell resolves. Modal
    /// wrappers share one spell object and therefore one destination.
    #[must_use]
    pub const fn with_resolution_destination(
        self,
        destination: SpellResolutionDestinationDef,
    ) -> Self {
        match self {
            Self::Nonmodal {
                targets,
                additional_cost,
                ..
            } => Self::Nonmodal {
                targets,
                additional_cost,
                resolution_destination: destination,
            },
            Self::Modal(modal) => Self::Modal(modal),
        }
    }

    #[must_use]
    pub const fn resolution_destination(self) -> SpellResolutionDestinationDef {
        match self {
            Self::Nonmodal {
                resolution_destination,
                ..
            } => resolution_destination,
            Self::Modal(_) => SpellResolutionDestinationDef::Graveyard,
        }
    }

    #[must_use]
    pub const fn modal_spell(
        modes: &'static [AbilityDef],
        minimum: u8,
        maximum: u8,
        may_repeat: bool,
    ) -> Self {
        Self::Modal(ModalSpellDef::new(modes, minimum, maximum, may_repeat))
    }

    /// Returns targets declared directly by a nonmodal spell. Modal wrappers
    /// have no direct targets; selected branches supply them instead.
    #[must_use]
    pub const fn targets(self) -> &'static [AbilityTargetDef] {
        match self {
            Self::Nonmodal { targets, .. } => targets,
            Self::Modal(_) => &[],
        }
    }

    #[must_use]
    pub const fn modal(self) -> Option<ModalSpellDef> {
        match self {
            Self::Nonmodal { .. } => None,
            Self::Modal(modal) => Some(modal),
        }
    }

    #[must_use]
    pub fn mode(self, id: ModeId) -> Option<&'static AbilityDef> {
        self.modal()?.modes.get(id.index())
    }
}

impl Default for SpellAbilityDef {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AbilityProcedureDef {
    /// Costs, action generation, trigger capture, and stack handling use the
    /// shared rules procedures for this ability category.
    Shared,
    /// Transitional compatibility path for an ability whose category is
    /// known but whose surrounding rules procedure still lives in legacy
    /// card behavior.
    Legacy,
}

/// When a printed "Activate only ..." clause allows an ability to be
/// activated. This restricts the window; it does not change priority, so an
/// ability that is also sorcery-speed still needs an empty stack.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ActivationTimingDef {
    /// Any time its controller has priority, which is the printed default.
    #[default]
    Any,
    /// Only during a turn its controller is taking.
    YourTurn,
    /// Only during the upkeep step of a turn its controller is taking.
    YourUpkeep,
    /// Only during an upkeep step, whoever is taking the turn. Tolaria opens
    /// on both, which is what makes it an answer to an attack.
    AnyUpkeep,
    /// Only during the combat phase, whoever is taking the turn. Every step
    /// from the beginning of combat through the end of combat, which is what
    /// lets an animated artifact block as well as attack.
    DuringCombat,
    /// Only during the end-of-combat step. Combat is over and the damage is
    /// dealt, so a land shooting an attacker here is finishing off something
    /// that survived rather than stopping it.
    EndOfCombat,
    /// Only when its controller could cast a sorcery: their own main phase,
    /// with the stack empty. Unlike the windows above, this one does depend
    /// on the stack, because that is what "as a sorcery" means.
    SorcerySpeed,
    /// "Activate only before the combat damage step." The window is open all
    /// turn until damage is about to be dealt, on either player's turn, which
    /// is what makes the ability something the attacker can be surprised by.
    BeforeCombatDamage,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ActivatedAbilityDef {
    pub source_zones: &'static [ZoneKind],
    pub costs: AbilityCostList,
    pub targets: &'static [AbilityTargetDef],
    pub procedure: AbilityProcedureDef,
    pub timing: ActivationTimingDef,
    /// How many times a printed "only once each turn" or "no more than twice
    /// each turn" clause allows this ability to be activated per turn from
    /// one object. `None` is the ordinary unlimited case.
    pub activation_limit: Option<u8>,
    /// Whether anyone may activate it, not just the permanent's controller.
    /// The permanent stays the ability's source whoever pays, so the damage
    /// it deals is still the permanent's damage.
    pub any_player_may_activate: bool,
}

impl ActivatedAbilityDef {
    #[must_use]
    pub const fn new(costs: &'static [AbilityCostDef]) -> Self {
        Self::with_costs(AbilityCostList::borrowed(costs))
    }

    #[must_use]
    pub(crate) const fn with_costs(costs: AbilityCostList) -> Self {
        Self {
            source_zones: &[ZoneKind::Battlefield],
            costs,
            targets: &[],
            procedure: AbilityProcedureDef::Shared,
            timing: ActivationTimingDef::Any,
            activation_limit: None,
            any_player_may_activate: false,
        }
    }

    /// "Any player may activate this ability." The permanent stays the
    /// source, so what it does is still the permanent's doing.
    #[must_use]
    pub const fn open_to_any_player(mut self) -> Self {
        self.any_player_may_activate = true;
        self
    }

    #[must_use]
    pub const fn with_source_zones(mut self, source_zones: &'static [ZoneKind]) -> Self {
        self.source_zones = source_zones;
        self
    }

    #[must_use]
    pub const fn with_targets(mut self, targets: &'static [AbilityTargetDef]) -> Self {
        self.targets = targets;
        self
    }

    #[must_use]
    pub const fn with_procedure(mut self, procedure: AbilityProcedureDef) -> Self {
        self.procedure = procedure;
        self
    }

    #[must_use]
    pub const fn with_timing(mut self, timing: ActivationTimingDef) -> Self {
        self.timing = timing;
        self
    }

    #[must_use]
    pub const fn with_activation_limit(mut self, limit: u8) -> Self {
        self.activation_limit = Some(limit);
        self
    }
}

/// Whether a condition has to hold for every matching player or just one.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum QuantifierDef {
    Every,
    Any,
}

/// How a counted amount is compared against a printed number.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ComparisonDef {
    Less,
    LessOrEqual,
    Equal,
    GreaterOrEqual,
    Greater,
}

/// An intervening-if condition, the "if ..." clause a trigger reads before it
/// does anything. Rule 603.4 checks such a condition twice: once when the
/// ability would go on the stack, and again as it resolves.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TriggerConditionDef {
    /// Whether the original source object is still on the battlefield.
    SourceOnBattlefield,
    /// Whether the source permanent is currently untapped.
    SourceUntapped,
    /// How many objects the query matches, against a printed number.
    ObjectCount {
        query: ObjectQueryDef,
        comparison: ComparisonDef,
        amount: u8,
    },
    /// Whose turn it is, relative to the ability's controller.
    ActivePlayer(PlayerRelation),
    /// How many spells a matching player cast during the turn before this
    /// one. "No spells were cast last turn" is every player at zero, and "a
    /// player cast two or more" is any player at two.
    SpellsCastLastTurn {
        /// Whether every matching player has to satisfy the comparison or
        /// only one. "No spells were cast last turn" is every player at zero;
        /// "a player cast two or more" is one player at two.
        quantifier: QuantifierDef,
        player: PlayerRelation,
        comparison: ComparisonDef,
        amount: u8,
    },
    /// How much loyalty the ability's own source has left.
    SourceLoyalty {
        comparison: ComparisonDef,
        amount: u8,
    },
    /// How many times this ability has been activated from its source this
    /// turn, counting the activation now resolving.
    SourceActivationsThisTurn {
        comparison: ComparisonDef,
        amount: u8,
    },
    /// Whether this ability's own source has dealt damage to an opponent of
    /// its controller at any point this turn, by any means.
    SourceDealtDamageToOpponentThisTurn,
    /// Whether the ability's own source is tapped, using last-known
    /// information if it has left the battlefield.
    SourceIsTapped,
    /// Whether the ability's own source is untapped, using last-known
    /// information if it has left the battlefield. Not the negation of
    /// [`Self::SourceIsTapped`] for an object that was never on the
    /// battlefield, which is neither.
    SourceIsUntapped,
    /// Whether the ability's controller is at or below this life total, for
    /// the fateful-hour clauses. Read live, so a static ability guarded by it
    /// switches on and off as life moves rather than being fixed when the
    /// permanent arrived.
    ControllerLifeAtMost(u16),
    /// Whether this ability's controller controls a creature whose power is
    /// at least every other creature's, which is what "the greatest power or
    /// tied for the greatest power" asks. False when no creature is on the
    /// battlefield at all.
    ControlsGreatestPowerCreature,
    /// Whether a creature has gone to a graveyard this turn. The condition
    /// form of the morbid value, for the intervening-ifs that ask rather than
    /// pick an amount.
    CreatureDiedThisTurn,
    /// Whether the ability's own source matches. The mirror of
    /// [`Self::AttachedPermanentMatches`] pointed at the source itself, for
    /// the intervening-ifs that ask what the permanent has been doing.
    SourceMatches { object: ObjectPredicateDef },
    /// Whether what the ability's source is attached to matches. This is what
    /// "as long as equipped creature is a Human" asks, and it is read live so
    /// the answer follows the Equipment as it moves.
    AttachedPermanentMatches { object: ObjectPredicateDef },
    /// How many counters of one kind the ability's own source carries. This
    /// is what "as long as there are exactly three tide counters on this
    /// creature" asks, and it is read live rather than captured.
    SourceCounters {
        kind: CounterKind,
        comparison: ComparisonDef,
        amount: u8,
    },
    /// Whether what a target slot points at still matches. Read when the
    /// condition is checked, so a delayed effect can ask about the target as
    /// it is then rather than as it was.
    TargetMatches {
        slot: TargetIndex,
        object: ObjectPredicateDef,
    },
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TriggeredAbilityDef {
    pub source_zones: &'static [ZoneKind],
    pub event: TriggerEventDef,
    pub targets: &'static [AbilityTargetDef],
    pub procedure: AbilityProcedureDef,
    /// Held by reference so that this definition stays small enough to pass
    /// around by value alongside a captured trigger.
    pub condition: Option<&'static TriggerConditionDef>,
}

impl TriggeredAbilityDef {
    #[must_use]
    pub const fn new(event: TriggerEventDef) -> Self {
        Self {
            source_zones: &[ZoneKind::Battlefield],
            event,
            targets: &[],
            procedure: AbilityProcedureDef::Shared,
            condition: None,
        }
    }

    #[must_use]
    pub const fn with_condition(mut self, condition: &'static TriggerConditionDef) -> Self {
        self.condition = Some(condition);
        self
    }

    #[must_use]
    pub const fn with_source_zones(mut self, source_zones: &'static [ZoneKind]) -> Self {
        self.source_zones = source_zones;
        self
    }

    #[must_use]
    pub const fn with_targets(mut self, targets: &'static [AbilityTargetDef]) -> Self {
        self.targets = targets;
        self
    }

    #[must_use]
    pub const fn with_procedure(mut self, procedure: AbilityProcedureDef) -> Self {
        self.procedure = procedure;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StaticAbilityDef {
    pub source_zones: &'static [ZoneKind],
}

/// The rules procedure and mana cost supplied by a printed
/// alternative-casting keyword.
///
/// A play option exposes a derived [`AlternativeCostDef`] whose identity is
/// the positional [`AbilityId`] of this clause. An overload clause uses its
/// [`AbilityDef::effect`] as the targetless text-replacement result; flashback
/// uses `EffectDef::None` and changes where the card may be cast and where it
/// goes after the stack.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AlternativeCastAbilityDef {
    pub mana_cost: AlternativeCastManaCostDef,
    pub kind: AlternativeCastKindDef,
    /// Rules text for the spell as modified by this alternative, when the
    /// procedure changes its visible instructions (as overload does).
    pub stack_text: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AlternativeCastKindDef {
    Flashback,
    Overload,
    /// Cast from hand only in the window opened by drawing the card, as the
    /// first card drawn that turn.
    Miracle,
}

/// How an alternative-casting ability determines the cost it supplies.
///
/// Printed abilities normally carry a fixed cost. A granted ability such as
/// Snapcaster Mage's flashback instead reads the mana cost of the card that
/// gained it, after a concrete play option has selected the spell form.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AlternativeCastManaCostDef {
    Fixed(ManaCost),
    ThisCardManaCost,
}

impl AlternativeCastManaCostDef {
    #[must_use]
    pub const fn resolve(self, card_mana_cost: Option<ManaCost>) -> Option<ManaCost> {
        match self {
            Self::Fixed(mana_cost) => Some(mana_cost),
            Self::ThisCardManaCost => card_mana_cost,
        }
    }
}

impl AlternativeCastKindDef {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Flashback => "Flashback",
            Self::Overload => "Overload",
            Self::Miracle => "Miracle",
        }
    }
}

impl AlternativeCastAbilityDef {
    #[must_use]
    pub fn rules_text(self) -> String {
        match (self.kind, self.mana_cost) {
            (AlternativeCastKindDef::Flashback, AlternativeCastManaCostDef::Fixed(mana_cost)) => {
                format!(
                    "Flashback {mana_cost} (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
                )
            }
            (
                AlternativeCastKindDef::Flashback,
                AlternativeCastManaCostDef::ThisCardManaCost,
            ) => "Flashback—the flashback cost is equal to this card's mana cost. (You may cast this card from your graveyard for its flashback cost. Then exile it.)".into(),
            (AlternativeCastKindDef::Overload, AlternativeCastManaCostDef::Fixed(mana_cost)) => {
                format!(
                    "Overload {mana_cost} (You may cast this spell for its overload cost. If you do, change \"target\" in its text to \"each.\")",
                )
            }
            (
                AlternativeCastKindDef::Overload,
                AlternativeCastManaCostDef::ThisCardManaCost,
            ) => "Overload—the overload cost is equal to this card's mana cost. (You may cast this spell for its overload cost. If you do, change \"target\" in its text to \"each.\")".into(),
            (AlternativeCastKindDef::Miracle, AlternativeCastManaCostDef::Fixed(mana_cost)) => {
                format!(
                    "Miracle {mana_cost} (You may cast this card for its miracle cost when you draw it if it's the first card you drew this turn.)",
                )
            }
            (
                AlternativeCastKindDef::Miracle,
                AlternativeCastManaCostDef::ThisCardManaCost,
            ) => "Miracle—the miracle cost is equal to this card's mana cost. (You may cast this card for its miracle cost when you draw it if it's the first card you drew this turn.)".into(),
        }
    }

    #[must_use]
    pub fn alternative_cost(
        self,
        ability: AbilityId,
        card_mana_cost: Option<ManaCost>,
    ) -> Option<AlternativeCostDef> {
        Some(AlternativeCostDef {
            id: AlternativeCostId(ability.0),
            label: self.kind.label().into(),
            mana_cost: self.mana_cost.resolve(card_mana_cost)?,
        })
    }
}

/// A replacement ability changes how an event happens and never uses the
/// stack. It is modeled separately from a triggered ability even when both
/// watch the same event.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ReplacementAbilityDef {
    pub source_zones: &'static [ZoneKind],
    pub event: ReplacementEventDef,
    pub condition: Option<ReplacementConditionDef>,
    /// Whether the affected player may decline to apply this replacement.
    pub optional: bool,
}

impl ReplacementAbilityDef {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            source_zones: &[ZoneKind::Battlefield],
            event: ReplacementEventDef::SourceEntersBattlefield,
            condition: None,
            optional: false,
        }
    }

    #[must_use]
    pub const fn with_event(mut self, event: ReplacementEventDef) -> Self {
        self.event = event;
        self
    }

    #[must_use]
    pub const fn with_condition(mut self, condition: ReplacementConditionDef) -> Self {
        self.condition = Some(condition);
        self
    }

    #[must_use]
    pub const fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    #[must_use]
    pub const fn with_source_zones(mut self, source_zones: &'static [ZoneKind]) -> Self {
        self.source_zones = source_zones;
        self
    }
}

impl Default for ReplacementAbilityDef {
    fn default() -> Self {
        Self::new()
    }
}

/// A rules-defined action a player may take without using the stack, such as
/// turning a face-down permanent face up. This is deliberately distinct from
/// both activated abilities and mana abilities; its timing category is never
/// inferred from its cost or effect.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SpecialActionDef {
    pub source_zones: &'static [ZoneKind],
    pub costs: &'static [AbilityCostDef],
}

impl SpecialActionDef {
    #[must_use]
    pub const fn new(source_zones: &'static [ZoneKind], costs: &'static [AbilityCostDef]) -> Self {
        Self {
            source_zones,
            costs,
        }
    }
}

impl StaticAbilityDef {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            source_zones: &[ZoneKind::Battlefield],
        }
    }

    #[must_use]
    pub const fn with_source_zones(mut self, source_zones: &'static [ZoneKind]) -> Self {
        self.source_zones = source_zones;
        self
    }
}

impl Default for StaticAbilityDef {
    fn default() -> Self {
        Self::new()
    }
}

/// The quality a "bands with other" ability names.
///
/// Each printed quality is its own variant rather than a free-form predicate,
/// the way protection is one keyword per color: the checkpoint wire names them
/// individually, and only two have ever been printed.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BandingQuality {
    /// "bands with other legendary creatures", which the five Legends lands
    /// grant to legendary creatures of their own color.
    LegendaryCreatures,
    /// "bands with other creatures named Wolves of the Hunt", printed on the
    /// tokens Master of the Hunt makes.
    WolvesOfTheHunt,
}

impl BandingQuality {
    /// Every printed quality, for the rules that have to try each one.
    pub const ALL: [Self; 2] = [Self::LegendaryCreatures, Self::WolvesOfTheHunt];

    /// What a creature must be to join a band formed on this quality.
    #[must_use]
    pub const fn predicate(self) -> &'static ObjectPredicateDef {
        match self {
            Self::LegendaryCreatures => &LEGENDARY_CREATURE,
            Self::WolvesOfTheHunt => &WOLF_OF_THE_HUNT,
        }
    }
}

static LEGENDARY_CREATURE: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::HasType(CardType::Creature),
    ObjectPredicateDef::Supertype(CardSupertype::Legendary),
]);

static WOLF_OF_THE_HUNT: ObjectPredicateDef = ObjectPredicateDef::All(&[
    ObjectPredicateDef::HasType(CardType::Creature),
    ObjectPredicateDef::Named("Wolves of the Hunt"),
]);

/// A keyword ability carried as an ordinary, ordered rules clause.
///
/// The clause's [`AbilityCoverageDef`] says whether the engine currently
/// executes the keyword. This keeps unimplemented keywords such as banding
/// visible and accurately reflected in aggregate coverage without hiding them
/// in card-level booleans.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum KeywordAbility {
    Flying,
    Trample,
    Haste,
    FirstStrike,
    DoubleStrike,
    Banding,
    /// CR 702.21j. Banding narrowed to a quality: the band's members must all
    /// have that quality, and at least one of them must have this ability.
    /// Unlike plain banding there is no free passenger, and the damage rule
    /// wants two qualifying creatures rather than one.
    BandsWithOther(BandingQuality),
    Vigilance,
    Defender,
    Deathtouch,
    Lifelink,
    Reach,
    Flash,
    Hexproof,
    Shroud,
    /// Unleash. The engine implements both halves: an optional +1/+1 counter
    /// as the permanent enters, and no blocking while it carries one.
    Unleash,
    Intimidate,
    /// CR 702.110. The creature cannot be blocked except by two or more
    /// creatures. A constraint on the completed declaration rather than on
    /// any one block: the first blocker is always legal, and it is finishing
    /// with exactly one that the rules forbid.
    Menace,
    Undying,
    Indestructible,
    /// "Attacks each combat if able." Not a printed keyword, but it behaves
    /// like one: a static requirement with no parameters that several cards
    /// state in the same words.
    AttacksEachCombatIfAble,
    /// CR 702.14. One keyword parameterized by land type: the creature cannot
    /// be blocked as long as the defending player controls a land of that
    /// type. The printed variants differ only in which type they name.
    Landwalk(BasicLandType),
    /// Landwalk naming a land supertype.
    LegendaryLandwalk,
    ProtectionFrom(ManaColor),
    /// CR 702.16. Protection is really one keyword per quality, and a quality
    /// need not be a color: "protection from Zombies" names a creature type
    /// and behaves identically otherwise.
    ProtectionFromCreatureType(ProtectedCreatureType),
    /// Protection from the card type, which is one quality rather than a
    /// family of them and so carries no parameter at all.
    ProtectionFromCreatures,
    /// Protection from every object of two or more colors. Not the union of
    /// the five color qualities: a monocolored source gets through, and a
    /// two-color one is stopped even where neither of its colors alone would
    /// have been.
    ProtectionFromMulticolored,
}

/// The creature types a printed protection clause names. A closed set for the
/// same reason [`BasicLandType`] is: every consumer that has to name one --
/// the checkpoint tag among them -- stays exhaustive, and a new printing adds
/// one variant here rather than an open string nobody can round-trip.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ProtectedCreatureType {
    Zombie,
    Vampire,
    Werewolf,
}

impl ProtectedCreatureType {
    /// The subtype string as it appears on a creature's type line.
    #[must_use]
    pub const fn subtype(self) -> &'static str {
        match self {
            Self::Zombie => "Zombie",
            Self::Vampire => "Vampire",
            Self::Werewolf => "Werewolf",
        }
    }
}

impl KeywordAbility {
    /// A dense index for the keywords that carry no parameter, so a set of
    /// them fits in a bitmask. Protection is excluded: it is really one
    /// keyword per quality, and the qualities are open-ended.
    #[must_use]
    pub const fn simple_index(self) -> Option<u32> {
        Some(match self {
            Self::Flying => 0,
            Self::Trample => 1,
            Self::Haste => 2,
            Self::FirstStrike => 3,
            Self::DoubleStrike => 4,
            Self::Banding => 5,
            Self::Vigilance => 6,
            Self::Defender => 7,
            Self::Deathtouch => 8,
            Self::Lifelink => 9,
            Self::Reach => 10,
            Self::Flash => 11,
            Self::Hexproof => 12,
            Self::Intimidate => 13,
            Self::Undying => 14,
            Self::Menace => 15,
            Self::AttacksEachCombatIfAble => 16,
            Self::ProtectionFromCreatures => 17,
            Self::ProtectionFromMulticolored => 27,
            Self::Indestructible => 18,
            Self::Shroud => 19,
            Self::Unleash => 26,
            // One index per land type, so a set of landwalks still packs into
            // the same bitmask as the parameterless keywords.
            Self::Landwalk(BasicLandType::Plains) => 20,
            Self::Landwalk(BasicLandType::Island) => 21,
            Self::Landwalk(BasicLandType::Swamp) => 22,
            Self::Landwalk(BasicLandType::Mountain) => 23,
            Self::Landwalk(BasicLandType::Forest) => 24,
            Self::LegendaryLandwalk => 25,
            Self::ProtectionFrom(_)
            | Self::ProtectionFromCreatureType(_)
            | Self::BandsWithOther(_) => return None,
        })
    }
}

/// The rules category and structural procedure of an ability. Text and
/// implementation coverage live on [`AbilityDef`] so every printed clause has
/// one canonical text string regardless of how it executes. Identity is
/// supplied only when a definition is attached.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeclarativeAbilityDef {
    Spell(SpellAbilityDef),
    ActivatedMana(ActivatedAbilityDef),
    TriggeredMana(TriggeredAbilityDef),
    Activated(ActivatedAbilityDef),
    Triggered(TriggeredAbilityDef),
    Static(StaticAbilityDef),
    Replacement(ReplacementAbilityDef),
    AlternativeCast(AlternativeCastAbilityDef),
    SpecialAction(SpecialActionDef),
    Keyword(KeywordAbility),
    /// Transitional structural marker for a clause still dispatched through
    /// the owning card's legacy custom behavior.
    Legacy,
}

/// How an ability's declared effect is executed.
///
/// Coverage is deliberately not represented here: a custom effect can be
/// complete or partial, and a declarative effect can likewise have a gap in
/// its costs, targeting, timing, or another non-effect portion of the clause.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EffectExecutionDef {
    Declarative,
    Custom(CardBehavior),
    /// A resolver the card itself supplies, reached through the set module's
    /// ability bindings rather than a shared dispatch key. The clause says so
    /// here so that a reader, the coverage view, and the shared-runtime
    /// boundary all learn how it executes from the clause itself.
    CardOwned,
}

/// The structured effect and the resolver responsible for executing it.
///
/// Custom execution retains the structured definition as documentation and a
/// migration target, but the shared resolver must not execute that definition
/// until the execution kind becomes [`EffectExecutionDef::Declarative`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AbilityProgramDef {
    Effects(EffectDef),
    Replacement(ReplacementEffectDef),
}

/// The structured program and the resolver responsible for executing it.
///
/// Replacement programs are typed separately because they mutate a
/// prospective event and preserve that event across any decisions they make;
/// they are not resolving stack effects.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AbilityEffectDef {
    pub definition: AbilityProgramDef,
    pub execution: EffectExecutionDef,
}

impl AbilityEffectDef {
    #[must_use]
    pub const fn declarative(definition: EffectDef) -> Self {
        Self {
            definition: AbilityProgramDef::Effects(definition),
            execution: EffectExecutionDef::Declarative,
        }
    }

    #[must_use]
    pub const fn replacement_program(definition: ReplacementEffectDef) -> Self {
        Self {
            definition: AbilityProgramDef::Replacement(definition),
            execution: EffectExecutionDef::Declarative,
        }
    }

    #[must_use]
    pub const fn with_execution(mut self, execution: EffectExecutionDef) -> Self {
        self.execution = execution;
        self
    }

    #[must_use]
    pub const fn declarative_definition(self) -> Option<EffectDef> {
        match (self.execution, self.definition) {
            (EffectExecutionDef::Declarative, AbilityProgramDef::Effects(definition)) => {
                Some(definition)
            }
            (EffectExecutionDef::Declarative, AbilityProgramDef::Replacement(_))
            | (EffectExecutionDef::Custom(_) | EffectExecutionDef::CardOwned, _) => None,
        }
    }

    #[must_use]
    pub const fn declarative_replacement(self) -> Option<ReplacementEffectDef> {
        match (self.execution, self.definition) {
            (EffectExecutionDef::Declarative, AbilityProgramDef::Replacement(definition)) => {
                Some(definition)
            }
            (EffectExecutionDef::Declarative, AbilityProgramDef::Effects(_))
            | (EffectExecutionDef::Custom(_) | EffectExecutionDef::CardOwned, _) => None,
        }
    }

    #[must_use]
    pub const fn custom_behavior(self) -> Option<CardBehavior> {
        match self.execution {
            EffectExecutionDef::Custom(behavior) => Some(behavior),
            EffectExecutionDef::Declarative | EffectExecutionDef::CardOwned => None,
        }
    }
}

/// Clause-level implementation coverage, independent of effect dispatch.
///
/// An explanation is optional only for an ordinary complete declarative
/// clause. Complete custom and compatibility clauses keep a note explaining
/// their implementation; partial and metadata-only clauses explain the gap.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AbilityCoverageDef {
    pub status: ImplementationStatus,
    pub explanation: Option<&'static str>,
}

impl AbilityCoverageDef {
    #[must_use]
    pub const fn complete() -> Self {
        Self {
            status: ImplementationStatus::Complete,
            explanation: None,
        }
    }

    #[must_use]
    pub const fn explained_complete(explanation: &'static str) -> Self {
        Self {
            status: ImplementationStatus::Complete,
            explanation: Some(explanation),
        }
    }

    #[must_use]
    pub const fn partial(explanation: &'static str) -> Self {
        Self {
            status: ImplementationStatus::Partial,
            explanation: Some(explanation),
        }
    }

    #[must_use]
    pub const fn metadata_only(explanation: &'static str) -> Self {
        Self {
            status: ImplementationStatus::MetadataOnly,
            explanation: Some(explanation),
        }
    }

    #[must_use]
    pub const fn is_executable(self) -> bool {
        !matches!(self.status, ImplementationStatus::MetadataOnly)
    }
}
