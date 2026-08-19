//! Stable definition IDs appended for the Premodern Angry Hermit list.

use crate::CardDefinitionId;

pub const HERMIT_DRUID: CardDefinitionId = CardDefinitionId(2070);
pub const STIFLE: CardDefinitionId = CardDefinitionId(2071);
pub const SHALLOW_GRAVE: CardDefinitionId = CardDefinitionId(2072);
pub const REFLECTING_POOL: CardDefinitionId = CardDefinitionId(2073);
pub const KROSAN_RECLAMATION: CardDefinitionId = CardDefinitionId(2074);

/// The body every face-down permanent presents: a 2/2 colourless creature
/// with no name and no abilities (CR 708.2). It is a rules lookup rather than
/// an identity -- the physical card underneath keeps its own definition -- so
/// nothing ever owns this one.
pub const FACE_DOWN_CREATURE: CardDefinitionId = CardDefinitionId(2075);
pub const GILDED_DRAKE: CardDefinitionId = CardDefinitionId(2083);
pub const PHYREXIAN_DREADNOUGHT: CardDefinitionId = CardDefinitionId(2085);
