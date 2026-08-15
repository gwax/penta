// What a continuous effect applies to the object or player it names.
//
// Separated from the resolving vocabulary next door because the two answer
// different questions: an `EffectDef` says what happens once, while these
// leaves say what stays true. Included textually into `effects.rs`, so the
// paths and imports here are the parent module's.

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

/// A continuous or rules-modifying effect applied to an object or player.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppliedEffectDef {
    /// Components applied to the same recipient for the same duration as one
    /// continuous effect.
    Composite(&'static [AppliedEffectDef]),
    /// One typed operation in the characteristic layer named by the leaf.
    Characteristic(CharacteristicOperationDef),
    /// One prohibition, permission, or prevention rule. Static rules are
    /// derived live from their source; resolving rules are stored with the
    /// authored duration alongside resolved characteristic changes.
    Rule(AppliedRuleDef),
}

/// A continuous rule modification applied to one object or player.
///
/// Keeping these leaves separate from characteristic operations makes their
/// layer-independent nature explicit without giving every printed wording a
/// top-level effect variant.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppliedRuleDef {
    /// The affected creature assigns no combat damage. This is a constraint
    /// on the assignment rather than a shield over the result: an attacker
    /// under it is not asked how to divide its damage at all, so trample has
    /// nothing to spill and no blocker is dealt a lethal share.
    AssignsNoCombatDamage,
    CannotBeCountered,
    /// A creature matching this predicate cannot block the affected creature.
    CannotBeBlockedBy(ObjectPredicateDef),
    /// No Aura may attach to the affected permanent. This restricts both the
    /// Aura spell's targeting and whether an existing attachment stays legal,
    /// so an Aura already on the permanent falls off.
    CannotBeEnchanted,
    /// No new Aura may attach to the affected permanent, but an Aura already
    /// attached remains legal. Guardian Beast needs this narrower prohibition.
    CannotBecomeEnchanted,
    /// The affected creature cannot block at all.
    CannotBlock,
    /// Another player cannot gain control of the affected permanent.
    CannotChangeController,
    /// The affected Aura stays attached even when protection would otherwise
    /// make its host an illegal one. This is the printed exception that lets
    /// an Aura grant protection from its own color without falling off.
    RemainsAttachedThroughProtection,
    /// The affected creature may block only creatures matching this
    /// predicate.
    CanBlockOnly(ObjectPredicateDef),
    /// The affected creature cannot be declared as an attacker.
    CannotAttack,
    /// Defender does not stop the affected creature from attacking.
    ///
    /// A permission rather than an ability removal: the creature keeps the
    /// keyword, so anything reading "a creature with defender" still finds
    /// one. Every other reason it cannot attack still applies.
    MayAttackDespiteDefender,
    /// Nothing can block the affected creature.
    CannotBeBlocked,
    /// Every creature matching this predicate that is able to block the
    /// affected creature must do so.
    ///
    /// A requirement never beats a restriction (CR 509.1c): "able" is read
    /// from the same legality that offers a block in the first place, so a
    /// tapped creature, one that cannot block at all, or one that cannot
    /// block *this* attacker is simply not required. What the requirement
    /// does is take away the alternatives -- a creature that could block the
    /// affected one may not be declared against anything else.
    MustBeBlockedBy(ObjectPredicateDef),
    /// Damage a matching source would deal to the affected permanent's
    /// controller is dealt to that permanent instead. The redirection is read
    /// live, so a condition on the recipient -- "as long as this creature is
    /// untapped" -- turns it off without the permanent being touched.
    RedirectPlayerDamageToThis(DamageSourceGroupDef),
    /// Damage the named source would deal to the affected player is dealt to
    /// the named destination instead. Resolving this rule freezes both object
    /// references for the authored duration.
    RedirectDamageFromTo {
        source: ObjectRefDef,
        destination: ObjectRefDef,
    },
    /// The affected player may untap at most one matching permanent during
    /// their untap step.
    ///
    /// A cap on the turn-based action, not a prohibition on untapping: the
    /// player still chooses which one, and anything that untaps a permanent
    /// outside the untap step is untouched. Several of these compose, each
    /// capping its own group.
    UntapAtMostOne(ObjectPredicateDef),
    /// The affected player cannot take matching cast or land-play actions.
    /// The recipient and lifetime live on `StaticApply` or `Apply`, just as
    /// they do for object-facing applied rules.
    CannotPlay(PlayRestrictionDef),
    /// Regeneration shields can still be created, but cannot replace a
    /// destruction while this rule applies. CR 701.19c.
    CannotRegenerate,
    /// The affected permanent is skipped by its controller's ordinary
    /// turn-based untap procedure. Other spells and abilities can still
    /// untap it.
    DoesNotUntapDuringUntapStep,
    /// The affected permanent's controller may choose to leave it tapped
    /// during their untap step. Unlike
    /// [`Self::DoesNotUntapDuringUntapStep`] this is a choice rather than a
    /// prohibition, so declining is what the printed cards are paying for.
    MayChooseNotToUntap,
    /// Caps matching damage while this rule applies. Unlike
    /// [`Self::PreventDamage`] nothing is spent: every matching event is
    /// limited for as long as the rule is there.
    LimitDamage {
        matcher: DamageEventMatcherDef,
        limit: DamageLimitDef,
    },
    /// An unlimited prevention rule derived live while this static applied
    /// effect exists. Two-sided prevention is an
    /// [`AppliedEffectDef::Composite`] of source and recipient matchers.
    PreventDamage(DamageEventMatcherDef),
}

/// Which kind of play action a restriction matches.
///
/// Keeping this axis separate from the object predicate lets one rule cover
/// both halves of text such as City in a Bottle while a cast-only rule such as
/// Aurelia's Fury leaves land plays untouched.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PlayActionMatcherDef {
    Any,
    CastSpell,
    PlayLand,
}

impl PlayActionMatcherDef {
    #[must_use]
    pub const fn matches(self, action: PlayActionKind) -> bool {
        matches!(self, Self::Any)
            || matches!(
                (self, action),
                (Self::CastSpell, PlayActionKind::CastSpell)
                    | (Self::PlayLand, PlayActionKind::PlayLand)
            )
    }
}

/// A prohibition over one play-action family and one object predicate.
///
/// This deliberately models prohibition rather than a per-turn quota. A
/// future Deafening Silence-style limit can share these two match axes, but
/// also needs matching cast history rather than being approximated as a
/// boolean prohibition.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PlayRestrictionDef {
    pub action: PlayActionMatcherDef,
    pub object: ObjectPredicateDef,
}

impl PlayRestrictionDef {
    #[must_use]
    pub const fn new(action: PlayActionMatcherDef, object: ObjectPredicateDef) -> Self {
        Self { action, object }
    }
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
        Self::Rule(AppliedRuleDef::PreventDamage(
            DamageEventMatcherDef::from_matching_to_affected(source),
        ))
    }

    #[must_use]
    pub const fn prevent_combat_damage_from(source: ObjectPredicateDef) -> Self {
        Self::Rule(AppliedRuleDef::PreventDamage(DamageEventMatcherDef {
            kind: DamageKindDef::Combat,
            source: DamageSourceMatcherDef::Matching(source),
            recipient: DamageRecipientMatcherDef::AffectedObject,
        }))
    }
}
