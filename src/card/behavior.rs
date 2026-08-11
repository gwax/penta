#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CardBehavior {
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative blocking restriction and damage prevention.
    ArgothianPixies,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative sacrifice cost and pump.
    Atog,
    AugurOfBolas,
    Balance,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// pumps declaratively and schedules its own delayed destruction.
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative spell clause and a play-time cast restriction.
    Berserk,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// records its chosen opponent and triggers on the shared stack.
    BlackVise,
    BloodBaronOfVizkopa,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative choose-one spell definition.
    BlueElementalBlast,
    BloodMoon,
    ChainLightning,
    Channel,
    ChaosOrb,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative state trigger.
    CityInABottle,
    CopyArtifact,
    Crusade,
    DemonicTutor,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative predicate, destroy, and damage.
    Detonate,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// destroys and reads last-known mana value declaratively.
    DivineOffering,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// targets and counters instant spells declaratively.
    Dispel,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses the shared counter-to-exile effect.
    Dissipate,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative nonblack-creature predicate and destroy.
    DoomBlade,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative drain that respects the life-gain cap, and a
    /// cost-level restriction on which mana may pay for X.
    DrainLife,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative pump and a conditional delayed sacrifice.
    DragonWhelp,
    DustToDust,
    Duress,
    Earthquake,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative targeted upkeep trigger.
    ErhnamDjinn,
    EssenceScatter,
    Fireball,
    Fork,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative until-end-of-turn pump.
    GiantGrowth,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative look-at-hand ability.
    GlassesOfUrza,
    GoblinGrenade,
    GrislySalvage,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// sweeps by ownership declaratively.
    HurkylsRecall,
    HymnToTourach,
    HypnoticSpecter,
    IcyManipulator,
    IronclawOrcs,
    KirdApe,
    LifebaneZombie,
    LibraryOfAlexandria,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// counters declaratively and schedules its own mana.
    ManaDrain,
    ManaVault,
    /// Mana Vault's upkeep trigger, keyed apart from the untap restriction so
    /// the two clauses resolve as the separate abilities they are.
    ManaVaultUntap,
    /// Mana Vault's draw-step damage trigger.
    ManaVaultDamage,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses declarative untap and combat-damage prevention.
    MazeOfIth,
    MindTwist,
    Moat,
    Mulch,
    Negate,
    NevinyrralsDisk,
    Pendelhaven,
    PillarOfFlame,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative artifact-or-creature predicate and destroy.
    Putrefy,
    Recall,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// targets the graveyard card it returns.
    Regrowth,
    SedgeTroll,
    SinCollector,
    SylvanLibrary,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative predicate and a no-regeneration destroy.
    Terror,
    TimeVault,
    Timetwister,
    FellwarStone,
    LightningBolt,
    MishrasFactory,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative choose-one spell definition.
    RedElementalBlast,
    Smoke,
    SphinxsRevelation,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative grant and a delayed destruction.
    StoneGiant,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative creature-sweeper definition.
    SupremeVerdict,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// exiles and reads last-known power declaratively.
    SwordsToPlowshares,
    TimeWalk,
    Tetravus,
    /// Tetravus's first upkeep trigger: trade +1/+1 counters for Tetravites.
    TetravusDetach,
    /// Tetravus's second upkeep trigger: exile its own Tetravites to take the
    /// counters back.
    TetravusAssemble,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative upkeep trigger with a chosen destruction.
    TheAbyss,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses the shared exact-color-count predicate and destroy.
    UltimatePrice,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// sequences shared damage and life-gain effects.
    WarleadersHelix,
    WheelOfFortune,
    /// Legacy dispatch key retained for source compatibility; the card now
    /// uses a declarative end-step trigger with an intervening-if.
    WhirlingDervish,
    WinterOrb,
    // Compatibility rules keys retained while CardDefinition::new still
    // accepts CardBehavior instead of CardRules directly.
    Mountain,
    Plains,
    Unsupported,
}

use super::{
    CardRules, CardSupertype, CardTypeSet, ColorSet, CreatureStats, KeywordAbility, ManaColor,
    ManaCost,
};
use crate::card::sets;

impl CardBehavior {
    /// Returns all declarative rules metadata for this card behavior.
    #[must_use]
    pub const fn rules(self) -> &'static CardRules {
        sets::rules(self)
    }

    #[must_use]
    pub const fn is_legendary(self) -> bool {
        self.rules().has_supertype(CardSupertype::Legendary)
    }

    #[must_use]
    pub fn rules_text(self) -> std::borrow::Cow<'static, str> {
        self.rules().rules_text()
    }

    #[must_use]
    pub const fn types(self) -> CardTypeSet {
        self.rules().types()
    }

    #[must_use]
    pub const fn mana_cost(self) -> Option<ManaCost> {
        self.rules().mana_cost()
    }

    #[must_use]
    pub const fn creature_stats(self) -> Option<CreatureStats> {
        self.rules().creature_stats()
    }

    #[must_use]
    pub fn is_goblin(self) -> bool {
        self.rules().has_subtype("Goblin")
    }

    #[must_use]
    pub fn has_flying(self) -> bool {
        self.rules().has_executable_keyword(KeywordAbility::Flying)
    }

    #[must_use]
    pub fn has_mountainwalk(self) -> bool {
        self.rules()
            .has_executable_keyword(KeywordAbility::Mountainwalk)
    }

    /// Returns the object's printed color-characteristic set.
    #[must_use]
    pub const fn colors(self) -> ColorSet {
        self.rules().color_set()
    }

    #[must_use]
    pub const fn is_red(self) -> bool {
        self.rules().has_color(ManaColor::Red)
    }

    #[must_use]
    pub const fn is_blue(self) -> bool {
        self.rules().has_color(ManaColor::Blue)
    }

    #[must_use]
    pub const fn is_white(self) -> bool {
        self.rules().has_color(ManaColor::White)
    }

    #[must_use]
    pub const fn is_black(self) -> bool {
        self.rules().has_color(ManaColor::Black)
    }

    #[must_use]
    pub const fn is_green(self) -> bool {
        self.rules().has_color(ManaColor::Green)
    }

    #[must_use]
    pub fn has_vigilance(self) -> bool {
        self.rules()
            .has_executable_keyword(KeywordAbility::Vigilance)
    }
}
