//! Built-in card corpus and representative Eternal Central Old School decks.

use crate::CardDefinitionId;
use crate::card::{CardBehavior, CardCatalog, CardDefinition, CardSet, CatalogError};
use crate::deck::Deck;

pub mod cards {
    use crate::CardDefinitionId;

    pub const ANKH_OF_MISHRA: CardDefinitionId = CardDefinitionId(1);
    pub const ATOG: CardDefinitionId = CardDefinitionId(2);
    pub const BALL_LIGHTNING: CardDefinitionId = CardDefinitionId(3);
    pub const BLACK_VISE: CardDefinitionId = CardDefinitionId(4);
    pub const BLOOD_MOON: CardDefinitionId = CardDefinitionId(5);
    pub const CHAIN_LIGHTNING: CardDefinitionId = CardDefinitionId(6);
    pub const COPPER_TABLET: CardDefinitionId = CardDefinitionId(7);
    pub const DETONATE: CardDefinitionId = CardDefinitionId(8);
    pub const FIREBALL: CardDefinitionId = CardDefinitionId(9);
    pub const FORK: CardDefinitionId = CardDefinitionId(10);
    pub const GLASSES_OF_URZA: CardDefinitionId = CardDefinitionId(11);
    pub const IRON_STAR: CardDefinitionId = CardDefinitionId(12);
    pub const LIGHTNING_BOLT: CardDefinitionId = CardDefinitionId(13);
    pub const MOUNTAIN: CardDefinitionId = CardDefinitionId(14);
    pub const RED_ELEMENTAL_BLAST: CardDefinitionId = CardDefinitionId(15);
    pub const SHATTER: CardDefinitionId = CardDefinitionId(16);
    pub const SMOKE: CardDefinitionId = CardDefinitionId(17);
    pub const STONE_GIANT: CardDefinitionId = CardDefinitionId(18);
    pub const SU_CHI: CardDefinitionId = CardDefinitionId(19);
    pub const WINTER_ORB: CardDefinitionId = CardDefinitionId(20);
    pub const BLACK_LOTUS: CardDefinitionId = CardDefinitionId(21);
    pub const CHAOS_ORB: CardDefinitionId = CardDefinitionId(22);
    pub const DRAGON_WHELP: CardDefinitionId = CardDefinitionId(23);
    pub const GOBLIN_BALLOON_BRIGADE: CardDefinitionId = CardDefinitionId(24);
    pub const GOBLIN_DIGGING_TEAM: CardDefinitionId = CardDefinitionId(25);
    pub const GOBLIN_GRENADE: CardDefinitionId = CardDefinitionId(26);
    pub const GOBLIN_KING: CardDefinitionId = CardDefinitionId(27);
    pub const GOBLINS_OF_THE_FLARG: CardDefinitionId = CardDefinitionId(28);
    pub const GRANITE_GARGOYLE: CardDefinitionId = CardDefinitionId(29);
    pub const IRONCLAW_ORCS: CardDefinitionId = CardDefinitionId(30);
    pub const MISHRA_S_FACTORY: CardDefinitionId = CardDefinitionId(31);
    pub const MOX_EMERALD: CardDefinitionId = CardDefinitionId(32);
    pub const MOX_JET: CardDefinitionId = CardDefinitionId(33);
    pub const MOX_PEARL: CardDefinitionId = CardDefinitionId(34);
    pub const MOX_RUBY: CardDefinitionId = CardDefinitionId(35);
    pub const MOX_SAPPHIRE: CardDefinitionId = CardDefinitionId(36);
    pub const ORCISH_MECHANICS: CardDefinitionId = CardDefinitionId(37);
    pub const SOL_RING: CardDefinitionId = CardDefinitionId(38);
    pub const STRIP_MINE: CardDefinitionId = CardDefinitionId(39);
    pub const WHEEL_OF_FORTUNE: CardDefinitionId = CardDefinitionId(40);
    pub const JUGGERNAUT: CardDefinitionId = CardDefinitionId(41);
    pub const MANA_VAULT: CardDefinitionId = CardDefinitionId(42);
    pub const TRISKELION: CardDefinitionId = CardDefinitionId(43);
    pub const ANCESTRAL_RECALL: CardDefinitionId = CardDefinitionId(44);
    pub const BRAINGEYSER: CardDefinitionId = CardDefinitionId(45);
    pub const COUNTERSPELL: CardDefinitionId = CardDefinitionId(46);
    pub const DISENCHANT: CardDefinitionId = CardDefinitionId(47);
    pub const FELLWAR_STONE: CardDefinitionId = CardDefinitionId(48);
    pub const ISLAND: CardDefinitionId = CardDefinitionId(49);
    pub const IVORY_TOWER: CardDefinitionId = CardDefinitionId(50);
    pub const JAYEMDAE_TOME: CardDefinitionId = CardDefinitionId(51);
    pub const PLAINS: CardDefinitionId = CardDefinitionId(52);
    pub const SERRA_ANGEL: CardDefinitionId = CardDefinitionId(53);
    pub const SWORDS_TO_PLOWSHARES: CardDefinitionId = CardDefinitionId(54);
    pub const TIME_WALK: CardDefinitionId = CardDefinitionId(55);
    pub const TUNDRA: CardDefinitionId = CardDefinitionId(56);
    pub const VOLCANIC_ISLAND: CardDefinitionId = CardDefinitionId(57);
    pub const ARMAGEDDON: CardDefinitionId = CardDefinitionId(58);
    pub const BADLANDS: CardDefinitionId = CardDefinitionId(59);
    pub const BALANCE: CardDefinitionId = CardDefinitionId(60);
    pub const BAYOU: CardDefinitionId = CardDefinitionId(61);
    pub const BLACK_KNIGHT: CardDefinitionId = CardDefinitionId(62);
    pub const BIRDS_OF_PARADISE: CardDefinitionId = CardDefinitionId(63);
    pub const BLUE_ELEMENTAL_BLAST: CardDefinitionId = CardDefinitionId(64);
    pub const CHANNEL: CardDefinitionId = CardDefinitionId(65);
    pub const CITY_OF_BRASS: CardDefinitionId = CardDefinitionId(66);
    pub const CRUSADE: CardDefinitionId = CardDefinitionId(67);
    pub const DARK_RITUAL: CardDefinitionId = CardDefinitionId(68);
    pub const DEMONIC_TUTOR: CardDefinitionId = CardDefinitionId(69);
    pub const DIVINE_OFFERING: CardDefinitionId = CardDefinitionId(70);
    pub const DRAIN_LIFE: CardDefinitionId = CardDefinitionId(71);
    pub const EARTHQUAKE: CardDefinitionId = CardDefinitionId(72);
    pub const ERHNAM_DJINN: CardDefinitionId = CardDefinitionId(73);
    pub const FOREST: CardDefinitionId = CardDefinitionId(74);
    pub const HYMN_TO_TOURACH: CardDefinitionId = CardDefinitionId(75);
    pub const HYPNOTIC_SPECTER: CardDefinitionId = CardDefinitionId(76);
    pub const ICATIAN_JAVELINEERS: CardDefinitionId = CardDefinitionId(77);
    pub const JUZAM_DJINN: CardDefinitionId = CardDefinitionId(78);
    pub const LIBRARY_OF_ALEXANDRIA: CardDefinitionId = CardDefinitionId(79);
    pub const MANA_DRAIN: CardDefinitionId = CardDefinitionId(80);
    pub const MAZE_OF_ITH: CardDefinitionId = CardDefinitionId(81);
    pub const MIND_TWIST: CardDefinitionId = CardDefinitionId(82);
    pub const MISHRA_S_WORKSHOP: CardDefinitionId = CardDefinitionId(83);
    pub const NEVINYRRALS_DISK: CardDefinitionId = CardDefinitionId(84);
    pub const ORDER_OF_LEITBUR: CardDefinitionId = CardDefinitionId(85);
    pub const ORDER_OF_THE_EBON_HAND: CardDefinitionId = CardDefinitionId(86);
    pub const PLATEAU: CardDefinitionId = CardDefinitionId(87);
    pub const PSIONIC_BLAST: CardDefinitionId = CardDefinitionId(88);
    pub const RECALL: CardDefinitionId = CardDefinitionId(89);
    pub const REGROWTH: CardDefinitionId = CardDefinitionId(90);
    pub const SAVANNAH: CardDefinitionId = CardDefinitionId(91);
    pub const SAVANNAH_LIONS: CardDefinitionId = CardDefinitionId(92);
    pub const SCRUBLAND: CardDefinitionId = CardDefinitionId(93);
    pub const SERENDIB_EFREET: CardDefinitionId = CardDefinitionId(94);
    pub const SENGIR_VAMPIRE: CardDefinitionId = CardDefinitionId(95);
    pub const SINKHOLE: CardDefinitionId = CardDefinitionId(96);
    pub const SWAMP: CardDefinitionId = CardDefinitionId(97);
    pub const SYLVAN_LIBRARY: CardDefinitionId = CardDefinitionId(98);
    pub const TAIGA: CardDefinitionId = CardDefinitionId(99);
    pub const TERROR: CardDefinitionId = CardDefinitionId(100);
    pub const THUNDER_SPIRIT: CardDefinitionId = CardDefinitionId(101);
    pub const TIME_VAULT: CardDefinitionId = CardDefinitionId(102);
    pub const TIMETWISTER: CardDefinitionId = CardDefinitionId(103);
    pub const TROPICAL_ISLAND: CardDefinitionId = CardDefinitionId(104);
    pub const UNDERGROUND_SEA: CardDefinitionId = CardDefinitionId(105);
    pub const WHIRLING_DERVISH: CardDefinitionId = CardDefinitionId(106);
    pub const WHITE_KNIGHT: CardDefinitionId = CardDefinitionId(107);
    pub const ARGOTHIAN_PIXIES: CardDefinitionId = CardDefinitionId(108);
    pub const BERSERK: CardDefinitionId = CardDefinitionId(109);
    pub const CITY_IN_A_BOTTLE: CardDefinitionId = CardDefinitionId(110);
    pub const COPY_ARTIFACT: CardDefinitionId = CardDefinitionId(111);
    pub const DUST_TO_DUST: CardDefinitionId = CardDefinitionId(112);
    pub const ENERGY_FLUX: CardDefinitionId = CardDefinitionId(113);
    pub const GIANT_GROWTH: CardDefinitionId = CardDefinitionId(114);
    pub const HURKYLS_RECALL: CardDefinitionId = CardDefinitionId(115);
    pub const ICY_MANIPULATOR: CardDefinitionId = CardDefinitionId(116);
    pub const KIRD_APE: CardDefinitionId = CardDefinitionId(117);
    pub const LLANOWAR_ELVES: CardDefinitionId = CardDefinitionId(118);
    pub const MOAT: CardDefinitionId = CardDefinitionId(119);
    pub const PENDELHAVEN: CardDefinitionId = CardDefinitionId(120);
    pub const RELIC_BARRIER: CardDefinitionId = CardDefinitionId(121);
    pub const SAGE_OF_LAT_NAM: CardDefinitionId = CardDefinitionId(122);
    pub const SEDGE_TROLL: CardDefinitionId = CardDefinitionId(123);
    pub const SCRYB_SPRITES: CardDefinitionId = CardDefinitionId(124);
    pub const STONE_RAIN: CardDefinitionId = CardDefinitionId(125);
    pub const TETRAVUS: CardDefinitionId = CardDefinitionId(126);
    pub const THE_ABYSS: CardDefinitionId = CardDefinitionId(127);
    pub const WRATH_OF_GOD: CardDefinitionId = CardDefinitionId(128);
}

