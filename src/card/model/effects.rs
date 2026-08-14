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
    AbilityDef, AddManaEffectDef, BasicLandType, CardTypeSet, ColorSet, CostDef, CounterKind,
    KeywordAbility, ManaColor, ManaCost, ObjectPredicateDef, PlayerRelation, TriggerConditionDef,
    ZoneKind, ZonePlacement,
};

/// An object reference evaluated in the resolving effect's context.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ObjectRefDef {
    /// The game object from which the resolving spell or ability originated.
    Source,
    /// The spell or ability object currently resolving. This is distinct from
    /// [`Self::Source`], which names its originating game object.
    ResolvingObject,
    /// One object saved by an earlier choice in this resolution.
    Binding(ObjectBindingIndex),
    AttachedToSource,
    Target(TargetIndex),
    TriggeringObject,
}

/// A player reference evaluated in the resolving effect's context.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PlayerRefDef {
    /// The controller captured by the resolving spell or ability.
    EffectController,
    EventPlayer,
    /// A target slot that directly names a player.
    Target(TargetIndex),
    /// The current controller of an object, falling back to last-known
    /// information. A target that directly names a player resolves to that
    /// player, preserving the ordinary meaning of "that player or its
    /// controller" selectors.
    ControllerOf(ObjectRefDef),
    /// The owner of an object, using last-known information when necessary.
    OwnerOf(ObjectRefDef),
}

/// A set of players. Relations are measured from the resolving effect's
/// controller unless the relation itself names an event or chosen player.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PlayerSetDef {
    All,
    One(PlayerRefDef),
    Related(PlayerRelation),
}

/// A set of objects selected without targeting.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ObjectSetDef {
    One(ObjectRefDef),
    /// A set of objects saved by an earlier choice or partition in this
    /// resolution.
    Binding(ObjectSetBindingIndex),
    Query(ObjectQueryDef),
    /// Every battlefield permanent sharing the referenced object's effective
    /// name, including the referenced object itself.
    SharingNameWith(ObjectRefDef),
}

/// The typed subject of an effect. A target slot remains its own category
/// because one slot can legally contain players and objects, and because its
/// contents must be legality-checked again on resolution.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EffectRecipientSetDef {
    LegalTargets(TargetIndex),
    Objects(ObjectSetDef),
    Players(PlayerSetDef),
}

/// An object or player set affected by an effect.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EffectRecipientDef(pub EffectRecipientSetDef);

// These const-friendly spellings keep card declarations compact while the
// runtime receives the compositional reference/query model above.
#[allow(non_snake_case, non_upper_case_globals)]
impl EffectRecipientDef {
    pub const Source: Self = Self::object(ObjectRefDef::Source);
    pub const AttachedPermanent: Self = Self::object(ObjectRefDef::AttachedToSource);
    pub const Controller: Self = Self::player(PlayerRefDef::EffectController);
    pub const Opponent: Self = Self::players(PlayerSetDef::Related(PlayerRelation::Opponent));
    pub const EachPlayer: Self = Self::players(PlayerSetDef::All);
    pub const TriggeringObject: Self = Self::object(ObjectRefDef::TriggeringObject);
    pub const ControllerOfTriggeringObject: Self =
        Self::player(PlayerRefDef::ControllerOf(ObjectRefDef::TriggeringObject));
    pub const EventPlayer: Self = Self::player(PlayerRefDef::EventPlayer);

    #[must_use]
    pub const fn object(object: ObjectRefDef) -> Self {
        Self(EffectRecipientSetDef::Objects(ObjectSetDef::One(object)))
    }

    #[must_use]
    pub const fn objects(objects: ObjectSetDef) -> Self {
        Self(EffectRecipientSetDef::Objects(objects))
    }

    #[must_use]
    pub const fn player(player: PlayerRefDef) -> Self {
        Self::players(PlayerSetDef::One(player))
    }

    #[must_use]
    pub const fn players(players: PlayerSetDef) -> Self {
        Self(EffectRecipientSetDef::Players(players))
    }

    #[must_use]
    pub const fn legal_target(self) -> Option<TargetIndex> {
        match self.0 {
            EffectRecipientSetDef::LegalTargets(target) => Some(target),
            EffectRecipientSetDef::Objects(_) | EffectRecipientSetDef::Players(_) => None,
        }
    }

    #[must_use]
    pub const fn object_reference(self) -> Option<ObjectRefDef> {
        match self.0 {
            EffectRecipientSetDef::Objects(ObjectSetDef::One(reference)) => Some(reference),
            EffectRecipientSetDef::LegalTargets(_)
            | EffectRecipientSetDef::Objects(
                ObjectSetDef::Binding(_)
                | ObjectSetDef::Query(_)
                | ObjectSetDef::SharingNameWith(_),
            )
            | EffectRecipientSetDef::Players(_) => None,
        }
    }

