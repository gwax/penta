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
    AnkhOfMishra,
    Atog,
    BallLightning,
    BlackLotus,
    BlackVise,
    BloodMoon,
    ChainLightning,
    ChaosOrb,
    CopperTablet,
    Detonate,
    DragonWhelp,
    Fireball,
    Fork,
    GlassesOfUrza,
    GoblinBalloonBrigade,
    GoblinDiggingTeam,
    GoblinGrenade,
    GoblinKing,
    GoblinsOfTheFlarg,
    GraniteGargoyle,
    IronStar,
    IronclawOrcs,
    Mountain,
    LightningBolt,
    MishrasFactory,
    MoxEmerald,
    MoxJet,
    MoxPearl,
    MoxRuby,
    MoxSapphire,
    OrcishMechanics,
    RedElementalBlast,
    Shatter,
    Smoke,
    SolRing,
    StoneGiant,
    StripMine,
    SuChi,
    WheelOfFortune,
    WinterOrb,
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
    pub red: u16,
    pub variable_x: bool,
}

impl ManaCost {
    #[must_use]
    pub const fn new(generic: u16, red: u16) -> Self {
        Self {
            generic,
            red,
            variable_x: false,
        }
    }

    #[must_use]
    pub const fn with_x(red: u16) -> Self {
        Self {
            generic: 0,
            red,
            variable_x: true,
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
    #[must_use]
    pub const fn kind(self) -> CardKind {
        match self {
            Self::Mountain | Self::MishrasFactory | Self::StripMine => CardKind::Land,
            Self::Atog
            | Self::BallLightning
            | Self::DragonWhelp
            | Self::GoblinBalloonBrigade
            | Self::GoblinDiggingTeam
            | Self::GoblinKing
            | Self::GoblinsOfTheFlarg
            | Self::GraniteGargoyle
            | Self::IronclawOrcs
            | Self::OrcishMechanics
            | Self::StoneGiant => CardKind::Creature,
            Self::SuChi => CardKind::ArtifactCreature,
            Self::AnkhOfMishra
            | Self::BlackLotus
            | Self::BlackVise
            | Self::ChaosOrb
            | Self::CopperTablet
            | Self::GlassesOfUrza
            | Self::IronStar
            | Self::MoxEmerald
            | Self::MoxJet
            | Self::MoxPearl
            | Self::MoxRuby
            | Self::MoxSapphire
            | Self::SolRing
            | Self::WinterOrb
            | Self::Unsupported => CardKind::Artifact,
            Self::BloodMoon | Self::Smoke => CardKind::Enchantment,
            Self::Fork | Self::LightningBolt | Self::RedElementalBlast | Self::Shatter => {
                CardKind::Instant
            }
            Self::ChainLightning
            | Self::Detonate
            | Self::Fireball
            | Self::GoblinGrenade
            | Self::WheelOfFortune => CardKind::Sorcery,
        }
    }

    #[must_use]
    pub const fn mana_cost(self) -> ManaCost {
        match self {
            Self::Mountain
            | Self::MishrasFactory
            | Self::StripMine
            | Self::BlackLotus
            | Self::MoxEmerald
            | Self::MoxJet
            | Self::MoxPearl
            | Self::MoxRuby
            | Self::MoxSapphire => ManaCost::new(0, 0),
            Self::SolRing | Self::BlackVise | Self::GlassesOfUrza | Self::IronStar => {
                ManaCost::new(1, 0)
            }
            Self::AnkhOfMishra | Self::ChaosOrb | Self::CopperTablet | Self::WinterOrb => {
                ManaCost::new(2, 0)
            }
            Self::Smoke | Self::Fork => ManaCost::new(0, 2),
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
            Self::Detonate | Self::Fireball => ManaCost::with_x(1),
            Self::SuChi => ManaCost::new(4, 0),
            Self::Unsupported => ManaCost::new(u16::MAX, u16::MAX),
        }
    }

    #[must_use]
    pub const fn creature_stats(self) -> Option<CreatureStats> {
        match self {
            Self::Atog => Some(CreatureStats {
                power: 1,
                toughness: 2,
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
            Self::GoblinBalloonBrigade
            | Self::GoblinDiggingTeam
            | Self::GoblinsOfTheFlarg
            | Self::OrcishMechanics => Some(CreatureStats {
                power: 1,
                toughness: 1,
                haste: false,
                trample: false,
            }),
            Self::GoblinKing | Self::GraniteGargoyle | Self::IronclawOrcs => Some(CreatureStats {
                power: 2,
                toughness: 2,
                haste: false,
                trample: false,
            }),
            Self::StoneGiant => Some(CreatureStats {
                power: 3,
                toughness: 4,
                haste: false,
                trample: false,
            }),
            Self::SuChi => Some(CreatureStats {
                power: 4,
                toughness: 4,
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
        matches!(self, Self::DragonWhelp | Self::GraniteGargoyle)
    }

    #[must_use]
    pub const fn has_mountainwalk(self) -> bool {
        matches!(self, Self::GoblinsOfTheFlarg)
    }

    #[must_use]
    pub const fn is_red(self) -> bool {
        matches!(
            self,
            Self::Atog
                | Self::BallLightning
                | Self::BloodMoon
                | Self::ChainLightning
                | Self::Detonate
                | Self::DragonWhelp
                | Self::Fireball
                | Self::Fork
                | Self::GoblinBalloonBrigade
                | Self::GoblinDiggingTeam
                | Self::GoblinGrenade
                | Self::GoblinKing
                | Self::GoblinsOfTheFlarg
                | Self::GraniteGargoyle
                | Self::IronclawOrcs
                | Self::LightningBolt
                | Self::OrcishMechanics
                | Self::RedElementalBlast
                | Self::Shatter
                | Self::Smoke
                | Self::StoneGiant
                | Self::WheelOfFortune
        )
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
