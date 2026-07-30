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
    Mountain,
    LightningBolt,
    Unsupported,
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
