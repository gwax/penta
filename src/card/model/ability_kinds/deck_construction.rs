// Clauses that are rules of deck construction rather than of play.
//
// Split out of `ability_kinds.rs` for the source-size budget, and kept
// distinct from everything next door because these clauses never do anything
// during a game: they are read while a deck is being assembled and are silent
// once it is shuffled. Included textually, so the imports here are that
// module's.

/// A permission a card grants the deck it is built into.
///
/// A card may print more than one over time, so this is an enumeration rather
/// than a flag; each variant names one printed sentence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeckConstructionDef {
    /// "<This card> can be your commander." A card that is not a legendary
    /// creature may still be the commander of a Commander deck (CR 903.3b).
    /// The permission is spent entirely at deck construction: the card does
    /// nothing extra once the game begins.
    MayBeCommander,
}
