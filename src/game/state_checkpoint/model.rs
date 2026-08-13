use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[allow(clippy::struct_excessive_bools)]
pub(super) struct GameSnapshot {
    pub(super) turns_started: [u32; 2],
    pub(super) consecutive_passes: u8,
    pub(super) attackers_declared: bool,
    pub(super) blockers_declared: bool,
    pub(super) untap_pending: bool,
    pub(super) cleanup_pending: bool,
    pub(super) mulligans: [u8; 2],
    pub(super) land_played_this_turn: [bool; 2],
    pub(super) tried_to_draw_from_empty_library: [bool; 2],
    pub(super) creature_died_this_turn: bool,
    pub(super) linked_exiles: Vec<[u32; 2]>,
    pub(super) sorcery_flash_grants: [u8; 2],
    pub(super) additional_combat_phases: u8,
    pub(super) noncreature_casts_locked: [bool; 2],
    pub(super) spells_cast_this_turn: [u16; 2],
    pub(super) spells_cast_last_turn: [u16; 2],
    pub(super) cards_drawn_this_turn: [u16; 2],
    pub(super) drawn_this_turn: [Vec<u32>; 2],
    pub(super) miracle_window: Option<u32>,
    pub(super) pending_combat_attackers: Vec<u32>,
    pub(super) combat_blocked_attackers: Vec<u32>,
    pub(super) extra_turns: Vec<usize>,
    pub(super) channel_active: [bool; 2],
    pub(super) skipped_turns: [u16; 2],
    pub(super) pregame: Option<PregameSnapshot>,
    pub(super) combat_damage_stage: CombatDamageStageSnapshot,
    pub(super) battlefield: Vec<PermanentSnapshot>,
    pub(super) emblems: Vec<EmblemSnapshot>,
    pub(super) stack: Vec<StackSnapshot>,
    pub(super) decision_state: Option<DecisionStateSnapshot>,
    pub(super) has_deferred_state: bool,
    pub(super) viewer: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(super) enum PregameSnapshot {
    Mulligan { seat: usize },
    Bottom { seat: usize },
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(super) enum CombatDamageStageSnapshot {
    #[default]
    NotStarted,
    Single,
    FirstStrike {
        combatants: Vec<u32>,
    },
    RegularAfterFirstStrike {
        combatants: Vec<u32>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub(super) struct PermanentSnapshot {
    pub(super) object_id: u32,
    pub(super) owner: usize,
    pub(super) timestamp: u64,
    pub(super) entered_controller_turn: u32,
    pub(super) power_bonus: i16,
    pub(super) toughness_bonus: i16,
    pub(super) unblockable_this_turn: bool,
    pub(super) combat_damage_prevented: bool,
    pub(super) combat_damage_dealt_by_prevented: bool,
    pub(super) control_reverts_to: Option<usize>,
    pub(super) chosen_player: Option<usize>,
    pub(super) destroy_at_end: bool,
    pub(super) counters: Vec<u16>,
    pub(super) attached_to: Option<u32>,
    pub(super) exile_instead_of_dying: bool,
    pub(super) combat_damage_assignment: Vec<CombatDamageAssignmentSnapshot>,
    pub(super) regeneration_shields: u8,
    pub(super) attacked_this_turn: bool,
    pub(super) attacks_this_turn: u8,
    pub(super) damage_sources: Vec<u32>,
    pub(super) dealt_damage_to_opponent_this_turn: bool,
    pub(super) deathtouch_damage: bool,
    pub(super) created_by: Option<u32>,
    pub(super) animation: Option<AnimationSnapshot>,
    pub(super) temporary_keywords: Vec<KeywordSnapshot>,
    pub(super) keywords_until_upkeep_of: Vec<UpkeepKeywordSnapshot>,
    pub(super) has_dynamic_characteristics: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CombatDamageAssignmentSnapshot {
    pub(super) recipient: String,
    pub(super) amount: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AnimationSnapshot {
    pub(super) power: i16,
    pub(super) toughness: i16,
    pub(super) types: String,
    pub(super) subtypes: Vec<String>,
    pub(super) all_creature_types: bool,
    pub(super) replaces_subtypes: bool,
    pub(super) loses_abilities: bool,
    pub(super) colors: Option<[bool; 5]>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum KeywordSnapshot {
    Flying,
    Trample,
    Haste,
    FirstStrike,
    DoubleStrike,
    Banding,
    Vigilance,
    Defender,
    Deathtouch,
    Lifelink,
    Reach,
    Flash,
    Hexproof,
    Shroud,
    Intimidate,
    Undying,
    Indestructible,
    AttacksEachCombatIfAble,
    Mountainwalk,
    Forestwalk,
    ProtectionFromWhite,
    ProtectionFromBlue,
    ProtectionFromBlack,
    ProtectionFromRed,
    ProtectionFromGreen,
    ProtectionFromColorless,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UpkeepKeywordSnapshot {
    pub(super) seat: usize,
    pub(super) keyword: KeywordSnapshot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct EmblemSnapshot {
    pub(super) object_id: u32,
    pub(super) definition: u16,
    pub(super) owner: usize,
    pub(super) presented_part_id: u8,
    pub(super) timestamp: u64,
    pub(super) entered_controller_turn: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StackSnapshot {
    pub(super) object_id: u32,
    pub(super) owner: usize,
    pub(super) ability_payload: Option<StackAbilitySnapshot>,
    pub(super) requires_retired_object: bool,
    pub(super) has_runtime_overrides: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StackAbilitySnapshot {
    pub(super) ability_locator: Option<AbilityLocator>,
    pub(super) target_selections: Vec<TargetSelectionSnapshot>,
    pub(super) context: TriggerContextSnapshot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AbilityLocator {
    pub(super) definition: u16,
    pub(super) part_id: u8,
    pub(super) ability_id: u8,
    pub(super) nested: Vec<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TargetSelectionSnapshot {
    pub(super) slot_id: u8,
    pub(super) targets: Vec<TargetSnapshot>,
    pub(super) amounts: Vec<u16>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum TargetSnapshot {
    Player { seat: SeatSnapshot },
    Card { object_id: u32 },
    Permanent { object_id: u32 },
    Spell { object_id: u32 },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(super) enum SeatSnapshot {
    #[serde(rename = "p1")]
    One,
    #[serde(rename = "p2")]
    Two,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TriggerContextSnapshot {
    pub(super) object: Option<u32>,
    pub(super) object_controller: Option<usize>,
    pub(super) event_player: Option<usize>,
    pub(super) amount: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DecisionStateSnapshot {
    pub(super) preference: DecisionPreferenceSnapshot,
    pub(super) continuation: DecisionContinuationSnapshot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub(super) enum DecisionPreferenceSnapshot {
    Name(String),
    PreferOption {
        #[serde(rename = "preferOption")]
        prefer_option: u32,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum DecisionContinuationSnapshot {
    BasicLandTypeTextChange {
        target: TargetSnapshot,
    },
    MiracleReveal {
        card: u32,
    },
    PileSplit {
        owner: usize,
    },
    PileChoice {
        first: Vec<u32>,
        second: Vec<u32>,
    },
    SacrificeOfChoice {
        optional: bool,
    },
    DestroyOfChoice {
        can_regenerate: bool,
    },
    TimeVault {
        permanent: u32,
        remaining: Vec<u32>,
    },
    SylvanOffer {
        player: usize,
    },
    SylvanSelect {
        player: usize,
        candidates: Vec<u32>,
        choices_left: usize,
    },
    SylvanMode {
        player: usize,
        card: u32,
        candidates: Vec<u32>,
        choices_left: usize,
    },
    TetravusDetach {
        source: u32,
    },
    TetravusAssemble {
        source: u32,
    },
}
