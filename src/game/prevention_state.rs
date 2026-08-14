use super::{GameObjectId, PlayerId};

/// Which sources a relational prevention answers. The variants name rules
/// rather than cards, but the list is deliberately closed: a prevention has
/// to survive a checkpoint, and a whole predicate has no serialised form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RelationalSourceFilter {
    CreaturesWithFlying,
    AttackingCreaturesWithoutFlying,
    Artifacts,
    UnblockedCreatures,
}

/// A turn-long damage-prevention rule whose affected objects are evaluated
/// when damage would be dealt rather than frozen when the spell resolves.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RelationalDamagePrevention {
    ToPlayerAndControlledCreatures(PlayerId),
    FromAllExcept(GameObjectId),
    /// Damage to one player from the sources a filter names. Unlike
    /// [`Self::ToPlayerAndControlledCreatures`] the player's creatures are
    /// not covered; the printed cards protect only their controller.
    ToPlayerFrom {
        player: PlayerId,
        source: RelationalSourceFilter,
    },
    /// Damage one named source would deal to one player lands on a named
    /// permanent instead. This is the turn-scoped, single-source form of the
    /// static bodyguard redirection.
    RedirectToPermanent {
        player: PlayerId,
        source: GameObjectId,
        destination: GameObjectId,
    },
}
