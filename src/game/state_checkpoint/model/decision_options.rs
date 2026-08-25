//! The wire shapes of a decision's options: what was offered, which cards
//! each option showed, and which zone they were in.
//!
//! Split out of `model.rs` for the source-size budget. Declared as a module
//! rather than included so the shapes it names stay explicit.

use serde::{Deserialize, Serialize};

use super::ObjectCharacteristicsSnapshot;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::game::state_checkpoint) struct DecisionOptionSnapshot {
    pub(in crate::game::state_checkpoint) id: u32,
    pub(in crate::game::state_checkpoint) label: String,
    pub(in crate::game::state_checkpoint) card: Option<DecisionCardSnapshot>,
    pub(in crate::game::state_checkpoint) members: Vec<DecisionCardSnapshot>,
    pub(in crate::game::state_checkpoint) ability_text: Option<String>,
    pub(in crate::game::state_checkpoint) zone: DecisionZoneSnapshot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::game::state_checkpoint) struct DecisionCardSnapshot {
    pub(in crate::game::state_checkpoint) object_id: u32,
    pub(in crate::game::state_checkpoint) characteristics: ObjectCharacteristicsSnapshot,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::game::state_checkpoint) enum DecisionZoneSnapshot {
    Hand,
    Graveyard,
    Battlefield,
    Stack,
    Library,
    Exile,
    OutsideGame,
    Command,
    DrawnThisStep,
    None,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::game::state_checkpoint) enum ZonePlacementSnapshot {
    Top,
    Bottom,
}