    #[must_use]
    pub const fn object_query(self) -> Option<ObjectQueryDef> {
        match self.0 {
            EffectRecipientSetDef::Objects(ObjectSetDef::Query(query)) => Some(query),
            EffectRecipientSetDef::LegalTargets(_)
            | EffectRecipientSetDef::Objects(
                ObjectSetDef::One(_) | ObjectSetDef::Binding(_) | ObjectSetDef::SharingNameWith(_),
            )
            | EffectRecipientSetDef::Players(_) => None,
        }
    }

    #[must_use]
    pub const fn object_binding(self) -> Option<ObjectBindingIndex> {
        match self.object_reference() {
            Some(ObjectRefDef::Binding(binding)) => Some(binding),
            Some(
                ObjectRefDef::Source
                | ObjectRefDef::ResolvingObject
                | ObjectRefDef::AttachedToSource
                | ObjectRefDef::Target(_)
                | ObjectRefDef::TriggeringObject,
            )
            | None => None,
        }
    }

    #[must_use]
    pub const fn Target(target: TargetIndex) -> Self {
        Self(EffectRecipientSetDef::LegalTargets(target))
    }

    #[must_use]
    pub const fn ControllerOfTarget(target: TargetIndex) -> Self {
        Self::player(PlayerRefDef::ControllerOf(ObjectRefDef::Target(target)))
    }

    #[must_use]
    pub const fn ObjectsSharingNameWithTarget(target: TargetIndex) -> Self {
        Self::objects(ObjectSetDef::SharingNameWith(ObjectRefDef::Target(target)))
    }

    #[must_use]
    pub const fn matching_objects(
        object: ObjectPredicateDef,
        zones: &'static [ZoneKind],
        controller_or_owner: PlayerRelation,
    ) -> Self {
        Self::objects(ObjectSetDef::Query(ObjectQueryDef::matching(
            object,
            zones,
            controller_or_owner,
        )))
    }

    #[must_use]
    pub const fn objects_controlled_by_target(
        object: ObjectPredicateDef,
        slot: TargetIndex,
    ) -> Self {
        Self::objects(ObjectSetDef::Query(ObjectQueryDef::controlled_by(
            object,
            &[ZoneKind::Battlefield],
            PlayerSetDef::One(PlayerRefDef::ControllerOf(ObjectRefDef::Target(slot))),
        )))
    }

    #[must_use]
    pub const fn objects_owned_by_target(object: ObjectPredicateDef, slot: TargetIndex) -> Self {
        Self::objects(ObjectSetDef::Query(ObjectQueryDef::owned_by(
            object,
            &[ZoneKind::Battlefield],
            PlayerSetDef::One(PlayerRefDef::Target(slot)),
        )))
    }

    #[must_use]
    pub const fn cards_owned_by_target(
        object: ObjectPredicateDef,
        zones: &'static [ZoneKind],
        slot: TargetIndex,
    ) -> Self {
        Self::objects(ObjectSetDef::Query(ObjectQueryDef::owned_by(
            object,
            zones,
            PlayerSetDef::One(PlayerRefDef::Target(slot)),
        )))
    }
}

/// The lifetime of a continuous effect created by a resolving spell or
/// ability. Static effects use [`EffectDef::StaticApply`] instead: they are
/// derived live from the ability that creates them and have no stored
/// expiration.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResolvedEffectDurationDef {
    Permanent,
    UntilEndOfTurn,
    /// Until the beginning of the resolving ability's controller's next
    /// upkeep, which outlives the cleanup that ends an until-end-of-turn
    /// effect.
    UntilYourNextUpkeep,
    /// Until the next turn of the effect's controller begins. The affected
    /// turn is captured when the resolving effect is created.
    UntilYourNextTurn,
    /// For as long as the effect's own source stays tapped. Unlike every
    /// other resolving duration this one has no deadline: the artifact that
    /// tapped to make it decides when it ends by untapping.
    WhileSourceTapped,
}

/// Whether a damage-prevention rule matches combat damage, or damage of any
/// kind.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DamageKindDef {
    Any,
    Combat,
}

/// The source side of a prospective damage event.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DamageSourceMatcherDef {
    Any,
    /// A checkpoint-stable relational source group named by the card text.
    Group(DamageSourceGroupDef),
    /// The object receiving a static applied effect.
    AffectedObject,
    Object(ObjectRefDef),
    Except(ObjectRefDef),
    Matching(ObjectPredicateDef),
}

/// The recipient side of a prospective damage event.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DamageRecipientMatcherDef {
    Any,
    /// The object receiving a static applied effect.
    AffectedObject,
    Recipients(EffectRecipientDef),
    /// A player resolved when the prevention is created, plus creatures that
    /// player controls when damage would be dealt.
    PlayerAndCreaturesControlledBy(PlayerRefDef),
}

