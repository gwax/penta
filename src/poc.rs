//! Built-in red/artifact card corpus and representative Old School decks.

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
        ANKH_OF_MISHRA, ATOG, BALL_LIGHTNING, BLACK_VISE, BLOOD_MOON, CHAIN_LIGHTNING,
        COPPER_TABLET, DETONATE, FIREBALL, FORK, GLASSES_OF_URZA, IRON_STAR, LIGHTNING_BOLT,
        MOUNTAIN, RED_ELEMENTAL_BLAST, SHATTER, SMOKE, STONE_GIANT, SU_CHI, WINTER_ORB,
    };
    use cards::{
        BLACK_LOTUS, CHAOS_ORB, DRAGON_WHELP, GOBLIN_BALLOON_BRIGADE, GOBLIN_DIGGING_TEAM,
        GOBLIN_GRENADE, GOBLIN_KING, GOBLINS_OF_THE_FLARG, GRANITE_GARGOYLE, IRONCLAW_ORCS,
        MISHRA_S_FACTORY, MOX_EMERALD, MOX_JET, MOX_PEARL, MOX_RUBY, MOX_SAPPHIRE,
        ORCISH_MECHANICS, SOL_RING, STRIP_MINE, WHEEL_OF_FORTUNE,
    };

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
    use super::{artifacts, catalog, goblins, sligh};
    use crate::rules;
    use crate::{CardBehavior, CardDefinitionId};

    #[test]
    fn built_in_decks_have_tournament_sizes() {
        for deck in [goblins(), sligh(), artifacts()] {
            assert_eq!(deck.main.len(), rules::MINIMUM_MAIN_DECK_SIZE);
            assert_eq!(deck.sideboard.len(), rules::MAXIMUM_SIDEBOARD_SIZE);
        }
    }

    #[test]
    fn built_in_decks_are_valid() {
        let catalog = catalog().unwrap();
        for deck in [goblins(), sligh(), artifacts()] {
            deck.validate(&catalog).unwrap();
        }
    }

    #[test]
    fn every_poc_card_has_engine_behavior() {
        let catalog = catalog().unwrap();
        for raw_id in 1..=40 {
            let card = catalog.get(CardDefinitionId(raw_id)).unwrap();
            assert_ne!(card.behavior, CardBehavior::Unsupported, "{}", card.name);
            assert!(
                !card.behavior.rules_text().is_empty(),
                "{} is missing rules text",
                card.name
            );
        }
    }
}
