//! Recipient stratification for static characteristic transformations.
//!
//! "All Forests are 1/1 creatures that are still lands" has to keep applying
//! as Forests come and go. The unified characteristic IR represents that as
//! independent card-type and power/toughness operations, and the ordinary
//! continuous-effect layer walkers derive those operations live.
//!
//! The recipient vocabulary remains narrow: it may ask about land types, the
//! card types below the operation being assembled, subtypes, which object is
//! the source, and control. None of those reads what these operations
//! supply -- a static animation adds the creature card type and may repaint
//! colour, and nothing here asks about either. A basic land subtype is the
//! exception the other way: the layer-4 operations do supply those, so a
//! subtype predicate naming one stays out. `runtime_support` uses this
//! same boundary, so a card that needs more is blocked rather than silently
//! misread. `card::catalog::validation` keeps a matching list for the
//! catalog-time refusal; the two are meant to say the same thing.

use super::{BasicLandType, CardType, Game, ObjectPredicateDef};

/// Whether a subtype name is one a static effect can itself supply. Basic
/// land subtypes are: the layer-4 basic-land-type operations set and remove
/// them, so a static animation asking about one could read what another
/// animation just wrote. Every other subtype is inert here.
fn subtype_is_supplied_by_a_static_effect(name: &str) -> bool {
    BasicLandType::ALL
        .iter()
        .any(|land_type| land_type.subtype() == name)
}

impl Game {
    /// Whether a static characteristic transformation's recipient predicate
    /// stays inside the stratified vocabulary above.
    #[must_use]
    pub fn static_animation_predicate_is_supported(predicate: ObjectPredicateDef) -> bool {
        match predicate {
            ObjectPredicateDef::Subtype(name) => !subtype_is_supplied_by_a_static_effect(name),
            ObjectPredicateDef::Any
            | ObjectPredicateDef::Source
            | ObjectPredicateDef::HasAnyBasicLandType(_)
            | ObjectPredicateDef::HasType(
                CardType::Land | CardType::Enchantment | CardType::Artifact,
            ) => true,
            ObjectPredicateDef::All(predicates) | ObjectPredicateDef::AnyOf(predicates) => {
                predicates
                    .iter()
                    .copied()
                    .all(Self::static_animation_predicate_is_supported)
            }
            ObjectPredicateDef::Not(predicate) => {
                Self::static_animation_predicate_is_supported(*predicate)
            }
            _ => false,
        }
    }
}