/// A conjunctive matcher over a prospective damage event.
///
/// Preventing damage both to and by one object is represented by two rules in
/// an [`EffectDef::Sequence`] or [`AppliedEffectDef::Composite`]. Keeping each
/// leaf conjunctive makes resolution and spending order explicit.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DamageEventMatcherDef {
    pub kind: DamageKindDef,
    pub source: DamageSourceMatcherDef,
    pub recipient: DamageRecipientMatcherDef,
}

impl DamageEventMatcherDef {
    pub const ANY: Self = Self {
        kind: DamageKindDef::Any,
        source: DamageSourceMatcherDef::Any,
        recipient: DamageRecipientMatcherDef::Any,
    };

    pub const COMBAT: Self = Self {
        kind: DamageKindDef::Combat,
        source: DamageSourceMatcherDef::Any,
        recipient: DamageRecipientMatcherDef::Any,
    };

    #[must_use]
    pub const fn to(recipients: EffectRecipientDef) -> Self {
        Self {
            recipient: DamageRecipientMatcherDef::Recipients(recipients),
            ..Self::ANY
        }
    }

    #[must_use]
    pub const fn from(source: ObjectRefDef) -> Self {
        Self {
            source: DamageSourceMatcherDef::Object(source),
            ..Self::ANY
        }
    }

    #[must_use]
    pub const fn from_group_to(
        source: DamageSourceGroupDef,
        recipients: EffectRecipientDef,
    ) -> Self {
        Self {
            source: DamageSourceMatcherDef::Group(source),
            recipient: DamageRecipientMatcherDef::Recipients(recipients),
            ..Self::ANY
        }
    }

    #[must_use]
    pub const fn combat_to(recipients: EffectRecipientDef) -> Self {
        Self {
            recipient: DamageRecipientMatcherDef::Recipients(recipients),
            ..Self::COMBAT
        }
    }

    #[must_use]
    pub const fn combat_from(source: ObjectRefDef) -> Self {
        Self {
            source: DamageSourceMatcherDef::Object(source),
            ..Self::COMBAT
        }
    }

    #[must_use]
    pub const fn combat_except(source: ObjectRefDef) -> Self {
        Self {
            source: DamageSourceMatcherDef::Except(source),
            ..Self::COMBAT
        }
    }

    #[must_use]
    pub const fn to_player_and_creatures_controlled_by(player: PlayerRefDef) -> Self {
        Self {
            recipient: DamageRecipientMatcherDef::PlayerAndCreaturesControlledBy(player),
            ..Self::ANY
        }
    }

    #[must_use]
    pub const fn from_matching_to_affected(source: ObjectPredicateDef) -> Self {
        Self {
            kind: DamageKindDef::Any,
            source: DamageSourceMatcherDef::Matching(source),
            recipient: DamageRecipientMatcherDef::AffectedObject,
        }
    }

    pub const COMBAT_FROM_AFFECTED: Self = Self {
        kind: DamageKindDef::Combat,
        source: DamageSourceMatcherDef::AffectedObject,
        recipient: DamageRecipientMatcherDef::Any,
    };

    pub const COMBAT_TO_AFFECTED: Self = Self {
        kind: DamageKindDef::Combat,
        source: DamageSourceMatcherDef::Any,
        recipient: DamageRecipientMatcherDef::AffectedObject,
    };
}

/// How long or how often a resolving prevention rule can be spent.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DamagePreventionCapacityDef {
    Amount(ValueDef),
    Events(u8),
    Unlimited,
}

/// How much of each matched damage event is prevented.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DamageCoverageDef {
    All,
    HalfRoundedDown,
}

/// A synchronous consequence of damage prevented by one rule.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DamagePreventionFollowUpDef {
    GainLife(PlayerRefDef),
}

/// One damage-prevention rule installed by a resolving effect.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DamagePreventionDef {
    pub matcher: DamageEventMatcherDef,
    pub capacity: DamagePreventionCapacityDef,
    pub coverage: DamageCoverageDef,
    pub follow_up: Option<DamagePreventionFollowUpDef>,
}

impl DamagePreventionDef {
    #[must_use]
    pub const fn amount(matcher: DamageEventMatcherDef, amount: ValueDef) -> Self {
        Self::new(matcher, DamagePreventionCapacityDef::Amount(amount))
    }

    #[must_use]
    pub const fn events(matcher: DamageEventMatcherDef, events: u8) -> Self {
        Self::new(matcher, DamagePreventionCapacityDef::Events(events))
    }

    #[must_use]
    pub const fn unlimited(matcher: DamageEventMatcherDef) -> Self {
        Self::new(matcher, DamagePreventionCapacityDef::Unlimited)
    }

    #[must_use]
    pub const fn new(
        matcher: DamageEventMatcherDef,
        capacity: DamagePreventionCapacityDef,
    ) -> Self {
        Self {
            matcher,
            capacity,
            coverage: DamageCoverageDef::All,
            follow_up: None,
        }
    }

