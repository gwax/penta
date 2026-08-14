//! Recipient stratification for static characteristic transformations.
//!
//! "All Forests are 1/1 creatures that are still lands" has to keep applying
//! as Forests come and go. The unified characteristic IR represents that as
//! independent card-type and power/toughness operations, and the ordinary
//! continuous-effect layer walkers derive those operations live.
//!
//! The recipient vocabulary remains narrow: it may ask about land types, the
//! card types below the operation being assembled, and control. None of those
//! reads what these operations supply. `runtime_support` uses this same
//! boundary, so a card that needs more is blocked rather than silently
//! misread.

use super::{CardType, Game, ObjectPredicateDef};

impl Game {
    /// Whether a static characteristic transformation's recipient predicate
    /// stays inside the stratified vocabulary above.
    #[must_use]
    pub fn static_animation_predicate_is_supported(predicate: ObjectPredicateDef) -> bool {
        match predicate {
            ObjectPredicateDef::Any
            | ObjectPredicateDef::HasAnyBasicLandType(_)
            | ObjectPredicateDef::HasType(CardType::Land) => true,
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
