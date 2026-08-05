use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

use crate::CardDefinitionId;
use crate::rules;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CardSet {
    Alpha,
    Beta,
    Unlimited,
    CollectorsEdition,
    InternationalCollectorsEdition,
    ArabianNights,
    Antiquities,
    Revised,
    Legends,
    TheDark,
    FallenEmpires,
    Promo1994,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardDefinition {
    pub id: CardDefinitionId,
    pub name: String,
    pub set: CardSet,
    pub is_basic_land: bool,
    pub behavior: CardBehavior,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CardBehavior {
    AncestralRecall,
    AnkhOfMishra,
    ArgothianPixies,
    Armageddon,
    Atog,
    Badlands,
    BallLightning,
    Balance,
    Berserk,
    Bayou,
    BlackLotus,
    BlackKnight,
    BlackVise,
    BirdsOfParadise,
    BlueElementalBlast,
    Braingeyser,
    BloodMoon,
    ChainLightning,
    Channel,
    ChaosOrb,
    CityOfBrass,
    CityInABottle,
    CopperTablet,
    CopyArtifact,
    Counterspell,
    Crusade,
    DarkRitual,
    DemonicTutor,
    Detonate,
    DivineOffering,
    DrainLife,
    DragonWhelp,
    Disenchant,
    DustToDust,
    EnergyFlux,
    Earthquake,
    ErhnamDjinn,
    Forest,
    Fireball,
    Fork,
    GiantGrowth,
    GlassesOfUrza,
    GoblinBalloonBrigade,
    GoblinDiggingTeam,
    GoblinGrenade,
    GoblinKing,
    GoblinsOfTheFlarg,
    GraniteGargoyle,
    HurkylsRecall,
    HymnToTourach,
    HypnoticSpecter,
    IcyManipulator,
    IcatianJavelineers,
    IronStar,
    IronclawOrcs,
    Island,
    IvoryTower,
    JayemdaeTome,
    Juggernaut,
    JuzamDjinn,
    KirdApe,
    LlanowarElves,
    LibraryOfAlexandria,
    ManaDrain,
    ManaVault,
    MazeOfIth,
    MindTwist,
    MishrasWorkshop,
    Moat,
    NevinyrralsDisk,
    OrderOfLeitbur,
    OrderOfTheEbonHand,
    Pendelhaven,
    Plateau,
    PsionicBlast,
    Recall,
    Regrowth,
    RelicBarrier,
    SageOfLatNam,
    Savannah,
    SavannahLions,
    Scrubland,
    SerendibEfreet,
    SedgeTroll,
    SengirVampire,
    ScrybSprites,
    Sinkhole,
    StoneRain,
    Swamp,
    SylvanLibrary,
    Taiga,
    Terror,
    ThunderSpirit,
    TimeVault,
    Timetwister,
    TropicalIsland,
    UndergroundSea,
    VolcanicIsland,
    FellwarStone,
    Mountain,
    LightningBolt,
    MishrasFactory,
    MoxEmerald,
    MoxJet,
    MoxPearl,
    MoxRuby,
    MoxSapphire,
    OrcishMechanics,
    Plains,
    RedElementalBlast,
    Shatter,
    Smoke,
    SolRing,
    SerraAngel,
    StoneGiant,
    StripMine,
    SuChi,
    SwordsToPlowshares,
    TimeWalk,
    Tundra,
    Triskelion,
    Tetravus,
    TheAbyss,
    WheelOfFortune,
    WhirlingDervish,
    WhiteKnight,
    WinterOrb,
    WrathOfGod,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CardKind {
    Land,
    Creature,
    Artifact,
    ArtifactCreature,
    Enchantment,
    Instant,
    Sorcery,
}

impl CardKind {
    #[must_use]
    pub const fn is_creature(self) -> bool {
        matches!(self, Self::Creature | Self::ArtifactCreature)
    }

    #[must_use]
    pub const fn is_artifact(self) -> bool {
        matches!(self, Self::Artifact | Self::ArtifactCreature)
    }

    #[must_use]
    pub const fn is_permanent(self) -> bool {
        matches!(
            self,
            Self::Land
                | Self::Creature
                | Self::Artifact
                | Self::ArtifactCreature
                | Self::Enchantment
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ManaCost {
    pub generic: u16,
    pub white: u16,
    pub blue: u16,
    pub black: u16,
    pub red: u16,
    pub green: u16,
    pub variable_x: bool,
    pub x_multiplier: u16,
}

impl ManaCost {
    #[must_use]
    pub const fn new(generic: u16, red: u16) -> Self {
        Self {
            generic,
            white: 0,
            blue: 0,
            black: 0,
            red,
            green: 0,
            variable_x: false,
            x_multiplier: 0,
        }
    }

    #[must_use]
    pub const fn colored(
        generic: u16,
        white: u16,
        blue: u16,
        black: u16,
        red: u16,
        green: u16,
    ) -> Self {
        Self {
            generic,
            white,
            blue,
            black,
            red,
            green,
            variable_x: false,
            x_multiplier: 0,
        }
    }

    #[must_use]
    pub const fn with_x(red: u16) -> Self {
        Self {
            generic: 0,
            white: 0,
            blue: 0,
            black: 0,
            red,
            green: 0,
            variable_x: true,
            x_multiplier: 1,
        }
    }

    #[must_use]
    pub const fn colored_x(white: u16, blue: u16, black: u16, red: u16, green: u16) -> Self {
        Self {
            generic: 0,
            white,
            blue,
            black,
            red,
            green,
            variable_x: true,
            x_multiplier: 1,
        }
    }

    #[must_use]
    pub const fn variable(
        generic: u16,
        white: u16,
        blue: u16,
        black: u16,
        red: u16,
        green: u16,
        x_multiplier: u16,
    ) -> Self {
        Self {
            generic,
            white,
            blue,
            black,
            red,
            green,
            variable_x: true,
            x_multiplier,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CreatureStats {
    pub power: i16,
    pub toughness: i16,
    pub haste: bool,
    pub trample: bool,
}

impl CardBehavior {
    /// The legendary permanents in the pool, for the legend rule.
    #[must_use]
    pub const fn is_legendary(self) -> bool {
        matches!(self, Self::Pendelhaven | Self::LibraryOfAlexandria)
    }

    /// Returns concise rules text for the behavior implemented by the simulator.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub const fn rules_text(self) -> &'static str {
        match self {
            Self::AncestralRecall => "Target player draws three cards.",
            Self::AnkhOfMishra => {
                "Whenever a land enters, Ankh of Mishra deals 2 damage to its controller."
            }
            Self::ArgothianPixies => "Argothian Pixies can't be blocked by artifact creatures.",
            Self::Armageddon => "Destroy all lands.",
            Self::Atog => "Sacrifice an artifact: Atog gets +2/+2 until end of turn.",
            Self::BallLightning => {
                "Trample, haste. Sacrifice Ball Lightning at the beginning of the end step."
            }
            Self::Badlands => "Tap: Add B or R.",
            Self::Balance => {
                "Each player discards and sacrifices creatures and lands until tied for the fewest of each."
            }
            Self::Berserk => {
                "Target creature gains trample and gets +X/+0 until end of turn, where X is its power. Destroy it at end of turn if it attacked this turn."
            }
            Self::Bayou => "Tap: Add B or G.",
            Self::BlackLotus => "Tap, sacrifice Black Lotus: Add RRR.",
            Self::BlackVise => {
                "As Black Vise enters, choose an opponent. At their upkeep, it deals 1 damage for each card in their hand beyond four."
            }
            Self::BlackKnight => "First strike, protection from white.",
            Self::BirdsOfParadise => "Flying. Tap: Add one mana of any color.",
            Self::BlueElementalBlast => "Counter target red spell or destroy target red permanent.",
            Self::Braingeyser => "Target player draws X cards.",
            Self::BloodMoon => "Nonbasic lands are Mountains.",
            Self::ChainLightning => {
                "Deal 3 damage to any target. That target's controller may pay RR to copy it and choose a new target."
            }
            Self::Channel => "Until end of turn, you may pay 1 life to add one colorless mana.",
            Self::ChaosOrb => {
                "1, Tap: Choose a permanent. On resolution, destroy it and Chaos Orb if Chaos Orb is still on the battlefield."
            }
            Self::CopperTablet => {
                "At the beginning of each player's upkeep, Copper Tablet deals 1 damage to that player."
            }
            Self::CityOfBrass => {
                "Whenever City of Brass becomes tapped, it deals 1 damage to you. Tap: Add one mana of any color."
            }
            Self::CityInABottle => {
                "At the beginning of each upkeep, destroy each other permanent from Arabian Nights."
            }
            Self::CopyArtifact => {
                "You may have Copy Artifact enter as a copy of any artifact on the battlefield."
            }
            Self::Counterspell => "Counter target spell.",
            Self::Crusade => "White creatures get +1/+1.",
            Self::DarkRitual => "Add BBB.",
            Self::DemonicTutor => {
                "Search your library for a card, put it into your hand, then shuffle."
            }
            Self::Detonate => {
                "Destroy target artifact with mana value X. Its controller takes X damage."
            }
            Self::DivineOffering => {
                "Destroy target artifact. You gain life equal to its mana value."
            }
            Self::DrainLife => {
                "Drain Life deals X damage to any target and you gain that much life."
            }
            Self::DragonWhelp => {
                "Flying. R: +1/+0 until end of turn. If activated four or more times this turn, destroy it at the end step."
            }
            Self::Disenchant => "Destroy target artifact or enchantment.",
            Self::DustToDust => "Exile two target artifacts.",
            Self::EnergyFlux => {
                "At the beginning of each player's upkeep, sacrifice each artifact unless you pay 2 for it."
            }
            Self::Earthquake => {
                "Earthquake deals X damage to each player and each creature without flying."
            }
            Self::ErhnamDjinn => {
                "At your upkeep, target opponent's creature gains forestwalk until your next upkeep."
            }
            Self::Forest | Self::LlanowarElves => "Tap: Add G.",
            Self::Fireball => {
                "Deal X damage divided evenly among the chosen targets. Each target beyond the first costs 1 more."
            }
            Self::Fork => {
                "Copy target instant or sorcery. You may choose new targets for the copy."
            }
            Self::GiantGrowth => "Target creature gets +3/+3 until end of turn.",
            Self::GlassesOfUrza => "Tap: Look at target player's hand.",
            Self::GoblinBalloonBrigade => "R: Gains flying until end of turn.",
            Self::GoblinDiggingTeam => "Sacrifice Goblin Digging Team: Destroy target Wall.",
            Self::GoblinGrenade => {
                "As an additional cost, sacrifice a Goblin. Deal 5 damage to any target."
            }
            Self::GoblinKing => "Other Goblins get +1/+1 and have mountainwalk.",
            Self::GoblinsOfTheFlarg => "Mountainwalk.",
            Self::GraniteGargoyle => "Flying. R: Gets +0/+1 until end of turn.",
            Self::HurkylsRecall => {
                "Return all artifacts target player controls to their owner's hand."
            }
            Self::HymnToTourach => "Target player discards two cards at random.",
            Self::HypnoticSpecter => {
                "Flying. Whenever Hypnotic Specter damages an opponent, they discard a card at random."
            }
            Self::IcyManipulator => "1, Tap: Tap target artifact, creature, or land.",
            Self::IcatianJavelineers => {
                "Enters with a javelin counter. Tap, remove it: Deal 1 damage to any target."
            }
            Self::IronStar => {
                "Whenever a red spell is cast, you may pay 1. If you do, gain 1 life."
            }
            Self::IronclawOrcs => "Can't block creatures with power 2 or greater.",
            Self::Island => "Tap: Add U.",
            Self::IvoryTower => {
                "At the beginning of your upkeep, gain 1 life for each card in your hand beyond four."
            }
            Self::JayemdaeTome => "4, Tap: Draw a card.",
            Self::Juggernaut => {
                "Attacks each combat if able. Juggernaut can't be blocked by Walls."
            }
            Self::JuzamDjinn => "At your upkeep, Juzam Djinn deals 1 damage to you.",
            Self::KirdApe => "Kird Ape gets +1/+2 as long as you control a Forest.",
            Self::LibraryOfAlexandria => {
                "Tap: Add 1. Tap: Draw a card. Activate only with exactly seven cards in hand."
            }
            Self::ManaDrain => {
                "Counter target spell. At your next main phase, add colorless mana equal to its mana value."
            }
            Self::ManaVault => {
                "Mana Vault doesn't untap during your untap step. At your upkeep, you may pay 4 to untap it. At your draw step, if tapped, it deals 1 damage to you. Tap: Add 3."
            }
            Self::MazeOfIth => {
                "Tap: Untap target attacking creature and prevent all combat damage it would deal and receive this turn."
            }
            Self::MindTwist => "Target player discards X cards at random.",
            Self::MishrasWorkshop => "Tap: Add 3. Spend this mana only to cast artifact spells.",
            Self::Moat => "Creatures without flying can't attack.",
            Self::NevinyrralsDisk => {
                "Enters tapped. 1, Tap: Destroy all artifacts, creatures, and enchantments."
            }
            Self::OrderOfLeitbur => {
                "Protection from black. WW: Gets +1/+0 until end of turn. W: Gains first strike until end of turn."
            }
            Self::OrderOfTheEbonHand => {
                "Protection from white. BB: Gets +1/+0 until end of turn. B: Gains first strike until end of turn."
            }
            Self::Pendelhaven => {
                "Tap: Add G. Tap: Target 1/1 creature gets +1/+2 until end of turn."
            }
            Self::Plateau => "Tap: Add R or W.",
            Self::PsionicBlast => "Deal 4 damage to any target and 2 damage to you.",
            Self::Recall => {
                "Discard X cards, then return X cards from your graveyard to your hand. Exile Recall."
            }
            Self::Regrowth => "Return target card from your graveyard to your hand.",
            Self::RelicBarrier => "Tap: Tap target artifact.",
            Self::SageOfLatNam => "Tap, sacrifice an artifact: Draw a card.",
            Self::Savannah => "Tap: Add G or W.",
            Self::SavannahLions => "A swift 2/1 creature.",
            Self::Scrubland => "Tap: Add W or B.",
            Self::SerendibEfreet => {
                "Flying. At your upkeep, Serendib Efreet deals 1 damage to you."
            }
            Self::SedgeTroll => {
                "Sedge Troll gets +1/+1 as long as you control a Swamp. R: Regenerate Sedge Troll."
            }
            Self::SengirVampire => {
                "Flying. Whenever a creature damaged by Sengir Vampire dies, put a +1/+1 counter on it."
            }
            Self::ScrybSprites => "Flying.",
            Self::Sinkhole | Self::StoneRain => "Destroy target land.",
            Self::Swamp => "Tap: Add B.",
            Self::SylvanLibrary => {
                "At your draw step, draw two additional cards, then put two cards drawn this turn back unless you pay 4 life for each."
            }
            Self::Taiga => "Tap: Add R or G.",
            Self::Terror => {
                "Destroy target nonartifact, nonblack creature. It can't be regenerated."
            }
            Self::ThunderSpirit => "Flying, first strike.",
            Self::TimeVault => {
                "Enters tapped and doesn't untap normally. Skip a turn to untap it. Tap: Take an extra turn."
            }
            Self::Timetwister => {
                "Each player shuffles their hand and graveyard into their library, then draws seven cards."
            }
            Self::TropicalIsland => "Tap: Add U or G.",
            Self::UndergroundSea => "Tap: Add U or B.",
            Self::VolcanicIsland => "Tap: Add U or R.",
            Self::FellwarStone => {
                "Tap: Add one mana of any color an opponent's land could produce."
            }
            Self::Mountain | Self::MoxRuby => "Tap: Add R.",
            Self::LightningBolt => "Deal 3 damage to any target.",
            Self::MishrasFactory => {
                "Tap: Add 1. 1: Becomes a 2/2 Assembly-Worker artifact creature until end of turn. Tap: Target Assembly-Worker gets +1/+1 until end of turn."
            }
            Self::MoxEmerald | Self::MoxJet | Self::MoxPearl | Self::MoxSapphire => "Tap: Add 1.",
            Self::OrcishMechanics => "Tap, sacrifice an artifact: Deal 2 damage to any target.",
            Self::Plains => "Tap: Add W.",
            Self::RedElementalBlast => {
                "Counter target blue spell or destroy target blue permanent."
            }
            Self::Shatter => "Destroy target artifact.",
            Self::Smoke => "Players can't untap more than one creature during their untap steps.",
            Self::SolRing => "Tap: Add 2.",
            Self::SerraAngel => "Flying, vigilance.",
            Self::StoneGiant => {
                "Tap: A smaller creature you control gains flying until end of turn. Destroy it at the end step."
            }
            Self::StripMine => "Tap, sacrifice Strip Mine: Destroy target land.",
            Self::SuChi => "When Su-Chi dies, add 4.",
            Self::SwordsToPlowshares => {
                "Exile target creature. Its controller gains life equal to its power."
            }
            Self::TimeWalk => "Take an extra turn after this one.",
            Self::Tundra => "Tap: Add W or U.",
            Self::Triskelion => {
                "Enters with three +1/+1 counters. Remove a +1/+1 counter: Deal 1 damage to any target."
            }
            Self::Tetravus => "Flying. Tetravus enters with three +1/+1 counters on it.",
            Self::TheAbyss => {
                "At the beginning of each upkeep, destroy target nonartifact creature."
            }
            Self::WheelOfFortune => "Each player discards their hand, then draws seven cards.",
            Self::WhirlingDervish => {
                "Protection from black. At each end step, if it damaged an opponent this turn, put a +1/+1 counter on it."
            }
            Self::WhiteKnight => "First strike, protection from black.",
            Self::WinterOrb => {
                "While untapped, players can't untap more than one land during their untap steps."
            }
            Self::WrathOfGod => "Destroy all creatures. They can't be regenerated.",
            Self::Unsupported => "Rules text is not implemented.",
        }
    }

    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub const fn kind(self) -> CardKind {
        match self {
            Self::Badlands
            | Self::Bayou
            | Self::CityOfBrass
            | Self::Pendelhaven
            | Self::Forest
            | Self::Island
            | Self::LibraryOfAlexandria
            | Self::MazeOfIth
            | Self::Mountain
            | Self::MishrasFactory
            | Self::MishrasWorkshop
            | Self::Plateau
            | Self::Plains
            | Self::Savannah
            | Self::Scrubland
            | Self::StripMine
            | Self::Swamp
            | Self::Taiga
            | Self::Tundra
            | Self::TropicalIsland
            | Self::UndergroundSea
            | Self::VolcanicIsland => CardKind::Land,
            Self::Atog
            | Self::ArgothianPixies
            | Self::BallLightning
            | Self::BirdsOfParadise
            | Self::BlackKnight
            | Self::DragonWhelp
            | Self::ErhnamDjinn
            | Self::GoblinBalloonBrigade
            | Self::GoblinDiggingTeam
            | Self::GoblinKing
            | Self::GoblinsOfTheFlarg
            | Self::GraniteGargoyle
            | Self::HypnoticSpecter
            | Self::IcatianJavelineers
            | Self::IronclawOrcs
            | Self::JuzamDjinn
            | Self::KirdApe
            | Self::LlanowarElves
            | Self::OrcishMechanics
            | Self::OrderOfLeitbur
            | Self::OrderOfTheEbonHand
            | Self::SavannahLions
            | Self::SerendibEfreet
            | Self::SedgeTroll
            | Self::SengirVampire
            | Self::SerraAngel
            | Self::ScrybSprites
            | Self::StoneGiant
            | Self::ThunderSpirit
            | Self::WhirlingDervish
            | Self::WhiteKnight => CardKind::Creature,
            Self::Juggernaut
            | Self::SageOfLatNam
            | Self::SuChi
            | Self::Tetravus
            | Self::Triskelion => CardKind::ArtifactCreature,
            Self::AnkhOfMishra
            | Self::BlackLotus
            | Self::BlackVise
            | Self::ChaosOrb
            | Self::CityInABottle
            | Self::CopperTablet
            | Self::GlassesOfUrza
            | Self::IcyManipulator
            | Self::IronStar
            | Self::IvoryTower
            | Self::JayemdaeTome
            | Self::ManaVault
            | Self::MoxEmerald
            | Self::MoxJet
            | Self::MoxPearl
            | Self::MoxRuby
            | Self::MoxSapphire
            | Self::RelicBarrier
            | Self::SolRing
            | Self::FellwarStone
            | Self::NevinyrralsDisk
            | Self::TimeVault
            | Self::WinterOrb
            | Self::Unsupported => CardKind::Artifact,
            Self::BloodMoon
            | Self::Crusade
            | Self::CopyArtifact
            | Self::EnergyFlux
            | Self::Moat
            | Self::Smoke
            | Self::SylvanLibrary
            | Self::TheAbyss => CardKind::Enchantment,
            Self::AncestralRecall
            | Self::BlueElementalBlast
            | Self::Berserk
            | Self::Counterspell
            | Self::DarkRitual
            | Self::Disenchant
            | Self::DivineOffering
            | Self::GiantGrowth
            | Self::Fork
            | Self::HurkylsRecall
            | Self::LightningBolt
            | Self::ManaDrain
            | Self::PsionicBlast
            | Self::RedElementalBlast
            | Self::Shatter
            | Self::SwordsToPlowshares
            | Self::Terror => CardKind::Instant,
            Self::Armageddon
            | Self::Balance
            | Self::Braingeyser
            | Self::ChainLightning
            | Self::Channel
            | Self::DemonicTutor
            | Self::Detonate
            | Self::DrainLife
            | Self::DustToDust
            | Self::Earthquake
            | Self::Fireball
            | Self::GoblinGrenade
            | Self::HymnToTourach
            | Self::MindTwist
            | Self::Recall
            | Self::Regrowth
            | Self::Sinkhole
            | Self::StoneRain
            | Self::TimeWalk
            | Self::Timetwister
            | Self::WheelOfFortune
            | Self::WrathOfGod => CardKind::Sorcery,
        }
    }

    #[must_use]
    #[allow(clippy::match_same_arms, clippy::too_many_lines)]
    pub const fn mana_cost(self) -> ManaCost {
        match self {
            Self::Badlands
            | Self::Bayou
            | Self::CityOfBrass
            | Self::Pendelhaven
            | Self::Forest
            | Self::Mountain
            | Self::Island
            | Self::LibraryOfAlexandria
            | Self::MazeOfIth
            | Self::Plains
            | Self::Plateau
            | Self::Savannah
            | Self::Scrubland
            | Self::Swamp
            | Self::Taiga
            | Self::Tundra
            | Self::TropicalIsland
            | Self::UndergroundSea
            | Self::VolcanicIsland
            | Self::MishrasFactory
            | Self::MishrasWorkshop
            | Self::StripMine
            | Self::BlackLotus
            | Self::MoxEmerald
            | Self::MoxJet
            | Self::MoxPearl
            | Self::MoxRuby
            | Self::MoxSapphire => ManaCost::new(0, 0),
            Self::AncestralRecall | Self::BlueElementalBlast => ManaCost::colored(0, 0, 1, 0, 0, 0),
            Self::Berserk | Self::GiantGrowth => ManaCost::colored(0, 0, 0, 0, 0, 1),
            Self::IcatianJavelineers | Self::SavannahLions | Self::SwordsToPlowshares => {
                ManaCost::colored(0, 1, 0, 0, 0, 0)
            }
            Self::KirdApe => ManaCost::new(0, 1),
            Self::LlanowarElves | Self::ScrybSprites => ManaCost::colored(0, 0, 0, 0, 0, 1),
            Self::ArgothianPixies => ManaCost::colored(1, 0, 0, 0, 0, 1),
            Self::BirdsOfParadise => ManaCost::colored(0, 0, 0, 0, 0, 1),
            Self::DarkRitual => ManaCost::colored(0, 0, 0, 1, 0, 0),
            Self::Counterspell | Self::ManaDrain => ManaCost::colored(0, 0, 2, 0, 0, 0),
            Self::Balance | Self::Disenchant | Self::DivineOffering => {
                ManaCost::colored(1, 1, 0, 0, 0, 0)
            }
            Self::DemonicTutor | Self::Terror => ManaCost::colored(1, 0, 0, 1, 0, 0),
            Self::HurkylsRecall => ManaCost::colored(1, 0, 1, 0, 0, 0),
            Self::Regrowth | Self::SylvanLibrary => ManaCost::colored(1, 0, 0, 0, 0, 1),
            Self::AnkhOfMishra
            | Self::ChaosOrb
            | Self::CopperTablet
            | Self::FellwarStone
            | Self::RelicBarrier
            | Self::TimeVault
            | Self::WinterOrb => ManaCost::new(2, 0),
            Self::CityInABottle => ManaCost::new(2, 0),
            Self::CopyArtifact => ManaCost::colored(1, 0, 1, 0, 0, 0),
            Self::EnergyFlux => ManaCost::colored(2, 0, 1, 0, 0, 0),
            Self::IcyManipulator
            | Self::JayemdaeTome
            | Self::Juggernaut
            | Self::NevinyrralsDisk
            | Self::SuChi => ManaCost::new(4, 0),
            Self::SageOfLatNam => ManaCost::colored(1, 0, 1, 0, 0, 0),
            Self::Tetravus => ManaCost::new(6, 0),
            Self::TheAbyss => ManaCost::colored(3, 0, 0, 1, 0, 0),
            Self::SolRing
            | Self::BlackVise
            | Self::GlassesOfUrza
            | Self::IronStar
            | Self::IvoryTower
            | Self::ManaVault => ManaCost::new(1, 0),
            Self::Smoke | Self::Fork => ManaCost::new(0, 2),
            Self::StoneRain => ManaCost::new(2, 1),
            Self::SedgeTroll => ManaCost::new(2, 1),
            Self::Atog | Self::IronclawOrcs | Self::OrcishMechanics | Self::Shatter => {
                ManaCost::new(1, 1)
            }
            Self::GoblinKing => ManaCost::new(1, 2),
            Self::GraniteGargoyle | Self::BloodMoon | Self::WheelOfFortune => ManaCost::new(2, 1),
            Self::DragonWhelp | Self::StoneGiant => ManaCost::new(2, 2),
            Self::BallLightning => ManaCost::new(0, 3),
            Self::ChainLightning
            | Self::GoblinBalloonBrigade
            | Self::GoblinDiggingTeam
            | Self::GoblinGrenade
            | Self::GoblinsOfTheFlarg
            | Self::LightningBolt
            | Self::RedElementalBlast => ManaCost::new(0, 1),
            Self::Detonate | Self::Earthquake | Self::Fireball => ManaCost::with_x(1),
            Self::Braingeyser => ManaCost::colored_x(0, 2, 0, 0, 0),
            Self::DrainLife => ManaCost::variable(1, 0, 0, 1, 0, 0, 1),
            Self::MindTwist => ManaCost::variable(0, 0, 0, 1, 0, 0, 1),
            Self::Recall => ManaCost::variable(0, 0, 1, 0, 0, 0, 2),
            Self::TimeWalk => ManaCost::colored(1, 0, 1, 0, 0, 0),
            Self::Armageddon => ManaCost::colored(3, 1, 0, 0, 0, 0),
            Self::DustToDust | Self::WrathOfGod => ManaCost::colored(2, 2, 0, 0, 0, 0),
            Self::Moat => ManaCost::colored(2, 2, 0, 0, 0, 0),
            Self::Crusade | Self::OrderOfLeitbur | Self::WhiteKnight => {
                ManaCost::colored(0, 2, 0, 0, 0, 0)
            }
            Self::Channel | Self::WhirlingDervish => ManaCost::colored(0, 0, 0, 0, 0, 2),
            Self::BlackKnight | Self::HymnToTourach | Self::OrderOfTheEbonHand | Self::Sinkhole => {
                ManaCost::colored(0, 0, 0, 2, 0, 0)
            }
            Self::HypnoticSpecter => ManaCost::colored(1, 0, 0, 2, 0, 0),
            Self::JuzamDjinn => ManaCost::colored(2, 0, 0, 2, 0, 0),
            Self::SengirVampire => ManaCost::colored(3, 0, 0, 2, 0, 0),
            Self::ErhnamDjinn => ManaCost::colored(3, 0, 0, 0, 0, 1),
            Self::PsionicBlast | Self::SerendibEfreet | Self::Timetwister => {
                ManaCost::colored(2, 0, 1, 0, 0, 0)
            }
            Self::ThunderSpirit => ManaCost::colored(1, 2, 0, 0, 0, 0),
            Self::SerraAngel => ManaCost::colored(3, 2, 0, 0, 0, 0),
            Self::Triskelion => ManaCost::new(6, 0),
            Self::Unsupported => ManaCost::new(u16::MAX, u16::MAX),
        }
    }

    #[must_use]
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::match_same_arms)]
    pub const fn creature_stats(self) -> Option<CreatureStats> {
        match self {
            Self::BirdsOfParadise => Some(CreatureStats {
                power: 0,
                toughness: 1,
                haste: false,
                trample: false,
            }),
            Self::Atog => Some(CreatureStats {
                power: 1,
                toughness: 2,
                haste: false,
                trample: false,
            }),
            Self::ArgothianPixies => Some(CreatureStats {
                power: 2,
                toughness: 1,
                haste: false,
                trample: false,
            }),
            Self::BallLightning => Some(CreatureStats {
                power: 6,
                toughness: 1,
                haste: true,
                trample: true,
            }),
            Self::DragonWhelp => Some(CreatureStats {
                power: 2,
                toughness: 3,
                haste: false,
                trample: false,
            }),
            Self::KirdApe | Self::LlanowarElves | Self::ScrybSprites => Some(CreatureStats {
                power: 1,
                toughness: 1,
                haste: false,
                trample: false,
            }),
            Self::GoblinBalloonBrigade
            | Self::GoblinDiggingTeam
            | Self::GoblinsOfTheFlarg
            | Self::IcatianJavelineers
            | Self::OrcishMechanics
            | Self::Triskelion => Some(CreatureStats {
                power: 1,
                toughness: 1,
                haste: false,
                trample: false,
            }),
            Self::BlackKnight
            | Self::GoblinKing
            | Self::GraniteGargoyle
            | Self::IronclawOrcs
            | Self::OrderOfLeitbur
            | Self::HypnoticSpecter
            | Self::ThunderSpirit
            | Self::WhirlingDervish
            | Self::WhiteKnight => Some(CreatureStats {
                power: 2,
                toughness: 2,
                haste: false,
                trample: false,
            }),
            Self::OrderOfTheEbonHand | Self::SavannahLions => Some(CreatureStats {
                power: 2,
                toughness: 1,
                haste: false,
                trample: false,
            }),
            Self::SerendibEfreet | Self::StoneGiant => Some(CreatureStats {
                power: 3,
                toughness: 4,
                haste: false,
                trample: false,
            }),
            Self::SedgeTroll => Some(CreatureStats {
                power: 2,
                toughness: 2,
                haste: false,
                trample: false,
            }),
            Self::ErhnamDjinn => Some(CreatureStats {
                power: 4,
                toughness: 5,
                haste: false,
                trample: false,
            }),
            Self::SengirVampire | Self::SerraAngel | Self::SuChi => Some(CreatureStats {
                power: 4,
                toughness: 4,
                haste: false,
                trample: false,
            }),
            Self::JuzamDjinn => Some(CreatureStats {
                power: 5,
                toughness: 5,
                haste: false,
                trample: false,
            }),
            Self::Juggernaut => Some(CreatureStats {
                power: 5,
                toughness: 3,
                haste: false,
                trample: false,
            }),
            Self::SageOfLatNam | Self::Tetravus => Some(CreatureStats {
                power: 1,
                toughness: 1,
                haste: false,
                trample: false,
            }),
            _ => None,
        }
    }

    #[must_use]
    pub const fn is_goblin(self) -> bool {
        matches!(
            self,
            Self::GoblinBalloonBrigade
                | Self::GoblinDiggingTeam
                | Self::GoblinKing
                | Self::GoblinsOfTheFlarg
        )
    }

    #[must_use]
    pub const fn has_flying(self) -> bool {
        matches!(
            self,
            Self::BirdsOfParadise
                | Self::DragonWhelp
                | Self::GraniteGargoyle
                | Self::HypnoticSpecter
                | Self::SerendibEfreet
                | Self::ScrybSprites
                | Self::SengirVampire
                | Self::SerraAngel
                | Self::ThunderSpirit
                | Self::Tetravus
        )
    }

    #[must_use]
    pub const fn has_mountainwalk(self) -> bool {
        matches!(self, Self::GoblinsOfTheFlarg)
    }

    /// Returns the printed color flags in `[white, blue, black, red, green]`
    /// order. Keeping this table in one place prevents the color predicates
    /// from drifting apart as the catalog grows.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::match_same_arms)]
    pub const fn color_identity(self) -> [bool; 5] {
        match self {
            Self::Armageddon
            | Self::Balance
            | Self::Crusade
            | Self::Disenchant
            | Self::DivineOffering
            | Self::DustToDust
            | Self::IcatianJavelineers
            | Self::Moat
            | Self::OrderOfLeitbur
            | Self::SavannahLions
            | Self::SerraAngel
            | Self::SwordsToPlowshares
            | Self::ThunderSpirit
            | Self::WhiteKnight
            | Self::WrathOfGod => [true, false, false, false, false],
            Self::AncestralRecall
            | Self::BlueElementalBlast
            | Self::Braingeyser
            | Self::CopyArtifact
            | Self::Counterspell
            | Self::EnergyFlux
            | Self::HurkylsRecall
            | Self::ManaDrain
            | Self::PsionicBlast
            | Self::Recall
            | Self::SageOfLatNam
            | Self::SerendibEfreet
            | Self::TimeWalk
            | Self::Timetwister => [false, true, false, false, false],
            Self::BlackKnight
            | Self::DarkRitual
            | Self::DemonicTutor
            | Self::DrainLife
            | Self::HymnToTourach
            | Self::HypnoticSpecter
            | Self::JuzamDjinn
            | Self::MindTwist
            | Self::OrderOfTheEbonHand
            | Self::SengirVampire
            | Self::Sinkhole
            | Self::Terror
            | Self::TheAbyss => [false, false, true, false, false],
            Self::Atog
            | Self::BallLightning
            | Self::BloodMoon
            | Self::ChainLightning
            | Self::Detonate
            | Self::DragonWhelp
            | Self::Earthquake
            | Self::Fireball
            | Self::Fork
            | Self::GoblinBalloonBrigade
            | Self::GoblinDiggingTeam
            | Self::GoblinGrenade
            | Self::GoblinKing
            | Self::GoblinsOfTheFlarg
            | Self::GraniteGargoyle
            | Self::IronclawOrcs
            | Self::KirdApe
            | Self::LightningBolt
            | Self::OrcishMechanics
            | Self::RedElementalBlast
            | Self::Shatter
            | Self::Smoke
            | Self::StoneGiant
            | Self::StoneRain
            | Self::SedgeTroll
            | Self::WheelOfFortune => [false, false, false, true, false],
            Self::ArgothianPixies
            | Self::Berserk
            | Self::GiantGrowth
            | Self::LlanowarElves
            | Self::ScrybSprites
            | Self::WhirlingDervish => [false, false, false, false, true],
            Self::BirdsOfParadise
            | Self::Channel
            | Self::ErhnamDjinn
            | Self::Regrowth
            | Self::SylvanLibrary => [false, false, false, false, true],
            _ => [false; 5],
        }
    }

    #[must_use]
    pub const fn is_red(self) -> bool {
        self.color_identity()[3]
    }

    #[must_use]
    pub const fn is_blue(self) -> bool {
        self.color_identity()[1]
    }

    #[must_use]
    pub const fn is_white(self) -> bool {
        self.color_identity()[0]
    }

    #[must_use]
    pub const fn is_black(self) -> bool {
        self.color_identity()[2]
    }

    #[must_use]
    pub const fn is_green(self) -> bool {
        self.color_identity()[4]
    }

    #[must_use]
    pub const fn has_vigilance(self) -> bool {
        matches!(self, Self::SerraAngel)
    }
}

#[derive(Clone, Debug, Default)]
pub struct CardCatalog {
    by_id: HashMap<CardDefinitionId, CardDefinition>,
    names: HashSet<String>,
}

impl CardCatalog {
    /// Builds a catalog whose card IDs and case-insensitive names are unique.
    ///
    /// # Errors
    ///
    /// Returns [`CatalogError`] when an ID or normalized card name is repeated.
    pub fn new(
        definitions: impl IntoIterator<Item = CardDefinition>,
    ) -> Result<Self, CatalogError> {
        let mut catalog = Self::default();
        for definition in definitions {
            if catalog.by_id.contains_key(&definition.id) {
                return Err(CatalogError::DuplicateId(definition.id));
            }
            let normalized_name = normalize_name(&definition.name);
            if !catalog.names.insert(normalized_name) {
                return Err(CatalogError::DuplicateName(definition.name));
            }
            catalog.by_id.insert(definition.id, definition);
        }
        Ok(catalog)
    }

    #[must_use]
    pub fn get(&self, id: CardDefinitionId) -> Option<&CardDefinition> {
        self.by_id.get(&id)
    }

    #[must_use]
    pub fn is_banned(&self, id: CardDefinitionId) -> bool {
        self.get(id)
            .is_some_and(|card| rules::is_banned(&card.name))
    }

    #[must_use]
    pub fn is_restricted(&self, id: CardDefinitionId) -> bool {
        self.get(id)
            .is_some_and(|card| rules::is_restricted(&card.name))
    }
}

fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CatalogError {
    DuplicateId(CardDefinitionId),
    DuplicateName(String),
}

impl fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(formatter, "duplicate card definition ID {id:?}"),
            Self::DuplicateName(name) => write!(formatter, "duplicate card name {name:?}"),
        }
    }
}

impl Error for CatalogError {}