/// Builds the complete card catalog required by the built-in decks.
///
/// # Errors
///
/// Returns [`CatalogError`] if the built-in IDs or names are accidentally
/// duplicated. Such an error indicates a bug in this crate.
#[allow(clippy::too_many_lines)]
pub fn catalog() -> Result<CardCatalog, CatalogError> {
    use cards::{
        ANCESTRAL_RECALL, BRAINGEYSER, COUNTERSPELL, DISENCHANT, FELLWAR_STONE, ISLAND,
        IVORY_TOWER, JAYEMDAE_TOME, PLAINS, SERRA_ANGEL, SWORDS_TO_PLOWSHARES, TIME_WALK, TUNDRA,
        VOLCANIC_ISLAND,
    };
    use cards::{
        ANKH_OF_MISHRA, ATOG, BALL_LIGHTNING, BLACK_VISE, BLOOD_MOON, CHAIN_LIGHTNING,
        COPPER_TABLET, DETONATE, FIREBALL, FORK, GLASSES_OF_URZA, IRON_STAR, LIGHTNING_BOLT,
        MOUNTAIN, RED_ELEMENTAL_BLAST, SHATTER, SMOKE, STONE_GIANT, SU_CHI, WINTER_ORB,
    };
    use cards::{
        ARGOTHIAN_PIXIES, BERSERK, CITY_IN_A_BOTTLE, COPY_ARTIFACT, DUST_TO_DUST, ENERGY_FLUX,
        GIANT_GROWTH, HURKYLS_RECALL, ICY_MANIPULATOR, KIRD_APE, LLANOWAR_ELVES, MOAT, PENDELHAVEN,
        RELIC_BARRIER, SAGE_OF_LAT_NAM, SCRYB_SPRITES, SEDGE_TROLL, STONE_RAIN, TETRAVUS,
        THE_ABYSS, WRATH_OF_GOD,
    };
    use cards::{
        BLACK_LOTUS, CHAOS_ORB, DRAGON_WHELP, GOBLIN_BALLOON_BRIGADE, GOBLIN_DIGGING_TEAM,
        GOBLIN_GRENADE, GOBLIN_KING, GOBLINS_OF_THE_FLARG, GRANITE_GARGOYLE, IRONCLAW_ORCS,
        MISHRA_S_FACTORY, MOX_EMERALD, MOX_JET, MOX_PEARL, MOX_RUBY, MOX_SAPPHIRE,
        ORCISH_MECHANICS, SOL_RING, STRIP_MINE, WHEEL_OF_FORTUNE,
    };
    use cards::{JUGGERNAUT, MANA_VAULT, TRISKELION};

    CardCatalog::new([
        card(
            ANKH_OF_MISHRA,
            "Ankh of Mishra",
            CardSet::Alpha,
            CardBehavior::AnkhOfMishra,
        ),
        card(ATOG, "Atog", CardSet::Antiquities, CardBehavior::Atog),
        card(
            BALL_LIGHTNING,
            "Ball Lightning",
            CardSet::TheDark,
            CardBehavior::BallLightning,
        ),
        card(
            BLACK_VISE,
            "Black Vise",
            CardSet::Alpha,
            CardBehavior::BlackVise,
        ),
        card(
            BLOOD_MOON,
            "Blood Moon",
            CardSet::TheDark,
            CardBehavior::BloodMoon,
        ),
        card(
            CHAIN_LIGHTNING,
            "Chain Lightning",
            CardSet::Legends,
            CardBehavior::ChainLightning,
        ),
        card(
            COPPER_TABLET,
            "Copper Tablet",
            CardSet::Alpha,
            CardBehavior::CopperTablet,
        ),
        card(
            DETONATE,
            "Detonate",
            CardSet::Antiquities,
            CardBehavior::Detonate,
        ),
        card(FIREBALL, "Fireball", CardSet::Alpha, CardBehavior::Fireball),
        card(FORK, "Fork", CardSet::Alpha, CardBehavior::Fork),
        card(
            GLASSES_OF_URZA,
            "Glasses of Urza",
            CardSet::Alpha,
            CardBehavior::GlassesOfUrza,
        ),
        card(
            IRON_STAR,
            "Iron Star",
            CardSet::Alpha,
            CardBehavior::IronStar,
        ),
        card_with_behavior(
            LIGHTNING_BOLT,
            "Lightning Bolt",
            CardSet::Alpha,
            false,
            CardBehavior::LightningBolt,
        ),
        card_with_behavior(
            MOUNTAIN,
            "Mountain",
            CardSet::Alpha,
            true,
            CardBehavior::Mountain,
        ),
        card(
            RED_ELEMENTAL_BLAST,
            "Red Elemental Blast",
            CardSet::Alpha,
            CardBehavior::RedElementalBlast,
        ),
        card(SHATTER, "Shatter", CardSet::Alpha, CardBehavior::Shatter),
        card(SMOKE, "Smoke", CardSet::Alpha, CardBehavior::Smoke),
        card(
            STONE_GIANT,
            "Stone Giant",
            CardSet::Alpha,
            CardBehavior::StoneGiant,
        ),
        card(SU_CHI, "Su-Chi", CardSet::Antiquities, CardBehavior::SuChi),
        card(
            WINTER_ORB,
            "Winter Orb",
            CardSet::Alpha,
            CardBehavior::WinterOrb,
        ),
        card(
            BLACK_LOTUS,
            "Black Lotus",
            CardSet::Alpha,
            CardBehavior::BlackLotus,
        ),
        card(
            CHAOS_ORB,
            "Chaos Orb",
            CardSet::Alpha,
            CardBehavior::ChaosOrb,
        ),
        card(
            DRAGON_WHELP,
            "Dragon Whelp",
            CardSet::Alpha,
            CardBehavior::DragonWhelp,
        ),
        card(
            GOBLIN_BALLOON_BRIGADE,
            "Goblin Balloon Brigade",
            CardSet::Alpha,
            CardBehavior::GoblinBalloonBrigade,
        ),
        card(
            GOBLIN_DIGGING_TEAM,
            "Goblin Digging Team",
            CardSet::TheDark,
            CardBehavior::GoblinDiggingTeam,
        ),
        card(
            GOBLIN_GRENADE,
            "Goblin Grenade",
            CardSet::FallenEmpires,
            CardBehavior::GoblinGrenade,
        ),
        card(
            GOBLIN_KING,
            "Goblin King",
            CardSet::Alpha,
            CardBehavior::GoblinKing,
        ),
        card(
            GOBLINS_OF_THE_FLARG,
            "Goblins of the Flarg",
            CardSet::TheDark,
            CardBehavior::GoblinsOfTheFlarg,
        ),
        card(
            GRANITE_GARGOYLE,
            "Granite Gargoyle",
            CardSet::Alpha,
            CardBehavior::GraniteGargoyle,
        ),
        card(
            IRONCLAW_ORCS,
            "Ironclaw Orcs",
            CardSet::Alpha,
            CardBehavior::IronclawOrcs,
        ),
        card(
            MISHRA_S_FACTORY,
            "Mishra's Factory",
            CardSet::Antiquities,
            CardBehavior::MishrasFactory,
        ),
        card(
            MOX_EMERALD,
            "Mox Emerald",
            CardSet::Alpha,
            CardBehavior::MoxEmerald,
        ),
        card(MOX_JET, "Mox Jet", CardSet::Alpha, CardBehavior::MoxJet),
        card(
            MOX_PEARL,
            "Mox Pearl",
            CardSet::Alpha,
            CardBehavior::MoxPearl,
        ),
        card(MOX_RUBY, "Mox Ruby", CardSet::Alpha, CardBehavior::MoxRuby),
        card(
            MOX_SAPPHIRE,
            "Mox Sapphire",
            CardSet::Alpha,
            CardBehavior::MoxSapphire,
        ),
        card(
            ORCISH_MECHANICS,
            "Orcish Mechanics",
            CardSet::Antiquities,
            CardBehavior::OrcishMechanics,
        ),
        card(SOL_RING, "Sol Ring", CardSet::Alpha, CardBehavior::SolRing),
        card(
            STRIP_MINE,
            "Strip Mine",
            CardSet::Antiquities,
            CardBehavior::StripMine,
        ),
        card(
            WHEEL_OF_FORTUNE,
            "Wheel of Fortune",
            CardSet::Alpha,
            CardBehavior::WheelOfFortune,
        ),
        card(
            JUGGERNAUT,
            "Juggernaut",
            CardSet::Alpha,
            CardBehavior::Juggernaut,
        ),
        card(
            MANA_VAULT,
            "Mana Vault",
            CardSet::Alpha,
            CardBehavior::ManaVault,
        ),
        card(
            TRISKELION,
            "Triskelion",
            CardSet::Antiquities,
            CardBehavior::Triskelion,
        ),
        card(
            ANCESTRAL_RECALL,
            "Ancestral Recall",
            CardSet::Alpha,
            CardBehavior::AncestralRecall,
        ),
        card(
            BRAINGEYSER,
            "Braingeyser",
            CardSet::Alpha,
            CardBehavior::Braingeyser,
        ),
        card(
            COUNTERSPELL,
            "Counterspell",
            CardSet::Alpha,
            CardBehavior::Counterspell,
        ),
        card(
            DISENCHANT,
            "Disenchant",
            CardSet::Alpha,
            CardBehavior::Disenchant,
        ),
        card(
            FELLWAR_STONE,
            "Fellwar Stone",
            CardSet::TheDark,
            CardBehavior::FellwarStone,
        ),
        card_with_behavior(ISLAND, "Island", CardSet::Alpha, true, CardBehavior::Island),
        card(
            IVORY_TOWER,
            "Ivory Tower",
            CardSet::Antiquities,
            CardBehavior::IvoryTower,
        ),
        card(
            JAYEMDAE_TOME,
            "Jayemdae Tome",
            CardSet::Alpha,
            CardBehavior::JayemdaeTome,
        ),
        card_with_behavior(PLAINS, "Plains", CardSet::Alpha, true, CardBehavior::Plains),
        card(
            SERRA_ANGEL,
            "Serra Angel",
            CardSet::Alpha,
            CardBehavior::SerraAngel,
        ),
        card(
            SWORDS_TO_PLOWSHARES,
            "Swords to Plowshares",
            CardSet::Alpha,
            CardBehavior::SwordsToPlowshares,
        ),
        card(
            TIME_WALK,
            "Time Walk",
            CardSet::Alpha,
            CardBehavior::TimeWalk,
        ),
        card(TUNDRA, "Tundra", CardSet::Alpha, CardBehavior::Tundra),
        card(
            VOLCANIC_ISLAND,
            "Volcanic Island",
            CardSet::Alpha,
            CardBehavior::VolcanicIsland,
        ),
        card(
            cards::ARMAGEDDON,
            "Armageddon",
            CardSet::Alpha,
            CardBehavior::Armageddon,
        ),
        card(
            cards::BADLANDS,
            "Badlands",
            CardSet::Alpha,
            CardBehavior::Badlands,
        ),
        card(
            cards::BALANCE,
            "Balance",
            CardSet::Alpha,
            CardBehavior::Balance,
        ),
        card(cards::BAYOU, "Bayou", CardSet::Alpha, CardBehavior::Bayou),
        card(
            cards::BLACK_KNIGHT,
            "Black Knight",
            CardSet::Alpha,
            CardBehavior::BlackKnight,
        ),
        card(
            cards::BIRDS_OF_PARADISE,
            "Birds of Paradise",
            CardSet::Alpha,
            CardBehavior::BirdsOfParadise,
        ),
        card(
            cards::BLUE_ELEMENTAL_BLAST,
            "Blue Elemental Blast",
            CardSet::Alpha,
            CardBehavior::BlueElementalBlast,
        ),
        card(
            cards::CHANNEL,
            "Channel",
            CardSet::Alpha,
            CardBehavior::Channel,
        ),
        card(
            cards::CITY_OF_BRASS,
            "City of Brass",
            CardSet::ArabianNights,
            CardBehavior::CityOfBrass,
        ),
        card(
            cards::CRUSADE,
            "Crusade",
            CardSet::Alpha,
            CardBehavior::Crusade,
        ),
        card(
            cards::DARK_RITUAL,
            "Dark Ritual",
            CardSet::Alpha,
            CardBehavior::DarkRitual,
        ),
        card(
            cards::DEMONIC_TUTOR,
            "Demonic Tutor",
            CardSet::Alpha,
            CardBehavior::DemonicTutor,
        ),
        card(
            cards::DIVINE_OFFERING,
            "Divine Offering",
            CardSet::Legends,
            CardBehavior::DivineOffering,
        ),
        card(
            cards::DRAIN_LIFE,
            "Drain Life",
            CardSet::Alpha,
            CardBehavior::DrainLife,
        ),
        card(
            cards::EARTHQUAKE,
            "Earthquake",
            CardSet::Alpha,
            CardBehavior::Earthquake,
        ),
        card(
            cards::ERHNAM_DJINN,
            "Erhnam Djinn",
            CardSet::ArabianNights,
            CardBehavior::ErhnamDjinn,
        ),
        card_with_behavior(
            cards::FOREST,
            "Forest",
            CardSet::Alpha,
            true,
            CardBehavior::Forest,
        ),
        card(
            cards::HYMN_TO_TOURACH,
            "Hymn to Tourach",
            CardSet::FallenEmpires,
            CardBehavior::HymnToTourach,
        ),
        card(
            cards::HYPNOTIC_SPECTER,
            "Hypnotic Specter",
            CardSet::Alpha,
            CardBehavior::HypnoticSpecter,
        ),
        card(
            cards::ICATIAN_JAVELINEERS,
            "Icatian Javelineers",
            CardSet::FallenEmpires,
            CardBehavior::IcatianJavelineers,
        ),
        card(
            cards::JUZAM_DJINN,
            "Juzam Djinn",
            CardSet::ArabianNights,
            CardBehavior::JuzamDjinn,
        ),
        card(
            cards::LIBRARY_OF_ALEXANDRIA,
            "Library of Alexandria",
            CardSet::ArabianNights,
            CardBehavior::LibraryOfAlexandria,
        ),
        card(
            cards::MANA_DRAIN,
            "Mana Drain",
            CardSet::Legends,
            CardBehavior::ManaDrain,
        ),
        card(
            cards::MAZE_OF_ITH,
            "Maze of Ith",
            CardSet::TheDark,
            CardBehavior::MazeOfIth,
        ),
        card(
            cards::MIND_TWIST,
            "Mind Twist",
            CardSet::Alpha,
            CardBehavior::MindTwist,
        ),
        card(
            cards::MISHRA_S_WORKSHOP,
            "Mishra's Workshop",
            CardSet::Antiquities,
            CardBehavior::MishrasWorkshop,
        ),
        card(
            cards::NEVINYRRALS_DISK,
            "Nevinyrral's Disk",
            CardSet::Alpha,
            CardBehavior::NevinyrralsDisk,
        ),
        card(
            cards::ORDER_OF_LEITBUR,
            "Order of Leitbur",
            CardSet::FallenEmpires,
            CardBehavior::OrderOfLeitbur,
        ),
        card(
            cards::ORDER_OF_THE_EBON_HAND,
            "Order of the Ebon Hand",
            CardSet::FallenEmpires,
            CardBehavior::OrderOfTheEbonHand,
        ),
        card(
            cards::PLATEAU,
            "Plateau",
            CardSet::Alpha,
            CardBehavior::Plateau,
        ),
        card(
            cards::PSIONIC_BLAST,
            "Psionic Blast",
            CardSet::Alpha,
            CardBehavior::PsionicBlast,
        ),
        card(
            cards::RECALL,
            "Recall",
            CardSet::Legends,
            CardBehavior::Recall,
        ),
        card(
            cards::REGROWTH,
            "Regrowth",
            CardSet::Alpha,
            CardBehavior::Regrowth,
        ),
        card(
            cards::SAVANNAH,
            "Savannah",
            CardSet::Alpha,
            CardBehavior::Savannah,
        ),
        card(
            cards::SAVANNAH_LIONS,
            "Savannah Lions",
            CardSet::Alpha,
            CardBehavior::SavannahLions,
        ),
        card(
            cards::SCRUBLAND,
            "Scrubland",
            CardSet::Alpha,
            CardBehavior::Scrubland,
        ),
        card(
            cards::SERENDIB_EFREET,
            "Serendib Efreet",
            CardSet::ArabianNights,
            CardBehavior::SerendibEfreet,
        ),
        card(
            cards::SENGIR_VAMPIRE,
            "Sengir Vampire",
            CardSet::Alpha,
            CardBehavior::SengirVampire,
        ),
        card(
            cards::SINKHOLE,
            "Sinkhole",
            CardSet::Alpha,
            CardBehavior::Sinkhole,
        ),
        card_with_behavior(
            cards::SWAMP,
            "Swamp",
            CardSet::Alpha,
            true,
            CardBehavior::Swamp,
        ),
        card(
            cards::SYLVAN_LIBRARY,
            "Sylvan Library",
            CardSet::Legends,
            CardBehavior::SylvanLibrary,
        ),
        card(cards::TAIGA, "Taiga", CardSet::Alpha, CardBehavior::Taiga),
        card(
            cards::TERROR,
            "Terror",
            CardSet::Alpha,
            CardBehavior::Terror,
        ),
        card(
            cards::THUNDER_SPIRIT,
            "Thunder Spirit",
            CardSet::Legends,
            CardBehavior::ThunderSpirit,
        ),
        card(
            cards::TIME_VAULT,
            "Time Vault",
            CardSet::Alpha,
            CardBehavior::TimeVault,
        ),
        card(
            cards::TIMETWISTER,
            "Timetwister",
            CardSet::Alpha,
            CardBehavior::Timetwister,
        ),
        card(
            cards::TROPICAL_ISLAND,
            "Tropical Island",
            CardSet::Alpha,
            CardBehavior::TropicalIsland,
        ),
        card(
            cards::UNDERGROUND_SEA,
            "Underground Sea",
            CardSet::Alpha,
            CardBehavior::UndergroundSea,
        ),
        card(
            cards::WHIRLING_DERVISH,
            "Whirling Dervish",
            CardSet::Legends,
            CardBehavior::WhirlingDervish,
        ),
        card(
            cards::WHITE_KNIGHT,
            "White Knight",
            CardSet::Alpha,
            CardBehavior::WhiteKnight,
        ),
        card(
            ARGOTHIAN_PIXIES,
            "Argothian Pixies",
            CardSet::Antiquities,
            CardBehavior::ArgothianPixies,
        ),
        card(BERSERK, "Berserk", CardSet::Alpha, CardBehavior::Berserk),
        card(
            CITY_IN_A_BOTTLE,
            "City in a Bottle",
            CardSet::ArabianNights,
            CardBehavior::CityInABottle,
        ),
        card(
            COPY_ARTIFACT,
            "Copy Artifact",
            CardSet::Alpha,
            CardBehavior::CopyArtifact,
        ),
        card(
            DUST_TO_DUST,
            "Dust to Dust",
            CardSet::TheDark,
            CardBehavior::DustToDust,
        ),
        card(
            ENERGY_FLUX,
            "Energy Flux",
            CardSet::Legends,
            CardBehavior::EnergyFlux,
        ),
        card(
            GIANT_GROWTH,
            "Giant Growth",
            CardSet::Alpha,
            CardBehavior::GiantGrowth,
        ),
        card(
            HURKYLS_RECALL,
            "Hurkyl's Recall",
            CardSet::Antiquities,
            CardBehavior::HurkylsRecall,
        ),
        card(
            ICY_MANIPULATOR,
            "Icy Manipulator",
            CardSet::Alpha,
            CardBehavior::IcyManipulator,
        ),
        card(
            KIRD_APE,
            "Kird Ape",
            CardSet::ArabianNights,
            CardBehavior::KirdApe,
        ),
        card(
            LLANOWAR_ELVES,
            "Llanowar Elves",
            CardSet::Alpha,
            CardBehavior::LlanowarElves,
        ),
        card(MOAT, "Moat", CardSet::Legends, CardBehavior::Moat),
        card(
            PENDELHAVEN,
            "Pendelhaven",
            CardSet::Legends,
            CardBehavior::Pendelhaven,
        ),
        card(
            RELIC_BARRIER,
            "Relic Barrier",
            CardSet::Legends,
            CardBehavior::RelicBarrier,
        ),
        card(
            SAGE_OF_LAT_NAM,
            "Sage of Lat-Nam",
            CardSet::Antiquities,
            CardBehavior::SageOfLatNam,
        ),
        card(
            SEDGE_TROLL,
            "Sedge Troll",
            CardSet::Legends,
            CardBehavior::SedgeTroll,
        ),
        card(
            SCRYB_SPRITES,
            "Scryb Sprites",
            CardSet::Alpha,
            CardBehavior::ScrybSprites,
        ),
        card(
            STONE_RAIN,
            "Stone Rain",
            CardSet::Alpha,
            CardBehavior::StoneRain,
        ),
        card(
            TETRAVUS,
            "Tetravus",
            CardSet::Antiquities,
            CardBehavior::Tetravus,
        ),
        card(
            THE_ABYSS,
            "The Abyss",
            CardSet::Legends,
            CardBehavior::TheAbyss,
        ),
        card(
            WRATH_OF_GOD,
            "Wrath of God",
            CardSet::Alpha,
            CardBehavior::WrathOfGod,
        ),
    ])
}