    #[must_use]
    pub const fn with_coverage(mut self, coverage: DamageCoverageDef) -> Self {
        self.coverage = coverage;
        self
    }

    #[must_use]
    pub const fn with_follow_up(mut self, follow_up: DamagePreventionFollowUpDef) -> Self {
        self.follow_up = Some(follow_up);
        self
    }
}

/// An add, remove, or set operation over one set-valued characteristic.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SetOperationDef<T> {
    Add(T),
    Remove(T),
    Set(T),
}

/// Creature subtypes named by one layer-4 operation.
///
/// `all` remains semantic rather than expanding to the engine's current list,
/// so a permanent with all creature types also matches types added later.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CreatureTypeSetDef {
    pub named: &'static [&'static str],
    pub all: bool,
}

impl CreatureTypeSetDef {
    #[must_use]
    pub const fn named(named: &'static [&'static str]) -> Self {
        Self { named, all: false }
    }

    pub const ALL: Self = Self {
        named: &[],
        all: true,
    };
}

/// One layer-6 operation over the affected object's abilities.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AbilityOperationDef {
    Add(&'static AbilityDef),
    Remove(AbilityPredicateDef),
}

/// One layer-7 operation over power and toughness.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PowerToughnessOperationDef {
    /// Set base power and toughness in layer 7b.
    SetBase {
        power: ValueDef,
        toughness: ValueDef,
    },
    /// Modify power and toughness in layer 7c.
    Modify {
        power: ValueDef,
        toughness: ValueDef,
    },
}

/// A typed continuous-effect leaf applied in its characteristic's rules
/// layer. Compound transformations use [`AppliedEffectDef::Composite`] so
/// each leaf keeps its own Add, Remove, or Set semantics.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CharacteristicOperationDef {
    Abilities(AbilityOperationDef),
    /// Basic land-subtype operations in layer 4. `Set` additionally has the
    /// rules consequences of CR 305.7; `Add` and `Remove` do not.
    BasicLandTypes(SetOperationDef<&'static [BasicLandType]>),
    CardTypes(SetOperationDef<CardTypeSet>),
    Colors(SetOperationDef<ColorSet>),
    CreatureTypes(SetOperationDef<CreatureTypeSetDef>),
    PowerToughness(PowerToughnessOperationDef),
}

