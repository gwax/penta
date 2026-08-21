// The small enums an effect names but that are not effects themselves.
//
// Each is one word of vocabulary several clauses reach for: what a follow-up
// reads off a sacrifice, which abilities a removal names, which turns a
// clause means, and who may watch a choice. Included textually into
// `effects.rs`, so the imports here are the parent module's.

/// Which characteristic of a sacrificed permanent a follow-up reads.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SacrificedAmountDef {
    Power,
    Toughness,
}

/// A reusable selector for ability-removing continuous effects.
///
/// `Any` supports ordinary "loses all abilities" effects. The keyword form is
/// also the seam needed by text-changing cards that replace one landwalk
/// ability with another without treating the whole rules box as opaque text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AbilityPredicateDef {
    Any,
    Keyword(KeywordAbility),
    /// Every "bands with other" ability, whatever quality it names. Two cards
    /// strip them all at once, and neither says which qualities it means.
    AnyBandsWithOther,
}
/// An event that a replacement ability can modify before it is committed.
///
/// Replacement events deliberately have their own vocabulary rather than
/// reusing [`TriggerEventDef`]: triggers observe events that have already
/// happened, while replacement abilities inspect and modify prospective
/// events.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TurnKindDef {
    /// Match a regular or extra turn.
    Any,
    /// Match only the next turn in the ordinary turn order.
    Regular,
    /// Match only a turn created by a spell or ability.
    Extra,
}

impl TurnKindDef {
    #[must_use]
    pub const fn matches(self, turn: Self) -> bool {
        matches!(
            (self, turn),
            (Self::Any, _) | (Self::Regular, Self::Regular) | (Self::Extra, Self::Extra)
        )
    }
}

/// Who may observe a pending choice and its available options.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ChoiceVisibilityDef {
    Public,
    Private,
}
