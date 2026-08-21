use crate::card::AbilityTargetDef;

use super::ScopedEffect;

/// The executable target/effect layout obtained after one concrete set of
/// spell modes has been selected. Building both vectors together keeps their
/// positional mapping atomic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SelectedSpellPlan {
    pub(super) target_defs: Vec<AbilityTargetDef>,
    pub(super) mode_effects: Vec<ScopedEffect>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CastSourceZone {
    Hand,
    Graveyard,
    /// A card on an adventure, which its owner may cast from exile as the
    /// creature it is on the other half.
    Exile,
    /// The top card of the caster's own library, for the permissions that
    /// reach up there. Only ever the topmost one: a permission to play from
    /// the top of a library names one card, not the library.
    LibraryTop,
}

impl CastSourceZone {
    /// The zone a card-facing clause names. The top of a library is the
    /// library: "cast from your library" is what a player would say, and no
    /// printed clause distinguishes the topmost card from the rest.
    pub(super) const fn zone(self) -> crate::card::ZoneKind {
        match self {
            Self::Hand => crate::card::ZoneKind::Hand,
            Self::Graveyard => crate::card::ZoneKind::Graveyard,
            Self::Exile => crate::card::ZoneKind::Exile,
            Self::LibraryTop => crate::card::ZoneKind::Library,
        }
    }

    /// The stable wire label for this zone, for a checkpoint that names it
    /// rather than storing an enum whose order could move.
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Hand => "hand",
            Self::Graveyard => "graveyard",
            Self::Exile => "exile",
            Self::LibraryTop => "libraryTop",
        }
    }
}

/// The inverse of [`CastSourceZone::label`]. An unknown label reads back as
/// nothing, which is what a spell nobody cast carries anyway.
pub(super) fn cast_source_zone_from_label(label: &str) -> Option<CastSourceZone> {
    match label {
        "hand" => Some(CastSourceZone::Hand),
        "graveyard" => Some(CastSourceZone::Graveyard),
        "exile" => Some(CastSourceZone::Exile),
        "libraryTop" => Some(CastSourceZone::LibraryTop),
        _ => None,
    }
}