/// A continuous or rules-modifying effect applied to a game object.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppliedEffectDef {
    /// Components applied to the same recipient for the same duration as one
    /// continuous effect.
    Composite(&'static [AppliedEffectDef]),
    /// One typed operation in the characteristic layer named by the leaf.
    Characteristic(CharacteristicOperationDef),
    CannotBeCountered,
    /// The affected permanent's controller may choose to leave it tapped
    /// during their untap step. Unlike
    /// [`Self::DoesNotUntapDuringUntapStep`] this is a choice rather than a
    /// prohibition, so declining is what the printed cards are paying for.
    MayChooseNotToUntap,
    /// The affected permanent is skipped by its controller's ordinary
    /// turn-based untap procedure. Other spells and abilities can still
    /// untap it.
    DoesNotUntapDuringUntapStep,
    /// No Aura may attach to the affected permanent. This restricts both the
    /// Aura spell's targeting and whether an existing attachment stays legal,
    /// so an Aura already on the permanent falls off.
    CannotBeEnchanted,
    /// No new Aura may attach to the affected permanent, but an Aura already
    /// attached remains legal. Guardian Beast needs this narrower prohibition.
    CannotBecomeEnchanted,
    /// Another player cannot gain control of the affected permanent.
    CannotChangeController,
    /// The affected Aura stays attached even when protection would otherwise
    /// make its host an illegal one. This is the printed "This effect doesn't
    /// remove this Aura" exception, which an Aura granting protection from
    /// its own color needs in order to survive granting it.
    RemainsAttachedThroughProtection,
    /// A creature matching this predicate cannot block the affected creature.
    CannotBeBlockedBy(ObjectPredicateDef),
    /// The affected creature cannot block at all. This is the blocker's own
    /// prohibition, the other side of [`Self::CannotBeBlockedBy`], and it is
    /// what "can't block" and "can't block this turn" both say.
    CannotBlock,
    /// The affected creature may block only creatures matching this
    /// predicate. This is the blocker's own restriction, and it narrows what
    /// it may block rather than who may block it.
    CanBlockOnly(ObjectPredicateDef),
    /// The affected creature cannot be declared as an attacker. Unlike
    /// [`EffectDef::CannotAttackUnless`], which a creature prints about
    /// itself, this is applied from elsewhere and so can cover a whole group.
    CannotAttack,
    /// Nothing can block the affected creature. The turn-scoped form of this
    /// is a resolving effect; this is the printed static one, so it holds for
    /// as long as its source does.
    CannotBeBlocked,
    /// Damage a matching source would deal to the affected permanent's
    /// controller is dealt to that permanent instead. The redirection is read
    /// live, so a condition on the recipient -- "as long as this creature is
    /// untapped" -- turns it off without the permanent being touched.
    RedirectPlayerDamageToThis(DamageSourceGroupDef),
    /// An unlimited prevention rule derived live while this static applied
    /// effect exists. Two-sided prevention is a [`Self::Composite`] of source
    /// and recipient matchers.
    PreventDamage(DamageEventMatcherDef),
    Special(&'static str),
}

impl AppliedEffectDef {
    #[must_use]
    pub const fn add_ability(ability: &'static AbilityDef) -> Self {
        Self::Characteristic(CharacteristicOperationDef::Abilities(
            AbilityOperationDef::Add(ability),
        ))
    }

    #[must_use]
    pub const fn remove_abilities(predicate: AbilityPredicateDef) -> Self {
        Self::Characteristic(CharacteristicOperationDef::Abilities(
            AbilityOperationDef::Remove(predicate),
        ))
    }

    #[must_use]
    pub const fn add_basic_land_types(types: &'static [BasicLandType]) -> Self {
        Self::Characteristic(CharacteristicOperationDef::BasicLandTypes(
            SetOperationDef::Add(types),
        ))
    }

    #[must_use]
    pub const fn set_basic_land_types(types: &'static [BasicLandType]) -> Self {
        Self::Characteristic(CharacteristicOperationDef::BasicLandTypes(
            SetOperationDef::Set(types),
        ))
    }

    #[must_use]
    pub const fn add_card_types(types: CardTypeSet) -> Self {
        Self::Characteristic(CharacteristicOperationDef::CardTypes(SetOperationDef::Add(
            types,
        )))
    }

    #[must_use]
    pub const fn add_creature_types(types: CreatureTypeSetDef) -> Self {
        Self::Characteristic(CharacteristicOperationDef::CreatureTypes(
            SetOperationDef::Add(types),
        ))
    }

    #[must_use]
    pub const fn set_creature_types(types: CreatureTypeSetDef) -> Self {
        Self::Characteristic(CharacteristicOperationDef::CreatureTypes(
            SetOperationDef::Set(types),
        ))
    }

    #[must_use]
    pub const fn set_colors(colors: ColorSet) -> Self {
        Self::Characteristic(CharacteristicOperationDef::Colors(SetOperationDef::Set(
            colors,
        )))
    }

    #[must_use]
    pub const fn set_base_power_toughness(power: ValueDef, toughness: ValueDef) -> Self {
        Self::Characteristic(CharacteristicOperationDef::PowerToughness(
            PowerToughnessOperationDef::SetBase { power, toughness },
        ))
    }

    #[must_use]
    pub const fn modify_power_toughness(power: ValueDef, toughness: ValueDef) -> Self {
        Self::Characteristic(CharacteristicOperationDef::PowerToughness(
            PowerToughnessOperationDef::Modify { power, toughness },
        ))
    }

    #[must_use]
    pub const fn prevent_damage_from(source: ObjectPredicateDef) -> Self {
        Self::PreventDamage(DamageEventMatcherDef::from_matching_to_affected(source))
    }

    #[must_use]
    pub const fn prevent_combat_damage_from(source: ObjectPredicateDef) -> Self {
        Self::PreventDamage(DamageEventMatcherDef {
            kind: DamageKindDef::Combat,
            source: DamageSourceMatcherDef::Matching(source),
            recipient: DamageRecipientMatcherDef::AffectedObject,
        })
    }
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

/// A player and the costs that player may choose to pay.
///
/// The rules procedure interpreting the surrounding effect decides which
/// cost atoms it can offer and how a successful payment resumes that effect.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PaymentDef {
    pub payer: PlayerRelation,
    pub costs: &'static [CostDef],
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

impl PaymentDef {
    #[must_use]
    pub const fn new(payer: PlayerRelation, costs: &'static [CostDef]) -> Self {
        Self { payer, costs }
    }
}

/// A reusable condition evaluated in an effect's source and event context.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ConditionDef {
    /// At least one object matches this zone, controller, and object query.
    Exists(ObjectQueryDef),
}

/// A condition checked while deciding whether a replacement ability applies
/// How cards are selected for a discard effect.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiscardSelectionDef {
    /// Each affected player chooses cards from their own hand.
    RecipientChooses,
    /// The engine selects cards using the recorded random seed.
    Random,
}

/// A private look at the top of a library followed by one bounded card
/// selection. Selected and unselected cards can go to different zones; an
/// optional follow-up resumes only after the choice is complete. This covers
/// both selection spells such as Impulse and scry-then-draw sequencing.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TopCardSelectionDef {
    pub count: ValueDef,
    pub minimum: u8,
    pub maximum: u8,
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