/// Returns a representative powered EC Goblins deck.
#[must_use]
pub fn goblins() -> Deck {
    use cards::{
        BALL_LIGHTNING, BLACK_LOTUS, BLACK_VISE, BLOOD_MOON, CHAIN_LIGHTNING, CHAOS_ORB, DETONATE,
        FIREBALL, FORK, GOBLIN_BALLOON_BRIGADE, GOBLIN_DIGGING_TEAM, GOBLIN_GRENADE, GOBLIN_KING,
        GOBLINS_OF_THE_FLARG, IRON_STAR, LIGHTNING_BOLT, MISHRA_S_FACTORY, MOUNTAIN, MOX_RUBY,
        RED_ELEMENTAL_BLAST, SHATTER, STRIP_MINE, WHEEL_OF_FORTUNE,
    };

    Deck {
        main: copies(MOUNTAIN, 17)
            .chain(copies(MISHRA_S_FACTORY, 1))
            .chain(copies(STRIP_MINE, 1))
            .chain(copies(GOBLIN_BALLOON_BRIGADE, 4))
            .chain(copies(GOBLIN_DIGGING_TEAM, 4))
            .chain(copies(GOBLINS_OF_THE_FLARG, 4))
            .chain(copies(GOBLIN_KING, 3))
            .chain(copies(BALL_LIGHTNING, 3))
            .chain(copies(LIGHTNING_BOLT, 4))
            .chain(copies(CHAIN_LIGHTNING, 4))
            .chain(copies(GOBLIN_GRENADE, 4))
            .chain(copies(FORK, 1))
            .chain(copies(FIREBALL, 1))
            .chain(copies(WHEEL_OF_FORTUNE, 1))
            .chain(copies(BLACK_VISE, 4))
            .chain(copies(BLACK_LOTUS, 1))
            .chain(copies(MOX_RUBY, 1))
            .chain(copies(CHAOS_ORB, 1))
            .chain(copies(BLOOD_MOON, 1))
            .collect(),
        sideboard: copies(RED_ELEMENTAL_BLAST, 4)
            .chain(copies(SHATTER, 3))
            .chain(copies(BLOOD_MOON, 2))
            .chain(copies(DETONATE, 2))
            .chain(copies(IRON_STAR, 4))
            .collect(),
    }
}

