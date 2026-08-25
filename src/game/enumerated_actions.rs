//! The legal-action list a game keeps between enumerating and applying.

use super::{Action, PlayerId};

/// A legal-action list the engine handed out and can still vouch for.
///
/// `apply` validates an ordinary action with `legal_actions(player).contains`,
/// a full re-enumeration of the list a caller usually chose from a moment
/// earlier. Holding on to that list lets the check read it instead, which is
/// most of the cost of a ply in a search that enumerates and then applies.
///
/// Every mutating entry point takes this, so it can only ever hold an
/// enumeration made between two mutations: taking it is how it is used, which
/// is also how it is cleared. Only [`Game::enumerate_legal_actions`] fills it,
/// and that needs `&mut Game`, so nothing inside a resolution can leave a
/// half-applied position's list behind.
#[derive(Debug, Default)]
pub(super) struct EnumeratedActions(pub(super) Option<(PlayerId, Vec<Action>)>);

impl Clone for EnumeratedActions {
    /// A clone starts with nothing enumerated. A search clones a position per
    /// rollout and would deep-copy a list of hundreds of actions to carry a
    /// memo the clone re-derives on its first enumeration anyway.
    fn clone(&self) -> Self {
        Self(None)
    }
}