/// A payment offered while an effect resolves.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EffectPaymentDef {
    Costs(PaymentDef),
    /// A const-friendly fixed mana payment.
    Mana {
        payer: PlayerRefDef,
        cost: ManaCost,
    },
    /// A generic mana payment whose amount is evaluated at resolution.
    GenericMana {
        payer: PlayerRefDef,
        amount: ValueDef,
    },
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
    /// Offer a structured optional payment and continue only when it is paid.
    #[must_use]
    pub const fn optional(payment: PaymentDef, if_paid: &'static EffectDef) -> Self {
        Self {
            payment: EffectPaymentDef::Costs(payment),
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
            payment: EffectPaymentDef::Mana {
                payer: PlayerRefDef::EffectController,
                cost,
            },
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
    None,
    Sequence(&'static [EffectDef]),
    /// Select one branch using the game's replay-stable seeded RNG.
    Randomized {
        likelihood: LikelihoodDef,
        on_success: &'static EffectDef,
        on_failure: &'static EffectDef,
    },
    Choose(ChooseDef),
    PayOr(PayOrDef),
    SplitIntoPiles(SplitIntoPilesDef),
    /// Install a resolved damage-prevention rule for the named duration.
    PreventDamage {
        prevention: DamagePreventionDef,
        duration: ResolvedEffectDurationDef,
    },
    AddMana(AddManaEffectDef),
    DealDamage {
        recipient: EffectRecipientDef,
        amount: ValueDef,
    },
    GainLife {
        recipient: EffectRecipientDef,
        amount: ValueDef,
    },
    /// Removes every counter of one kind from the recipient, however many
    /// there are.
    RemoveAllCounters {
        object: EffectRecipientDef,
        kind: CounterKind,
    },
    /// The object sits out that many of its controller's untap steps,
    /// starting with their next one. Unlike the continuous prohibition this
    /// is spent as those steps arrive.
    SkipNextUntapSteps {
        object: EffectRecipientDef,
        count: u8,
    },
    /// The object is destroyed when this combat phase ends. Unlike an
    /// end-step destruction this happens while the combat that caused it is
    /// still the current phase.
    DestroyAtEndOfCombat {
        object: EffectRecipientDef,
    },
    /// Poison counters given to a player. Ten of them is a state-based loss,
    /// which is why this is not expressible as life loss.
    AddPoisonCounters {
        recipient: EffectRecipientDef,
        amount: ValueDef,
    },
    DrawCards {
        recipient: EffectRecipientDef,
        amount: ValueDef,
    },
    /// Randomizes each recipient player's library. Effects that shuffle
    /// cards from other zones into a library first express those zone moves
    /// with [`Self::MoveToZone`], then use this shared operation.
    ShuffleLibrary {
        player: EffectRecipientDef,
    },
    /// The affected player loses all unspent mana without invoking the
    /// turn-based mana-pool emptying procedure (and therefore without mana
    /// burn in formats that use it).
    EmptyManaPool {
        player: EffectRecipientDef,
    },
    /// Each recipient discards that many cards selected in the specified way.
    /// A player holding fewer cards discards their whole hand.
    Discard {
        recipient: EffectRecipientDef,
        amount: ValueDef,
        selection: DiscardSelectionDef,
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
    Tap {
        object: EffectRecipientDef,
    },
    /// The recipient does not untap during its controller's untap step for as
    /// long as the effect's own source stays tapped. Unlike
    /// [`Self::SkipNextUntapSteps`] nothing is spent: the source decides when
    /// it ends by untapping.
    DoesNotUntapWhileSourceTapped {
        object: EffectRecipientDef,
    },
    /// CR 506.4: the permanent stops attacking or blocking, and anything
    /// blocking it stops. An attacker removed this way was still blocked, so
    /// it deals no damage rather than getting through.
    RemoveFromCombat {
        object: EffectRecipientDef,
    },
    Untap {
        object: EffectRecipientDef,
    },
    /// For the rest of the turn, damage the target would deal to the
    /// recipient player is dealt to this effect's own source instead.
    RedirectTargetDamageToSourceThisTurn {
        player: EffectRecipientDef,
        from: TargetIndex,
    },
    /// Puts token copies of `token` onto the battlefield under the resolving
    /// object's controller.
    CreateToken {
        token: CardDefinitionId,
        count: ValueDef,
        /// Whether the created token arrives tapped, as "create a tapped 2/2
        /// black Zombie creature token" asks.
        tapped: bool,
    },
    /// Creates a token copying the recipient's copiable values, which for a
    /// token is the definition it was created from. This is populate, whose
    /// copy is always of a token and so never has to reproduce a printed
    /// card's characteristics.
    CreateTokenCopyOf {
        object: EffectRecipientDef,
    },
    /// An Aura spell attaching itself to what it enchants. The permanent the
    /// spell becomes is what attaches, so this is only meaningful on the spell
    /// clause of an Aura.
    Attach {
        object: EffectRecipientDef,
    },
    Destroy {
        object: EffectRecipientDef,
        can_regenerate: bool,
    },
    /// Creates a regeneration shield (CR 701.15). The shield is not the
    /// regeneration: it waits, and the next destruction this turn is replaced
    /// by tapping the permanent, removing it from combat, and removing all
    /// damage from it. Shields that go unused are cleared in cleanup, so a
    /// creature that was never destroyed keeps nothing.
    Regenerate {
        object: EffectRecipientDef,
    },
    Sacrifice {
        object: EffectRecipientDef,
    },
    /// Each recipient player chooses one permanent they control that matches,
    /// and sacrifices it. Unlike [`Self::Sacrifice`] the choice is the
    /// player's, so nothing happens when they control nothing matching.
    SacrificeOfChoice {
        player: EffectRecipientDef,
        object: ObjectPredicateDef,
        /// Run after the sacrifice, with the sacrificed permanent's power as
        /// [`ValueDef::TriggerEventAmount`]. A sacrifice of choice waits on a
        /// decision, so anything reading what was sacrificed has to be part
        /// of the same continuation rather than the next effect in sequence.
        then: Option<&'static EffectDef>,
        /// Whether the player may decline. An optional sacrifice runs `then`
        /// only when something was actually sacrificed, which is what "if a
        /// player does" means; a compulsory one runs it either way, so an
        /// amount read off nothing is zero rather than skipped.
        optional: bool,
    },
    /// Put that many cards from the top of a library into its owner's
    /// graveyard.
    Mill {
        player: EffectRecipientDef,
        amount: ValueDef,
    },
    /// One player looks at another's hand. Nothing changes zones and no
    /// decision follows; the looking player simply knows.
    LookAtHand {
        player: EffectRecipientDef,
    },
    /// Look at the top card of a library and, if it matches, offer to take
    /// it. Looking is private and changes nothing, so declining leaves the
    /// card exactly where it was.
    LookAtTopAndMayTake {
        player: EffectRecipientDef,
        object: ObjectPredicateDef,
    },
    /// Look privately at the top cards of a library, choose a bounded subset,
    /// place both groups, then optionally continue resolving.
    LookAtTopAndSelect {
        player: EffectRecipientDef,
        selection: &'static TopCardSelectionDef,
    },
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
        /// tapped, as a fetch land's does.
        enters_tapped: bool,
    },
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
    /// Replace the named player's next draw this turn with another effect.
    /// The replacement is frozen with the resolving object and consumed even
    /// when its instructions cannot move a card.
    ReplaceNextDrawThisTurn {
        player: EffectRecipientDef,
        effect: &'static EffectDef,
    },
    /// Resolve one branch under a particular per-game format profile. Card
    /// definitions remain format-neutral; only the rules procedure varies.
    IfFormat {
        format: Format,
        then: &'static EffectDef,
        otherwise: &'static EffectDef,
    },
    /// Counter a spell and put its card into `zone`. Ordinary counters use
    /// the graveyard; replacement-style counters such as Dissipate use exile.
    Counter {
        object: EffectRecipientDef,
        zone: ZoneKind,
    },
    /// Deals damage and gains its controller that much life, but no more
    /// than the recipient had to give: a player's life total, a
    /// planeswalker's loyalty, or a creature's toughness, each read before
    /// the damage. Draining an almost-dead target gains only what was there.
    DrainLife {
        recipient: EffectRecipientDef,
        amount: ValueDef,
    },
    /// Adds mana of one colour, however much a value says. Mana abilities use
    /// [`Self::AddMana`] with a fixed amount so the mana planner can read
    /// them without resolving anything; this is for the effects that cannot
    /// know their amount until they resolve.
    AddManaEqualTo {
        color: ManaColor,
        amount: ValueDef,
    },
    AddCounters {
        object: EffectRecipientDef,
        kind: CounterKind,
        amount: ValueDef,
    },
    /// On resolution, choose two different basic land-type words and apply
    /// the resulting indefinite, noncopiable text change to the object.
    ChangeTextBasicLandType {
        object: EffectRecipientDef,
    },
    /// Replaces the source permanent's copiable values with the target's.
    /// Some copy effects, such as Thespian's Stage, retain the resolving
    /// ability as an exception to the copied values.
    BecomeCopyOf {
        object: EffectRecipientDef,
        retain_source_ability: bool,
    },
    /// Stops the affected players casting noncreature spells for the rest of
    /// the turn.
    CannotCastNoncreatureSpellsThisTurn {
        player: EffectRecipientDef,
    },
    /// Lets the next sorcery its controller casts this turn be cast as
    /// though it had flash.
    GrantFlashToNextSorcery,
    /// An effect the named player may decline. Held by reference so that
    /// `EffectDef` does not grow a recursive inline copy of itself.
    May {
        player: EffectRecipientDef,
        effect: &'static EffectDef,
    },
    /// Exiles, remembering which object sent it there so a later clause can
    /// bring it back. This is the Oblivion Ring shape.
    ExileLinkedToSource {
        object: EffectRecipientDef,
    },
    /// Returns everything this ability's source exiled, to the named zone.
    /// A returned permanent keeps `grant` until end of turn, which is how
    /// Obzedat comes back ready to attack.
    ReturnLinkedExiles {
        zone: ZoneKind,
        grant: Option<KeywordAbility>,
    },
    /// Makes an object unblockable for the rest of the turn.
    MakeUnblockableThisTurn {
        object: EffectRecipientDef,
    },
    /// The recipient cannot be regenerated for the rest of the turn. CR
    /// 701.19c: regeneration shields are not removed and resolving effects may
    /// still create them, but they cannot apply while the prohibition holds.
    CannotRegenerateThisTurn {
        object: EffectRecipientDef,
    },
    /// Gain control of the recipient for as long as the ability's source
    /// stays on the battlefield under the same controller. Unlike
    /// [`Self::GainControlThisTurn`] this outlives the turn and ends when the
    /// source does, which is the "for as long as you control this creature"
    /// that several printed cards use.
    /// Detain: until the resolving controller's next turn, the recipient
    /// cannot attack or block and its activated abilities cannot be
    /// activated. One effect rather than three, because the keyword is one
    /// thing and the three restrictions always travel together.
    Detain {
        object: EffectRecipientDef,
    },
    GainControlWhileSourceRemains {
        object: EffectRecipientDef,
        /// Whether the source also has to stay tapped, for the cards that
        /// pair this with an optional untap so the choice is a real cost.
        while_tapped: bool,
    },
    /// Gain control of a permanent for the rest of the turn. Control reverts
    /// in cleanup, so nothing needs to remember which effect took it.
    GainControlThisTurn {
        object: EffectRecipientDef,
    },
    /// Runs `then` only if the condition holds where this effect resolves.
    /// A condition on a triggered ability is an intervening-if and is checked
    /// twice; this one is part of the effect and is checked once.
    IfCondition {
        condition: &'static TriggerConditionDef,
        then: &'static EffectDef,
    },
    /// Installs a triggered ability that listens from outside every zone.
    InstallTrigger(InstalledTriggerDef),
    /// A static prohibition: no spell or ability an opponent controls can
    /// make this ability's controller sacrifice a permanent.
    CannotBeForcedToSacrifice,
    /// This card costs that much less generic mana to cast. A static ability
    /// that works from the hand, where casting reads it.
    ReduceGenericCostBy(ValueDef),
    /// "Players can't cast spells or play lands with ..." A static
    /// prohibition read while play options are being offered, so a card it
    /// matches is never a legal action rather than a spell that fizzles.
    PlayersCantPlay(&'static ObjectPredicateDef),
    /// A static attack restriction: this creature cannot be declared as an
    /// attacker unless the query matches. The query carries its own controller
    /// relation, so "unless defending player controls an Island" is an
    /// opponent-relative battlefield query rather than a special case.
    CannotAttackUnless(&'static ObjectQueryDef),
    /// A static effect that turns off one landwalk for blocking purposes:
    /// creatures with it can be blocked as though they did not have it. The
    /// keyword is untouched -- anything else reading it still sees it -- so
    /// this is a blocking rule rather than an ability-removing one.
    LandwalkCanBeBlocked(BasicLandType),
    /// Adds a combat phase after the one now ending.
    AdditionalCombatPhase,
    /// Gives each affected player an extra turn after the current one. Extra
    /// turns are queued by the turn engine, so a later-created turn happens
    /// before an earlier-created one.
    TakeExtraTurn {
        player: EffectRecipientDef,
    },
    /// Gives its controller an emblem, an object that sits outside every
    /// zone and does nothing but carry its abilities.
    CreateEmblem {
        emblem: CardDefinitionId,
    },
    /// Turns a double-faced permanent over to its other face.
    Transform {
        object: EffectRecipientDef,
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
    /// A continuous or rules-modifying effect derived live from a static
    /// ability. Its lifetime is the ability's own applicability rather than a
    /// stored duration.
    StaticApply {
        recipient: EffectRecipientDef,
        effect: AppliedEffectDef,
    },
    Apply {
        recipient: EffectRecipientDef,
        effect: AppliedEffectDef,
        duration: ResolvedEffectDurationDef,
    },
    /// A descriptive marker for an effect portion the shared vocabulary does
    /// not yet represent. The surrounding costs, targets, and timing can still
    /// remain declarative; clause coverage records whether and how it executes.
    Special(&'static str),
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

/// A named group of damage sources a turn-long prevention can answer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DamageSourceGroupDef {
    CreaturesWithFlying,
    AttackingCreaturesWithoutFlying,
    Artifacts,
    /// Attacking creatures nothing is blocking. The question is asked as the
    /// damage arrives, so a blocker removed mid-combat changes the answer.
    UnblockedCreatures,
}

/// Turn structure used by beginning/end-of-step trigger declarations.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TurnStepDef {
    Untap,
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
    Cleanup,
}
