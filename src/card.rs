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
    /// Returns concise rules text for the behavior implemented by the simulator.
    #[must_use]
    pub const fn rules_text(self) -> &'static str {
        match self {
            Self::AnkhOfMishra => {
                "Whenever a land enters, Ankh of Mishra deals 2 damage to its controller."
            }
            Self::Atog => "Sacrifice an artifact: Atog gets +2/+2 until end of turn.",
            Self::BallLightning => {
                "Trample, haste. Sacrifice Ball Lightning at the beginning of the end step."
            }
            Self::BlackLotus => "Tap, sacrifice Black Lotus: Add RRR.",
            Self::BlackVise => {
                "As Black Vise enters, choose an opponent. At their upkeep, it deals 1 damage for each card in their hand beyond four."
            }
            Self::BloodMoon => "Nonbasic lands are Mountains.",
            Self::ChainLightning => {
                "Deal 3 damage to any target. That target's controller may pay RR to copy it and choose a new target."
            }
            Self::ChaosOrb => {
                "1, Tap: Choose a permanent. On resolution, destroy it and Chaos Orb if Chaos Orb is still on the battlefield."
            }
            Self::CopperTablet => {
                "At the beginning of each player's upkeep, Copper Tablet deals 1 damage to that player."
            }
            Self::Detonate => {
                "Destroy target artifact with mana value X. Its controller takes X damage."
            }
            Self::DragonWhelp => {
                "Flying. R: +1/+0 until end of turn. If activated four or more times this turn, destroy it at the end step."
            }
            Self::Fireball => {
                "Deal X damage divided evenly among the chosen targets. Each target beyond the first costs 1 more."
            }
            Self::Fork => {
                "Copy target instant or sorcery. You may choose new targets for the copy."
            }
            Self::GlassesOfUrza => "Tap: Look at target player's hand.",
            Self::GoblinBalloonBrigade => "R: Gains flying until end of turn.",
            Self::GoblinDiggingTeam => "Sacrifice Goblin Digging Team: Destroy target Wall.",
            Self::GoblinGrenade => {
                "As an additional cost, sacrifice a Goblin. Deal 5 damage to any target."
            }
            Self::GoblinKing => "Other Goblins get +1/+1 and have mountainwalk.",
            Self::GoblinsOfTheFlarg => "Mountainwalk.",
            Self::GraniteGargoyle => "Flying. R: Gets +0/+1 until end of turn.",
            Self::IronStar => {
                "Whenever a red spell is cast, you may pay 1. If you do, gain 1 life."
            }
            Self::IronclawOrcs => "Can't block creatures with power 2 or greater.",
            Self::Mountain => "Tap: Add R.",
            Self::LightningBolt => "Deal 3 damage to any target.",
            Self::MishrasFactory => {
                "Tap: Add 1. 1: Becomes a 2/2 Assembly-Worker artifact creature until end of turn. Tap: Target Assembly-Worker gets +1/+1 until end of turn."
            }
            Self::MoxRuby => "Tap: Add R.",
            Self::MoxEmerald | Self::MoxJet | Self::MoxPearl | Self::MoxSapphire => "Tap: Add 1.",
            Self::OrcishMechanics => "Tap, sacrifice an artifact: Deal 2 damage to any target.",
            Self::RedElementalBlast => {
                "Counter target blue spell or destroy target blue permanent."
            }
            Self::Shatter => "Destroy target artifact.",
            Self::Smoke => "Players can't untap more than one creature during their untap steps.",
            Self::SolRing => "Tap: Add 2.",
            Self::StoneGiant => {
                "Tap: A smaller creature you control gains flying until end of turn. Destroy it at the end step."
            }
            Self::StripMine => "Tap, sacrifice Strip Mine: Destroy target land.",
            Self::SuChi => "When Su-Chi dies, add 4.",
            Self::WheelOfFortune => "Each player discards their hand, then draws seven cards.",
            Self::WinterOrb => {
                "While untapped, players can't untap more than one land during their untap steps."
            }
            Self::Unsupported => "Rules text is not implemented.",
        }
    }

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