/// Returns a representative powered EC Sligh deck.
#[must_use]
pub fn sligh() -> Deck {
    use cards::{
        ANKH_OF_MISHRA, BALL_LIGHTNING, BLACK_VISE, BLOOD_MOON, CHAIN_LIGHTNING, CHAOS_ORB,
        DETONATE, DRAGON_WHELP, FIREBALL, GOBLIN_BALLOON_BRIGADE, GOBLINS_OF_THE_FLARG,
        GRANITE_GARGOYLE, IRON_STAR, IRONCLAW_ORCS, LIGHTNING_BOLT, MISHRA_S_FACTORY, MOUNTAIN,
        MOX_RUBY, RED_ELEMENTAL_BLAST, SHATTER, STRIP_MINE, WHEEL_OF_FORTUNE,
    };

    Deck {
        main: copies(MOUNTAIN, 14)
            .chain(copies(MISHRA_S_FACTORY, 4))
            .chain(copies(STRIP_MINE, 4))
            .chain(copies(GOBLIN_BALLOON_BRIGADE, 4))
            .chain(copies(GOBLINS_OF_THE_FLARG, 4))
            .chain(copies(IRONCLAW_ORCS, 4))
            .chain(copies(BALL_LIGHTNING, 3))
            .chain(copies(GRANITE_GARGOYLE, 2))
            .chain(copies(DRAGON_WHELP, 2))
            .chain(copies(LIGHTNING_BOLT, 4))
            .chain(copies(CHAIN_LIGHTNING, 4))
            .chain(copies(FIREBALL, 2))
            .chain(copies(WHEEL_OF_FORTUNE, 1))
            .chain(copies(BLOOD_MOON, 2))
            .chain(copies(ANKH_OF_MISHRA, 2))
            .chain(copies(BLACK_VISE, 2))
            .chain(copies(CHAOS_ORB, 1))
            .chain(copies(MOX_RUBY, 1))
            .collect(),
        sideboard: copies(RED_ELEMENTAL_BLAST, 4)
            .chain(copies(SHATTER, 4))
            .chain(copies(BLOOD_MOON, 1))
            .chain(copies(DETONATE, 2))
            .chain(copies(IRON_STAR, 4))
            .collect(),
    }
}

