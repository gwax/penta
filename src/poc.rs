//! Built-in card corpus and deck for the first playable proof of concept.
//!
//! The corpus is based on a representative unpowered Mono-Red Atog list. Its
//! lone Strip Mine is replaced by a Mountain so every card is red, an artifact,
//! or a basic Mountain.

use crate::CardDefinitionId;
use crate::card::{CardCatalog, CardDefinition, CardSet, CatalogError};
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
}

/// Builds the complete card catalog required by [`mono_red_atog`].
///
/// # Errors
///
/// Returns [`CatalogError`] if the built-in IDs or names are accidentally
/// duplicated. Such an error indicates a bug in this crate.
pub fn catalog() -> Result<CardCatalog, CatalogError> {
    use cards::{
        ANKH_OF_MISHRA, ATOG, BALL_LIGHTNING, BLACK_VISE, BLOOD_MOON, CHAIN_LIGHTNING,
        COPPER_TABLET, DETONATE, FIREBALL, FORK, GLASSES_OF_URZA, IRON_STAR, LIGHTNING_BOLT,
        MOUNTAIN, RED_ELEMENTAL_BLAST, SHATTER, SMOKE, STONE_GIANT, SU_CHI, WINTER_ORB,
    };

    CardCatalog::new([
        card(ANKH_OF_MISHRA, "Ankh of Mishra", CardSet::Alpha, false),
        card(ATOG, "Atog", CardSet::Antiquities, false),
        card(BALL_LIGHTNING, "Ball Lightning", CardSet::TheDark, false),
        card(BLACK_VISE, "Black Vise", CardSet::Alpha, false),
        card(BLOOD_MOON, "Blood Moon", CardSet::TheDark, false),
        card(CHAIN_LIGHTNING, "Chain Lightning", CardSet::Legends, false),
        card(COPPER_TABLET, "Copper Tablet", CardSet::Alpha, false),
        card(DETONATE, "Detonate", CardSet::Antiquities, false),
        card(FIREBALL, "Fireball", CardSet::Alpha, false),
        card(FORK, "Fork", CardSet::Alpha, false),
        card(GLASSES_OF_URZA, "Glasses of Urza", CardSet::Alpha, false),
        card(IRON_STAR, "Iron Star", CardSet::Alpha, false),
        card(LIGHTNING_BOLT, "Lightning Bolt", CardSet::Alpha, false),
        card(MOUNTAIN, "Mountain", CardSet::Alpha, true),
        card(
            RED_ELEMENTAL_BLAST,
            "Red Elemental Blast",
            CardSet::Alpha,
            false,
        ),
        card(SHATTER, "Shatter", CardSet::Alpha, false),
        card(SMOKE, "Smoke", CardSet::Alpha, false),
        card(STONE_GIANT, "Stone Giant", CardSet::Alpha, false),
        card(SU_CHI, "Su-Chi", CardSet::Antiquities, false),
        card(WINTER_ORB, "Winter Orb", CardSet::Alpha, false),
    ])
}

/// Returns the 60-card main deck and 15-card sideboard that define the POC.
#[must_use]
pub fn mono_red_atog() -> Deck {
    use cards::{
        ANKH_OF_MISHRA, ATOG, BALL_LIGHTNING, BLACK_VISE, BLOOD_MOON, CHAIN_LIGHTNING,
        COPPER_TABLET, DETONATE, FIREBALL, FORK, GLASSES_OF_URZA, IRON_STAR, LIGHTNING_BOLT,
        MOUNTAIN, RED_ELEMENTAL_BLAST, SHATTER, SMOKE, STONE_GIANT, SU_CHI, WINTER_ORB,
    };

    Deck {
        main: copies(ANKH_OF_MISHRA, 4)
            .chain(copies(ATOG, 4))
            .chain(copies(BALL_LIGHTNING, 4))
            .chain(copies(BLACK_VISE, 4))
            .chain(copies(CHAIN_LIGHTNING, 4))
            .chain(copies(COPPER_TABLET, 4))
            .chain(copies(DETONATE, 2))
            .chain(copies(FIREBALL, 1))
            .chain(copies(FORK, 2))
            .chain(copies(GLASSES_OF_URZA, 2))
            .chain(copies(LIGHTNING_BOLT, 4))
            .chain(copies(MOUNTAIN, 16))
            .chain(copies(SHATTER, 2))
            .chain(copies(SMOKE, 2))
            .chain(copies(SU_CHI, 3))
            .chain(copies(WINTER_ORB, 2))
            .collect(),
        sideboard: copies(BLOOD_MOON, 3)
            .chain(copies(DETONATE, 2))
            .chain(copies(IRON_STAR, 4))
            .chain(copies(RED_ELEMENTAL_BLAST, 4))
            .chain(copies(STONE_GIANT, 2))
            .collect(),
    }
}

fn card(id: CardDefinitionId, name: &str, set: CardSet, is_basic_land: bool) -> CardDefinition {
    CardDefinition {
        id,
        name: name.into(),
        set,
        is_basic_land,
    }
}

fn copies(id: CardDefinitionId, count: usize) -> impl Iterator<Item = CardDefinitionId> {
    std::iter::repeat_n(id, count)
}

#[cfg(test)]
mod tests {
    use super::{catalog, mono_red_atog};
    use crate::rules;

    #[test]
    fn built_in_deck_has_tournament_sizes() {
        let deck = mono_red_atog();
        assert_eq!(deck.main.len(), rules::MINIMUM_MAIN_DECK_SIZE);
        assert_eq!(deck.sideboard.len(), rules::MAXIMUM_SIDEBOARD_SIZE);
    }

    #[test]
    fn built_in_deck_is_valid() {
        mono_red_atog().validate(&catalog().unwrap()).unwrap();
    }
}
