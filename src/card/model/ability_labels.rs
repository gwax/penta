//! Semantic names for authored abilities and payment purposes.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AbilityLabel(pub &'static str);

impl AbilityLabel {
    pub const CYCLING: Self = Self("cycling");
    pub const CUMULATIVE_UPKEEP: Self = Self("cumulative upkeep");
}