/// Returns a representative powered EC Atog artifact deck.
#[must_use]
pub fn artifacts() -> Deck {
    use cards::{
        ANKH_OF_MISHRA, ATOG, BLACK_LOTUS, BLACK_VISE, BLOOD_MOON, CHAIN_LIGHTNING, CHAOS_ORB,
        COPPER_TABLET, DETONATE, FIREBALL, LIGHTNING_BOLT, MISHRA_S_FACTORY, MOUNTAIN, MOX_EMERALD,
        MOX_JET, MOX_PEARL, MOX_RUBY, MOX_SAPPHIRE, ORCISH_MECHANICS, RED_ELEMENTAL_BLAST, SHATTER,
        SOL_RING, STRIP_MINE, SU_CHI, WHEEL_OF_FORTUNE, WINTER_ORB,
    };

    Deck {
        main: copies(MOUNTAIN, 13)
            .chain(copies(MISHRA_S_FACTORY, 4))
            .chain(copies(STRIP_MINE, 4))
            .chain(copies(ATOG, 4))
            .chain(copies(ORCISH_MECHANICS, 3))
            .chain(copies(ANKH_OF_MISHRA, 4))
            .chain(copies(BLACK_VISE, 4))
            .chain(copies(COPPER_TABLET, 4))
            .chain(copies(BLACK_LOTUS, 1))
            .chain(copies(MOX_EMERALD, 1))
            .chain(copies(MOX_JET, 1))
            .chain(copies(MOX_PEARL, 1))
            .chain(copies(MOX_RUBY, 1))
            .chain(copies(MOX_SAPPHIRE, 1))
            .chain(copies(SOL_RING, 1))
            .chain(copies(WINTER_ORB, 1))
            .chain(copies(CHAOS_ORB, 1))
            .chain(copies(LIGHTNING_BOLT, 4))
            .chain(copies(CHAIN_LIGHTNING, 4))
            .chain(copies(WHEEL_OF_FORTUNE, 1))
            .chain(copies(BLOOD_MOON, 2))
            .collect(),
        sideboard: copies(RED_ELEMENTAL_BLAST, 4)
            .chain(copies(SHATTER, 3))
            .chain(copies(DETONATE, 2))
            .chain(copies(BLOOD_MOON, 1))
            .chain(copies(SU_CHI, 4))
            .chain(copies(FIREBALL, 1))
            .collect(),
    }
}

