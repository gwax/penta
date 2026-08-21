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