/// Returns a representative powered EC mono-red Robots deck.
#[must_use]
pub fn robots() -> Deck {
    use cards::{
        ATOG, BLACK_LOTUS, BLACK_VISE, BLOOD_MOON, CHAOS_ORB, DETONATE, FIREBALL, JUGGERNAUT,
        LIGHTNING_BOLT, MANA_VAULT, MISHRA_S_FACTORY, MOUNTAIN, MOX_RUBY, RED_ELEMENTAL_BLAST,
        SHATTER, SOL_RING, STRIP_MINE, SU_CHI, TRISKELION, WHEEL_OF_FORTUNE,
    };

    Deck {
        main: copies(MOUNTAIN, 15)
            .chain(copies(MISHRA_S_FACTORY, 4))
            .chain(copies(STRIP_MINE, 4))
            .chain(copies(BLACK_LOTUS, 1))
            .chain(copies(MOX_RUBY, 1))
            .chain(copies(SOL_RING, 1))
            .chain(copies(MANA_VAULT, 4))
            .chain(copies(ATOG, 4))
            .chain(copies(SU_CHI, 4))
            .chain(copies(JUGGERNAUT, 4))
            .chain(copies(TRISKELION, 4))
            .chain(copies(BLACK_VISE, 4))
            .chain(copies(LIGHTNING_BOLT, 4))
            .chain(copies(FIREBALL, 2))
            .chain(copies(WHEEL_OF_FORTUNE, 1))
            .chain(copies(CHAOS_ORB, 1))
            .chain(copies(BLOOD_MOON, 2))
            .collect(),
        sideboard: copies(RED_ELEMENTAL_BLAST, 4)
            .chain(copies(SHATTER, 4))
            .chain(copies(DETONATE, 4))
            .chain(copies(BLOOD_MOON, 2))
            .chain(copies(FIREBALL, 1))
            .collect(),
    }
}

/// Returns a representative powered EC control deck known as "The Deck."
#[must_use]
pub fn the_deck() -> Deck {
    use cards::{
        ANCESTRAL_RECALL, BALANCE, BLACK_LOTUS, BLUE_ELEMENTAL_BLAST, BRAINGEYSER, CHAOS_ORB,
        CITY_OF_BRASS, COUNTERSPELL, DEMONIC_TUTOR, DISENCHANT, DIVINE_OFFERING, FELLWAR_STONE,
        FIREBALL, IVORY_TOWER, JAYEMDAE_TOME, LIBRARY_OF_ALEXANDRIA, LIGHTNING_BOLT, MANA_DRAIN,
        MIND_TWIST, MISHRA_S_FACTORY, MOX_EMERALD, MOX_JET, MOX_PEARL, MOX_RUBY, MOX_SAPPHIRE,
        RECALL, RED_ELEMENTAL_BLAST, SERRA_ANGEL, SOL_RING, STRIP_MINE, SWORDS_TO_PLOWSHARES,
        TIME_WALK, TIMETWISTER, TUNDRA, UNDERGROUND_SEA, VOLCANIC_ISLAND,
    };

    Deck {
        main: copies(CITY_OF_BRASS, 4)
            .chain(copies(UNDERGROUND_SEA, 4))
            .chain(copies(TUNDRA, 4))
            .chain(copies(VOLCANIC_ISLAND, 3))
            .chain(copies(LIBRARY_OF_ALEXANDRIA, 1))
            .chain(copies(MISHRA_S_FACTORY, 2))
            .chain(copies(STRIP_MINE, 1))
            .chain(copies(BLACK_LOTUS, 1))
            .chain(copies(MOX_EMERALD, 1))
            .chain(copies(MOX_JET, 1))
            .chain(copies(MOX_PEARL, 1))
            .chain(copies(MOX_RUBY, 1))
            .chain(copies(MOX_SAPPHIRE, 1))
            .chain(copies(SOL_RING, 1))
            .chain(copies(COUNTERSPELL, 4))
            .chain(copies(MANA_DRAIN, 1))
            .chain(copies(SWORDS_TO_PLOWSHARES, 4))
            .chain(copies(DISENCHANT, 2))
            .chain(copies(DIVINE_OFFERING, 1))
            .chain(copies(BALANCE, 1))
            .chain(copies(CHAOS_ORB, 1))
            .chain(copies(ANCESTRAL_RECALL, 1))
            .chain(copies(TIME_WALK, 1))
            .chain(copies(BRAINGEYSER, 1))
            .chain(copies(TIMETWISTER, 1))
            .chain(copies(DEMONIC_TUTOR, 1))
            .chain(copies(MIND_TWIST, 1))
            .chain(copies(RECALL, 1))
            .chain(copies(JAYEMDAE_TOME, 3))
            .chain(copies(SERRA_ANGEL, 2))
            .chain(copies(FIREBALL, 1))
            .chain(copies(LIGHTNING_BOLT, 1))
            .chain(copies(IVORY_TOWER, 2))
            .chain(copies(FELLWAR_STONE, 2))
            .chain(copies(BLUE_ELEMENTAL_BLAST, 1))
            .chain(copies(RED_ELEMENTAL_BLAST, 1))
            .collect(),
        sideboard: copies(RED_ELEMENTAL_BLAST, 3)
            .chain(copies(BLUE_ELEMENTAL_BLAST, 3))
            .chain(copies(DISENCHANT, 2))
            .chain(copies(SERRA_ANGEL, 2))
            .chain(copies(IVORY_TOWER, 2))
            .chain(copies(JAYEMDAE_TOME, 1))
            .chain(copies(FIREBALL, 2))
            .collect(),
    }
}

/// Returns a representative powered EC Mono Black deck.
#[must_use]
pub fn mono_black() -> Deck {
    #[allow(clippy::wildcard_imports)]
    use cards::*;
    Deck {
        main: copies(SWAMP, 17)
            .chain(copies(MISHRA_S_FACTORY, 4))
            .chain(copies(STRIP_MINE, 4))
            .chain(copies(BLACK_LOTUS, 1))
            .chain(copies(MOX_JET, 1))
            .chain(copies(SOL_RING, 1))
            .chain(copies(DARK_RITUAL, 4))
            .chain(copies(BLACK_KNIGHT, 4))
            .chain(copies(ORDER_OF_THE_EBON_HAND, 4))
            .chain(copies(HYPNOTIC_SPECTER, 4))
            .chain(copies(JUZAM_DJINN, 4))
            .chain(copies(HYMN_TO_TOURACH, 4))
            .chain(copies(SINKHOLE, 4))
            .chain(copies(TERROR, 2))
            .chain(copies(MIND_TWIST, 1))
            .chain(copies(DEMONIC_TUTOR, 1))
            .collect(),
        sideboard: copies(NEVINYRRALS_DISK, 3)
            .chain(copies(SENGIR_VAMPIRE, 3))
            .chain(copies(DRAIN_LIFE, 3))
            .chain(copies(TERROR, 2))
            .chain(copies(BLACK_VISE, 4))
            .collect(),
    }
}

/// Returns a representative powered EC White Weenie deck.
#[must_use]
pub fn white_weenie() -> Deck {
    #[allow(clippy::wildcard_imports)]
    use cards::*;
    Deck {
        main: copies(PLAINS, 16)
            .chain(copies(MISHRA_S_FACTORY, 4))
            .chain(copies(STRIP_MINE, 4))
            .chain(copies(BLACK_LOTUS, 1))
            .chain(copies(MOX_PEARL, 1))
            .chain(copies(SOL_RING, 1))
            .chain(copies(SAVANNAH_LIONS, 4))
            .chain(copies(ICATIAN_JAVELINEERS, 4))
            .chain(copies(WHITE_KNIGHT, 4))
            .chain(copies(ORDER_OF_LEITBUR, 4))
            .chain(copies(THUNDER_SPIRIT, 3))
            .chain(copies(CRUSADE, 4))
            .chain(copies(SWORDS_TO_PLOWSHARES, 4))
            .chain(copies(DISENCHANT, 3))
            .chain(copies(ARMAGEDDON, 2))
            .chain(copies(BALANCE, 1))
            .collect(),
        sideboard: copies(DIVINE_OFFERING, 3)
            .chain(copies(BLUE_ELEMENTAL_BLAST, 3))
            .chain(copies(ARMAGEDDON, 2))
            .chain(copies(SERRA_ANGEL, 2))
            .chain(copies(BLACK_VISE, 4))
            .chain(copies(DISENCHANT, 1))
            .collect(),
    }
}

/// Returns a representative powered EC Erhnamgeddon deck.
#[must_use]
pub fn erhnamgeddon() -> Deck {
    #[allow(clippy::wildcard_imports)]
    use cards::*;
    Deck {
        main: copies(FOREST, 8)
            .chain(copies(SAVANNAH, 4))
            .chain(copies(CITY_OF_BRASS, 4))
            .chain(copies(MISHRA_S_FACTORY, 4))
            .chain(copies(STRIP_MINE, 2))
            .chain(copies(BLACK_LOTUS, 1))
            .chain(copies(MOX_EMERALD, 1))
            .chain(copies(MOX_PEARL, 1))
            .chain(copies(SOL_RING, 1))
            .chain(copies(BIRDS_OF_PARADISE, 4))
            .chain(copies(ERHNAM_DJINN, 4))
            .chain(copies(WHIRLING_DERVISH, 4))
            .chain(copies(SERRA_ANGEL, 2))
            .chain(copies(SU_CHI, 3))
            .chain(copies(ARMAGEDDON, 4))
            .chain(copies(SWORDS_TO_PLOWSHARES, 4))
            .chain(copies(DISENCHANT, 3))
            .chain(copies(SYLVAN_LIBRARY, 2))
            .chain(copies(BALANCE, 1))
            .chain(copies(CHAOS_ORB, 1))
            .chain(copies(JAYEMDAE_TOME, 2))
            .collect(),
        sideboard: copies(BLUE_ELEMENTAL_BLAST, 4)
            .chain(copies(DIVINE_OFFERING, 4))
            .chain(copies(SERRA_ANGEL, 2))
            .chain(copies(DISENCHANT, 1))
            .chain(copies(BLACK_VISE, 4))
            .collect(),
    }
}

/// Returns a representative powered EC Counterburn deck.
#[must_use]
pub fn counterburn() -> Deck {
    #[allow(clippy::wildcard_imports)]
    use cards::*;
    Deck {
        main: copies(ISLAND, 8)
            .chain(copies(VOLCANIC_ISLAND, 4))
            .chain(copies(CITY_OF_BRASS, 4))
            .chain(copies(MISHRA_S_FACTORY, 4))
            .chain(copies(STRIP_MINE, 2))
            .chain(copies(BLACK_LOTUS, 1))
            .chain(copies(MOX_SAPPHIRE, 1))
            .chain(copies(MOX_RUBY, 1))
            .chain(copies(SOL_RING, 1))
            .chain(copies(SERENDIB_EFREET, 4))
            .chain(copies(SU_CHI, 3))
            .chain(copies(LIGHTNING_BOLT, 4))
            .chain(copies(CHAIN_LIGHTNING, 4))
            .chain(copies(PSIONIC_BLAST, 4))
            .chain(copies(EARTHQUAKE, 2))
            .chain(copies(COUNTERSPELL, 4))
            .chain(copies(MANA_DRAIN, 1))
            .chain(copies(BLACK_VISE, 3))
            .chain(copies(ANCESTRAL_RECALL, 1))
            .chain(copies(TIME_WALK, 1))
            .chain(copies(BRAINGEYSER, 1))
            .chain(copies(CHAOS_ORB, 1))
            .chain(copies(WHEEL_OF_FORTUNE, 1))
            .collect(),
        sideboard: copies(BLUE_ELEMENTAL_BLAST, 4)
            .chain(copies(RED_ELEMENTAL_BLAST, 4))
            .chain(copies(SHATTER, 3))
            .chain(copies(BLOOD_MOON, 2))
            .chain(copies(EARTHQUAKE, 2))
            .collect(),
    }
}

/// Returns a representative powered EC Lions/Dib blue-white tempo deck.
#[must_use]
pub fn lions_dib() -> Deck {
    #[allow(clippy::wildcard_imports)]
    use cards::*;
    Deck {
        main: copies(ISLAND, 5)
            .chain(copies(PLAINS, 5))
            .chain(copies(TUNDRA, 4))
            .chain(copies(CITY_OF_BRASS, 4))
            .chain(copies(MISHRA_S_FACTORY, 4))
            .chain(copies(STRIP_MINE, 2))
            .chain(copies(BLACK_LOTUS, 1))
            .chain(copies(MOX_SAPPHIRE, 1))
            .chain(copies(MOX_PEARL, 1))
            .chain(copies(SOL_RING, 1))
            .chain(copies(SAVANNAH_LIONS, 4))
            .chain(copies(ICATIAN_JAVELINEERS, 4))
            .chain(copies(WHITE_KNIGHT, 3))
            .chain(copies(SERENDIB_EFREET, 4))
            .chain(copies(SERRA_ANGEL, 2))
            .chain(copies(SWORDS_TO_PLOWSHARES, 4))
            .chain(copies(DISENCHANT, 3))
            .chain(copies(COUNTERSPELL, 4))
            .chain(copies(MANA_DRAIN, 1))
            .chain(copies(PSIONIC_BLAST, 1))
            .chain(copies(ANCESTRAL_RECALL, 1))
            .chain(copies(TIME_WALK, 1))
            .collect(),
        sideboard: copies(BLUE_ELEMENTAL_BLAST, 4)
            .chain(copies(RED_ELEMENTAL_BLAST, 4))
            .chain(copies(DIVINE_OFFERING, 3))
            .chain(copies(ARMAGEDDON, 2))
            .chain(copies(SERRA_ANGEL, 2))
            .collect(),
    }
}

/// Returns a representative powered BWR aggro deck.
#[must_use]
pub fn bwr_aggro() -> Deck {
    #[allow(clippy::wildcard_imports)]
    use cards::*;
    Deck {
        main: copies(BADLANDS, 3)
            .chain(copies(SCRUBLAND, 3))
            .chain(copies(PLATEAU, 3))
            .chain(copies(CITY_OF_BRASS, 4))
            .chain(copies(MISHRA_S_FACTORY, 4))
            .chain(copies(STRIP_MINE, 1))
            .chain(copies(BLACK_LOTUS, 1))
            .chain(copies(MOX_JET, 1))
            .chain(copies(MOX_PEARL, 1))
            .chain(copies(MOX_RUBY, 1))
            .chain(copies(SAVANNAH_LIONS, 4))
            .chain(copies(WHITE_KNIGHT, 4))
            .chain(copies(ORDER_OF_LEITBUR, 4))
            .chain(copies(BLACK_KNIGHT, 4))
            .chain(copies(HYPNOTIC_SPECTER, 3))
            .chain(copies(LIGHTNING_BOLT, 4))
            .chain(copies(CHAIN_LIGHTNING, 4))
            .chain(copies(SWORDS_TO_PLOWSHARES, 4))
            .chain(copies(DISENCHANT, 3))
            .chain(copies(CHAOS_ORB, 1))
            .chain(copies(DEMONIC_TUTOR, 1))
            .chain(copies(MIND_TWIST, 1))
            .chain(copies(FIREBALL, 1))
            .collect(),
        sideboard: copies(RED_ELEMENTAL_BLAST, 4)
            .chain(copies(BLUE_ELEMENTAL_BLAST, 3))
            .chain(copies(DIVINE_OFFERING, 3))
            .chain(copies(EARTHQUAKE, 2))
            .chain(copies(ARMAGEDDON, 2))
            .chain(copies(SENGIR_VAMPIRE, 1))
            .collect(),
    }
}

/// Returns a representative powered green-red aggro deck.
#[must_use]
pub fn gr_aggro() -> Deck {
    #[allow(clippy::wildcard_imports)]
    use cards::*;
    Deck {
        main: copies(FOREST, 6)
            .chain(copies(MOUNTAIN, 5))
            .chain(copies(TAIGA, 4))
            .chain(copies(MISHRA_S_FACTORY, 3))
            .chain(copies(PENDELHAVEN, 2))
            .chain(copies(CITY_OF_BRASS, 1))
            .chain(copies(STRIP_MINE, 1))
            .chain(copies(BLACK_LOTUS, 1))
            .chain(copies(MOX_RUBY, 1))
            .chain(copies(MOX_EMERALD, 1))
            .chain(copies(KIRD_APE, 4))
            .chain(copies(ARGOTHIAN_PIXIES, 4))
            .chain(copies(LLANOWAR_ELVES, 4))
            .chain(copies(SCRYB_SPRITES, 4))
            .chain(copies(ERHNAM_DJINN, 4))
            .chain(copies(LIGHTNING_BOLT, 4))
            .chain(copies(CHAIN_LIGHTNING, 4))
            .chain(copies(GIANT_GROWTH, 4))
            .chain(copies(BERSERK, 2))
            .chain(copies(WHEEL_OF_FORTUNE, 1))
            .collect(),
        sideboard: copies(RED_ELEMENTAL_BLAST, 3)
            .chain(copies(BLOOD_MOON, 2))
            .chain(copies(WHIRLING_DERVISH, 3))
            .chain(copies(SHATTER, 2))
            .chain(copies(EARTHQUAKE, 2))
            .chain(copies(STONE_RAIN, 2))
            .chain(copies(BLUE_ELEMENTAL_BLAST, 1))
            .collect(),
    }
}

/// Returns a representative powered Sedge Troll / Disk deck.
#[must_use]
pub fn troll_disk() -> Deck {
    #[allow(clippy::wildcard_imports)]
    use cards::*;
    Deck {
        main: copies(SWAMP, 7)
            .chain(copies(BADLANDS, 4))
            .chain(copies(VOLCANIC_ISLAND, 4))
            .chain(copies(CITY_OF_BRASS, 3))
            .chain(copies(MISHRA_S_FACTORY, 4))
            .chain(copies(STRIP_MINE, 1))
            .chain(copies(MAZE_OF_ITH, 1))
            .chain(copies(BLACK_LOTUS, 1))
            .chain(copies(MOX_JET, 1))
            .chain(copies(MOX_RUBY, 1))
            .chain(copies(SOL_RING, 1))
            .chain(copies(MANA_VAULT, 1))
            .chain(copies(SEDGE_TROLL, 4))
            .chain(copies(HYPNOTIC_SPECTER, 4))
            .chain(copies(SU_CHI, 4))
            .chain(copies(NEVINYRRALS_DISK, 4))
            .chain(copies(CHAOS_ORB, 1))
            .chain(copies(DARK_RITUAL, 4))
            .chain(copies(LIGHTNING_BOLT, 4))
            .chain(copies(SINKHOLE, 4))
            .chain(copies(STONE_RAIN, 2))
            .collect(),
        sideboard: copies(RED_ELEMENTAL_BLAST, 4)
            .chain(copies(SHATTER, 3))
            .chain(copies(BLOOD_MOON, 2))
            .chain(copies(TERROR, 2))
            .chain(copies(FIREBALL, 1))
            .chain(copies(DRAIN_LIFE, 3))
            .collect(),
    }
}

/// Returns a representative powered Jeskai tempo deck.
#[must_use]
pub fn jeskai_aggro() -> Deck {
    #[allow(clippy::wildcard_imports)]
    use cards::*;
    Deck {
        main: copies(ISLAND, 4)
            .chain(copies(PLAINS, 4))
            .chain(copies(TUNDRA, 4))
            .chain(copies(VOLCANIC_ISLAND, 4))
            .chain(copies(PLATEAU, 3))
            .chain(copies(CITY_OF_BRASS, 3))
            .chain(copies(MISHRA_S_FACTORY, 4))
            .chain(copies(STRIP_MINE, 1))
            .chain(copies(BLACK_LOTUS, 1))
            .chain(copies(MOX_PEARL, 1))
            .chain(copies(MOX_SAPPHIRE, 1))
            .chain(copies(MOX_RUBY, 1))
            .chain(copies(SOL_RING, 1))
            .chain(copies(SAVANNAH_LIONS, 4))
            .chain(copies(WHITE_KNIGHT, 3))
            .chain(copies(SERENDIB_EFREET, 4))
            .chain(copies(LIGHTNING_BOLT, 4))
            .chain(copies(CHAIN_LIGHTNING, 4))
            .chain(copies(PSIONIC_BLAST, 3))
            .chain(copies(SWORDS_TO_PLOWSHARES, 4))
            .chain(copies(COUNTERSPELL, 2))
            .collect(),
        sideboard: copies(RED_ELEMENTAL_BLAST, 4)
            .chain(copies(BLUE_ELEMENTAL_BLAST, 4))
            .chain(copies(DISENCHANT, 3))
            .chain(copies(DIVINE_OFFERING, 2))
            .chain(copies(ARMAGEDDON, 2))
            .collect(),
    }
}

/// Returns the Lion/Dib shell with the burn package used by current lists.
#[must_use]
pub fn lions_dib_bolt() -> Deck {
    #[allow(clippy::wildcard_imports)]
    use cards::*;
    Deck {
        main: copies(ISLAND, 5)
            .chain(copies(PLAINS, 5))
            .chain(copies(TUNDRA, 4))
            .chain(copies(CITY_OF_BRASS, 4))
            .chain(copies(MISHRA_S_FACTORY, 4))
            .chain(copies(STRIP_MINE, 2))
            .chain(copies(BLACK_LOTUS, 1))
            .chain(copies(MOX_SAPPHIRE, 1))
            .chain(copies(MOX_PEARL, 1))
            .chain(copies(SOL_RING, 1))
            .chain(copies(SAVANNAH_LIONS, 4))
            .chain(copies(ICATIAN_JAVELINEERS, 4))
            .chain(copies(WHITE_KNIGHT, 3))
            .chain(copies(SERENDIB_EFREET, 4))
            .chain(copies(SERRA_ANGEL, 1))
            .chain(copies(SWORDS_TO_PLOWSHARES, 4))
            .chain(copies(DISENCHANT, 2))
            .chain(copies(COUNTERSPELL, 2))
            .chain(copies(LIGHTNING_BOLT, 4))
            .chain(copies(CHAIN_LIGHTNING, 2))
            .chain(copies(ANCESTRAL_RECALL, 1))
            .chain(copies(TIME_WALK, 1))
            .collect(),
        sideboard: copies(BLUE_ELEMENTAL_BLAST, 4)
            .chain(copies(RED_ELEMENTAL_BLAST, 4))
            .chain(copies(DIVINE_OFFERING, 3))
            .chain(copies(ARMAGEDDON, 2))
            .chain(copies(SERRA_ANGEL, 2))
            .collect(),
    }
}

/// Backwards-compatible name for the built-in artifact deck.
#[must_use]
pub fn mono_red_atog() -> Deck {
    artifacts()
}

fn card(id: CardDefinitionId, name: &str, set: CardSet, behavior: CardBehavior) -> CardDefinition {
    card_with_behavior(id, name, set, false, behavior)
}

fn card_with_behavior(
    id: CardDefinitionId,
    name: &str,
    set: CardSet,
    is_basic_land: bool,
    behavior: CardBehavior,
) -> CardDefinition {
    CardDefinition {
        id,
        name: name.into(),
        set,
        is_basic_land,
        behavior,
    }
}

fn copies(id: CardDefinitionId, count: usize) -> impl Iterator<Item = CardDefinitionId> {
    std::iter::repeat_n(id, count)
}

#[cfg(test)]
mod tests {
    use super::{
        artifacts, bwr_aggro, catalog, counterburn, erhnamgeddon, goblins, gr_aggro, jeskai_aggro,
        lions_dib, lions_dib_bolt, mono_black, robots, sligh, the_deck, troll_disk, white_weenie,
    };
    use crate::rules;
    use crate::{CardBehavior, CardDefinitionId};

    #[test]
    fn built_in_decks_have_tournament_sizes() {
        for deck in all_decks() {
            assert_eq!(deck.main.len(), rules::MINIMUM_MAIN_DECK_SIZE);
            assert_eq!(deck.sideboard.len(), rules::MAXIMUM_SIDEBOARD_SIZE);
        }
    }

    #[test]
    fn built_in_decks_are_valid() {
        let catalog = catalog().unwrap();
        for deck in all_decks() {
            deck.validate(&catalog).unwrap();
        }
    }

    #[test]
    fn every_poc_card_has_engine_behavior() {
        let catalog = catalog().unwrap();
        for raw_id in 1..=128 {
            let card = catalog.get(CardDefinitionId(raw_id)).unwrap();
            assert_ne!(card.behavior, CardBehavior::Unsupported, "{}", card.name);
            assert!(
                !card.behavior.rules_text().is_empty(),
                "{} is missing rules text",
                card.name
            );
        }
    }

    fn all_decks() -> [crate::Deck; 16] {
        [
            goblins(),
            sligh(),
            artifacts(),
            robots(),
            the_deck(),
            mono_black(),
            white_weenie(),
            erhnamgeddon(),
            counterburn(),
            lions_dib(),
            bwr_aggro(),
            gr_aggro(),
            troll_disk(),
            jeskai_aggro(),
            lions_dib_bolt(),
            super::mono_red_atog(),
        ]
    }
}
