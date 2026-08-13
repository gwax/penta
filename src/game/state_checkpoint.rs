use std::collections::{BTreeMap, VecDeque};

use serde_json::Value;

use super::{
    CardInstance, CharacteristicSource, CombatDamageStage, ContinuousEffectTimestamp, CounterKind,
    Game, GameEvent, GameObjectId, GameStack, ObjectBacking, Permanent, PlayerId, PlayerState,
    Pregame, ReplayRng, StackAbilityPayload, StackObject, StackObjectKind, Step, TriggerContext,
};
use crate::card::{BasicLandType, DeclarativeAbilityDef, SpellForm};
use crate::casting::{CastChoices, CastSignature, CostConfiguration, TargetSelection};
use crate::{
    AbilityId, AbilityOrigin, AdditionalCostId, AlternativeCostId, AttackDefender, CardCatalog,
    CardDefinitionId, CardPartId, Format, GameObjectId as PublicGameObjectId, GrantId, ModeId,
    PlayOptionId, Target, TargetSlotId,
};

mod decision;
mod emblem;
mod model;
mod semantics;
mod stack;

use decision::{decision_snapshot, parse_pending_decision};
use emblem::{emblem_snapshot, parse_emblems};
use model::{
    CombatDamageAssignmentSnapshot, CombatDamageStageSnapshot, GameSnapshot, PermanentSnapshot,
    PregameSnapshot, StackSnapshot, UpkeepKeywordSnapshot,
};
use semantics::{
    ability_locator, animation_snapshot, catalog_ability, catalog_animation, keyword_snapshot,
    parse_keyword,
};
use stack::{parse_stack, stack_ability_snapshot, stack_object_requires_retired};

impl Game {
    /// Hidden-safe rules bookkeeping needed to use an observation as a
    /// current-state checkpoint. Presentation fields stay in the ordinary
    /// observation; this object carries the state which cannot be inferred
    /// reliably from them.
    #[allow(clippy::too_many_lines)]
    fn snapshot(&self, viewer: PlayerId) -> GameSnapshot {
        let decision_state = (self.pending_decisions.len() == 1)
            .then(|| decision_snapshot(&self.pending_decisions[0]))
            .flatten();
        let has_unsupported_decision =
            !self.pending_decisions.is_empty() && decision_state.is_none();
        let visible_drawn_this_turn = [PlayerId::One, PlayerId::Two].map(|player| {
            if player == viewer {
                self.drawn_this_turn[player.index()]
                    .iter()
                    .map(|id| id.0)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        });
        GameSnapshot {
            turns_started: self.turns_started,
            consecutive_passes: self.consecutive_passes,
            attackers_declared: self.attackers_declared,
            blockers_declared: self.blockers_declared,
            untap_pending: self.untap_pending,
            cleanup_pending: self.cleanup_pending,
            mulligans: self.mulligans,
            land_played_this_turn: [
                self.players[0].land_played_this_turn,
                self.players[1].land_played_this_turn,
            ],
            tried_to_draw_from_empty_library: [
                self.players[0].tried_to_draw_from_empty_library,
                self.players[1].tried_to_draw_from_empty_library,
            ],
            creature_died_this_turn: self.creature_died_this_turn,
            linked_exiles: self
                .linked_exiles
                .iter()
                .map(|(source, card)| [source.0, card.0])
                .collect(),
            sorcery_flash_grants: self.sorcery_flash_grants,
            additional_combat_phases: self.additional_combat_phases,
            noncreature_casts_locked: self.noncreature_casts_locked,
            spells_cast_this_turn: self.spells_cast_this_turn,
            spells_cast_last_turn: self.spells_cast_last_turn,
            cards_drawn_this_turn: self.cards_drawn_this_turn,
            drawn_this_turn: visible_drawn_this_turn,
            miracle_window: self
                .miracle_window
                .filter(|id| {
                    self.players[viewer.index()]
                        .hand
                        .iter()
                        .any(|card| card.id == *id)
                })
                .map(|id| id.0),
            pending_combat_attackers: self
                .pending_combat_attackers
                .iter()
                .map(|id| id.0)
                .collect(),
            combat_blocked_attackers: self
                .combat_blocked_attackers
                .iter()
                .map(|id| id.0)
                .collect(),
            extra_turns: self
                .extra_turns
                .iter()
                .map(|player| player.index())
                .collect(),
            channel_active: self.channel_active,
            skipped_turns: self.skipped_turns,
            pregame: self.pregame.map(|pregame| match pregame {
                Pregame::Mulligan(player) => PregameSnapshot::Mulligan {
                    seat: player.index(),
                },
                Pregame::Bottom(player) => PregameSnapshot::Bottom {
                    seat: player.index(),
                },
            }),
            combat_damage_stage: match &self.combat_damage_stage {
                CombatDamageStage::NotStarted => CombatDamageStageSnapshot::NotStarted,
                CombatDamageStage::Single => CombatDamageStageSnapshot::Single,
                CombatDamageStage::FirstStrike {
                    strike_wave_combatants,
                } => CombatDamageStageSnapshot::FirstStrike {
                    combatants: strike_wave_combatants.iter().map(|id| id.0).collect(),
                },
                CombatDamageStage::RegularAfterFirstStrike {
                    strike_wave_combatants,
                } => CombatDamageStageSnapshot::RegularAfterFirstStrike {
                    combatants: strike_wave_combatants.iter().map(|id| id.0).collect(),
                },
            },
            battlefield: self.battlefield.iter().map(permanent_snapshot).collect(),
            emblems: self.emblems.iter().map(emblem_snapshot).collect(),
            stack: self
                .stack
                .iter()
                .map(|object| {
                    let ability_payload = (object.kind != StackObjectKind::Spell)
                        .then(|| stack_ability_snapshot(self, object))
                        .flatten();
                    StackSnapshot {
                        object_id: object.id.0,
                        owner: object.card.owner.index(),
                        ability_payload,
                        requires_retired_object: stack_object_requires_retired(self, object),
                        has_runtime_overrides: !object.applied_effects.is_empty()
                            || !object.text_changes.is_empty()
                            || object.colors.is_some()
                            || object.cast_via_flashback
                            || object.is_copy,
                    }
                })
                .collect(),
            decision_state,
            has_deferred_state: !self.temporary_ability_grants.is_empty()
                || !self.delayed_triggers.is_empty()
                || !self.floating_triggers.is_empty()
                || has_unsupported_decision
                || !self.pending_events.is_empty()
                || !self.pending_triggers.is_empty()
                || self.players.iter().any(|player| {
                    player.mana.iter().any(|mana| {
                        mana.source.is_some()
                            || !mana.restrictions.is_empty()
                            || !mana.spend_effects.is_empty()
                    })
                })
                || self
                    .battlefield
                    .iter()
                    .any(|permanent| !permanent.damage_sources.is_empty()),
            // Makes accidental reuse with another seat fail closed in the
            // importer without revealing anything about that other seat.
            viewer: viewer.index(),
        }
    }

    /// Compatibility projection for protocol 20. The checkpoint has one
    /// typed schema internally; only this boundary turns it into JSON.
    pub(super) fn checkpoint_json(&self, viewer: PlayerId) -> Value {
        serde_json::to_value(self.snapshot(viewer)).expect("GameSnapshot is serializable")
    }

    /// Rebuilds a decision-boundary state from its seat checkpoint and
    /// separately supplied hidden-zone hypothesis.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn from_observation_checkpoint(
        catalog: CardCatalog,
        format: Format,
        observation: &Value,
        hidden: &Value,
        rollout_seed: u64,
    ) -> Result<Self, String> {
        let checkpoint: GameSnapshot =
            serde_json::from_value(field(observation, "checkpoint")?.clone())
                .map_err(|error| format!("invalid game snapshot: {error}"))?;
        if checkpoint.has_deferred_state {
            return Err("checkpoint contains a decision, deferred trigger, emblem, restricted mana, or other rules state not yet represented by semantic locators".into());
        }
        let viewer = seat_value(field(observation, "seat")?)?;
        if checkpoint.viewer != viewer.index() {
            return Err("checkpoint viewer does not match observation seat".into());
        }

        let mut next_object_id = max_public_object_id(observation)
            .unwrap_or(0)
            .saturating_add(1);
        let own_hand = parse_cards(field(observation, "hand")?, viewer, &catalog)?;
        let opponent = viewer.opponent();
        let opponent_hand_defs = hidden_definitions(hidden, "hands", opponent)?;
        if opponent_hand_defs.len() != usize_field(observation, "opponentHandSize")? {
            return Err("hidden opponent hand does not match opponentHandSize".into());
        }
        let opponent_hand =
            mint_cards(&opponent_hand_defs, opponent, &catalog, &mut next_object_id)?;
        let libraries = [PlayerId::One, PlayerId::Two].map(|player| {
            hidden_definitions(hidden, "libraries", player).and_then(|definitions| {
                let expected = array(field(observation, "librarySizes")?)?
                    .get(player.index())
                    .and_then(Value::as_u64)
                    .and_then(|value| usize::try_from(value).ok())
                    .ok_or_else(|| "librarySizes must contain two counts".to_owned())?;
                if definitions.len() != expected {
                    return Err(format!(
                        "hidden {} library has {} cards, expected {expected}",
                        seat_label(player),
                        definitions.len()
                    ));
                }
                mint_cards(&definitions, player, &catalog, &mut next_object_id)
            })
        });
        let [library_one, library_two] = libraries;
        let library_one = library_one?;
        let library_two = library_two?;

        let graveyards = parse_two_public_zones(field(observation, "graveyards")?, &catalog)?;
        let exiles = parse_two_public_zones(field(observation, "exiles")?, &catalog)?;
        let life = i16_pair(field(observation, "life")?)?;
        let checkpoint_hands = if viewer == PlayerId::One {
            [own_hand, opponent_hand]
        } else {
            [opponent_hand, own_hand]
        };
        let libraries = [library_one, library_two];
        let land_played = checkpoint.land_played_this_turn;
        let tried_empty = checkpoint.tried_to_draw_from_empty_library;
        let mana_values = array(field(observation, "manaPools")?)?;
        if mana_values.len() != 2 {
            return Err("manaPools must contain p1 and p2 values".into());
        }
        let mana_pools = [
            parse_mana_pool(&mana_values[0])?,
            parse_mana_pool(&mana_values[1])?,
        ];
        let players = [PlayerId::One, PlayerId::Two].map(|player| PlayerState {
            life: life[player.index()],
            library: libraries[player.index()].clone(),
            tried_to_draw_from_empty_library: tried_empty[player.index()],
            hand: checkpoint_hands[player.index()].clone(),
            graveyard: graveyards[player.index()].clone(),
            exile: exiles[player.index()].clone(),
            mana_pool: mana_pools[player.index()],
            mana: mana_from_pool(mana_pools[player.index()]),
            land_played_this_turn: land_played[player.index()],
        });

        let turns_started = checkpoint.turns_started;
        let mut game = Self {
            format,
            seed: rollout_seed,
            rng: ReplayRng::new(rollout_seed),
            catalog,
            physical_cards: Vec::new(),
            players,
            battlefield: Vec::new(),
            stack: GameStack::default(),
            retired_objects: BTreeMap::new(),
            temporary_ability_grants: Vec::new(),
            next_object_id,
            next_continuous_effect_timestamp: u64::from(next_object_id),
            turn: u32_field(observation, "turn")?,
            turns_started,
            active_player: seat_value(field(observation, "activeSeat")?)?,
            priority: seat_value(field(observation, "prioritySeat")?)?,
            consecutive_passes: checkpoint.consecutive_passes,
            step: parse_step(str_field(observation, "step")?)?,
            attackers_declared: checkpoint.attackers_declared,
            creature_died_this_turn: checkpoint.creature_died_this_turn,
            linked_exiles: checkpoint
                .linked_exiles
                .iter()
                .map(|pair| (GameObjectId(pair[0]), GameObjectId(pair[1])))
                .collect(),
            sorcery_flash_grants: checkpoint.sorcery_flash_grants,
            additional_combat_phases: checkpoint.additional_combat_phases,
            noncreature_casts_locked: checkpoint.noncreature_casts_locked,
            emblems: Vec::new(),
            spells_cast_this_turn: checkpoint.spells_cast_this_turn,
            spells_cast_last_turn: checkpoint.spells_cast_last_turn,
            cards_drawn_this_turn: checkpoint.cards_drawn_this_turn,
            drawn_this_turn: parse_drawn_this_turn(&checkpoint, hidden, viewer, &checkpoint_hands)?,
            miracle_window: parse_miracle_window(&checkpoint, hidden, viewer, &checkpoint_hands)?,
            delayed_triggers: Vec::new(),
            floating_triggers: Vec::new(),
            blockers_declared: checkpoint.blockers_declared,
            untap_pending: checkpoint.untap_pending,
            pregame: parse_pregame(checkpoint.pregame)?,
            mulligans: checkpoint.mulligans,
            cleanup_pending: checkpoint.cleanup_pending,
            pending_decisions: Vec::new(),
            next_decision_id: 0,
            pending_events: VecDeque::new(),
            pending_triggers: Vec::new(),
            next_trigger_id: 0,
            last_seen_hands: [None, None],
            pending_combat_attackers: ids(&checkpoint.pending_combat_attackers),
            combat_damage_stage: parse_combat_stage(&checkpoint.combat_damage_stage),
            combat_blocked_attackers: ids(&checkpoint.combat_blocked_attackers),
            extra_turns: checkpoint
                .extra_turns
                .iter()
                .copied()
                .map(player_from_index)
                .collect::<Result<Vec<_>, _>>()?,
            channel_active: checkpoint.channel_active,
            skipped_turns: checkpoint.skipped_turns,
            result: None,
            events: vec![GameEvent::GameStarted { seed: rollout_seed }],
        };
        game.battlefield = parse_battlefield(observation, &checkpoint.battlefield, &game.catalog)?;
        game.emblems = parse_emblems(observation, &checkpoint.emblems, &game)?;
        game.stack = parse_stack(observation, &checkpoint.stack, &game)?;
        game.pending_decisions =
            parse_pending_decision(observation, checkpoint.decision_state.as_ref())?
                .into_iter()
                .collect();
        game.next_decision_id = game
            .pending_decisions
            .first()
            .map_or(0, |decision| decision.observation.id.saturating_add(1));
        game.last_seen_hands[viewer.index()] =
            parse_last_seen_hand(observation.get("lastSeenHand"))?;
        game.next_continuous_effect_timestamp = game
            .battlefield
            .iter()
            .chain(&game.emblems)
            .map(|permanent| permanent.timestamp.0)
            .max()
            .unwrap_or(u64::from(game.next_object_id))
            .saturating_add(1);
        Ok(game)
    }
}

fn permanent_snapshot(permanent: &Permanent) -> PermanentSnapshot {
    PermanentSnapshot {
        object_id: permanent.card.id.0,
        owner: permanent.card.owner.index(),
        timestamp: permanent.timestamp.0,
        entered_controller_turn: permanent.entered_controller_turn,
        power_bonus: permanent.power_bonus,
        toughness_bonus: permanent.toughness_bonus,
        unblockable_this_turn: permanent.unblockable_this_turn,
        combat_damage_prevented: permanent.combat_damage_prevented,
        combat_damage_dealt_by_prevented: permanent.combat_damage_dealt_by_prevented,
        control_reverts_to: permanent.control_reverts_to.map(PlayerId::index),
        chosen_player: permanent.chosen_player.map(PlayerId::index),
        destroy_at_end: permanent.destroy_at_end,
        counters: permanent.counters.to_vec(),
        attached_to: permanent.attached_to.map(|id| id.0),
        exile_instead_of_dying: permanent.exile_instead_of_dying,
        combat_damage_assignment: permanent
            .combat_damage_assignment
            .iter()
            .map(|assignment| CombatDamageAssignmentSnapshot {
                recipient: format!("{:?}", assignment.recipient),
                amount: assignment.amount,
            })
            .collect(),
        regeneration_shields: permanent.regeneration_shields,
        attacked_this_turn: permanent.attacked_this_turn,
        attacks_this_turn: permanent.attacks_this_turn,
        damage_sources: permanent.damage_sources.iter().map(|id| id.0).collect(),
        dealt_damage_to_opponent_this_turn: permanent.dealt_damage_to_opponent_this_turn,
        deathtouch_damage: permanent.deathtouch_damage,
        created_by: permanent.created_by.map(|id| id.0),
        animation: permanent.animation.map(animation_snapshot),
        temporary_keywords: permanent
            .temporary_keywords
            .iter()
            .copied()
            .map(keyword_snapshot)
            .collect(),
        keywords_until_upkeep_of: permanent
            .keywords_until_upkeep_of
            .iter()
            .map(|(player, keyword)| UpkeepKeywordSnapshot {
                seat: player.index(),
                keyword: keyword_snapshot(*keyword),
            })
            .collect(),
        has_dynamic_characteristics: !permanent.temporary_granted_abilities.is_empty()
            || !permanent.temporary_removed_abilities.is_empty()
            || permanent.copy_effect.is_some()
            || !permanent.text_changes.is_empty(),
    }
}

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a Value, String> {
    value
        .get(name)
        .ok_or_else(|| format!("missing field {name}"))
}

fn array(value: &Value) -> Result<&Vec<Value>, String> {
    value.as_array().ok_or_else(|| "expected an array".into())
}

fn str_field<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    field(value, name)?
        .as_str()
        .ok_or_else(|| format!("field {name} must be a string"))
}

fn bool_field(value: &Value, name: &str) -> Result<bool, String> {
    field(value, name)?
        .as_bool()
        .ok_or_else(|| format!("field {name} must be boolean"))
}

fn usize_field(value: &Value, name: &str) -> Result<usize, String> {
    field(value, name)?
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| format!("field {name} must be an unsigned integer"))
}

fn u32_field(value: &Value, name: &str) -> Result<u32, String> {
    usize_field(value, name)
        .and_then(|v| u32::try_from(v).map_err(|_| format!("field {name} is too large")))
}
fn u8_field(value: &Value, name: &str) -> Result<u8, String> {
    usize_field(value, name)
        .and_then(|v| u8::try_from(v).map_err(|_| format!("field {name} is too large")))
}

fn seat_label(player: PlayerId) -> &'static str {
    if player == PlayerId::One { "p1" } else { "p2" }
}
fn seat_value(value: &Value) -> Result<PlayerId, String> {
    match value.as_str() {
        Some("p1") => Ok(PlayerId::One),
        Some("p2") => Ok(PlayerId::Two),
        _ => Err("seat must be p1 or p2".into()),
    }
}
fn definitions(value: &Value) -> Result<Vec<CardDefinitionId>, String> {
    array(value)?
        .iter()
        .map(|v| {
            v.as_u64()
                .and_then(|n| u16::try_from(n).ok())
                .map(CardDefinitionId)
                .ok_or_else(|| "card definitions must be u16 integers".into())
        })
        .collect()
}
fn hidden_definitions(
    hidden: &Value,
    zone: &str,
    player: PlayerId,
) -> Result<Vec<CardDefinitionId>, String> {
    definitions(field(field(hidden, zone)?, seat_label(player))?)
}

fn card(
    id: GameObjectId,
    definition: CardDefinitionId,
    owner: PlayerId,
    catalog: &CardCatalog,
) -> Result<CardInstance, String> {
    if catalog.get(definition).is_none() {
        return Err(format!("unknown card definition {}", definition.0));
    }
    Ok(CardInstance {
        id,
        definition,
        owner,
        backing: ObjectBacking::None,
        characteristics: CharacteristicSource::Card(definition),
    })
}

fn parse_cards(
    value: &Value,
    owner: PlayerId,
    catalog: &CardCatalog,
) -> Result<Vec<CardInstance>, String> {
    array(value)?
        .iter()
        .map(|value| {
            let id = GameObjectId(u32_field(value, "objectId")?);
            let definition = CardDefinitionId(
                u16::try_from(usize_field(value, "definition")?)
                    .map_err(|_| "definition is too large")?,
            );
            card(id, definition, owner, catalog)
        })
        .collect()
}

fn mint_cards(
    definitions: &[CardDefinitionId],
    owner: PlayerId,
    catalog: &CardCatalog,
    next: &mut u32,
) -> Result<Vec<CardInstance>, String> {
    definitions
        .iter()
        .map(|definition| {
            let id = GameObjectId(*next);
            *next = next
                .checked_add(1)
                .ok_or_else(|| "game object ids exhausted".to_owned())?;
            card(id, *definition, owner, catalog)
        })
        .collect()
}

fn parse_two_public_zones(
    value: &Value,
    catalog: &CardCatalog,
) -> Result<[Vec<CardInstance>; 2], String> {
    let zones = array(value)?;
    if zones.len() != 2 {
        return Err("public zone must contain p1 and p2 arrays".into());
    }
    Ok([
        parse_cards(&zones[0], PlayerId::One, catalog)?,
        parse_cards(&zones[1], PlayerId::Two, catalog)?,
    ])
}

fn max_public_object_id(observation: &Value) -> Option<u32> {
    [
        "hand",
        "graveyards",
        "exiles",
        "battlefield",
        "emblems",
        "stack",
    ]
    .into_iter()
    .filter_map(|name| observation.get(name))
    .flat_map(walk_object_ids)
    .max()
}
fn walk_object_ids(value: &Value) -> Box<dyn Iterator<Item = u32> + '_> {
    match value {
        Value::Array(values) => Box::new(values.iter().flat_map(walk_object_ids)),
        Value::Object(map) => Box::new(
            map.get("objectId")
                .and_then(Value::as_u64)
                .and_then(|id| u32::try_from(id).ok())
                .into_iter()
                .chain(map.values().flat_map(walk_object_ids)),
        ),
        _ => Box::new(std::iter::empty()),
    }
}

fn read_u8(v: &Value) -> Result<u8, String> {
    v.as_u64()
        .and_then(|n| u8::try_from(n).ok())
        .ok_or_else(|| "expected u8".into())
}
fn read_u16(v: &Value) -> Result<u16, String> {
    v.as_u64()
        .and_then(|n| u16::try_from(n).ok())
        .ok_or_else(|| "expected u16".into())
}
fn read_u32(v: &Value) -> Result<u32, String> {
    v.as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| "expected u32".into())
}
fn read_i16(v: &Value) -> Result<i16, String> {
    v.as_i64()
        .and_then(|n| i16::try_from(n).ok())
        .ok_or_else(|| "expected i16".into())
}
fn i16_pair(value: &Value) -> Result<[i16; 2], String> {
    let values = array(value)?;
    if values.len() != 2 {
        return Err("expected a two-element array".into());
    }
    Ok([read_i16(&values[0])?, read_i16(&values[1])?])
}

fn parse_mana_pool(value: &Value) -> Result<super::ManaPool, String> {
    Ok(super::ManaPool {
        white: u16::try_from(usize_field(value, "white")?).map_err(|_| "white mana too large")?,
        blue: u16::try_from(usize_field(value, "blue")?).map_err(|_| "blue mana too large")?,
        black: u16::try_from(usize_field(value, "black")?).map_err(|_| "black mana too large")?,
        red: u16::try_from(usize_field(value, "red")?).map_err(|_| "red mana too large")?,
        green: u16::try_from(usize_field(value, "green")?).map_err(|_| "green mana too large")?,
        colorless: u16::try_from(usize_field(value, "colorless")?)
            .map_err(|_| "colorless mana too large")?,
    })
}
fn mana_from_pool(pool: super::ManaPool) -> Vec<super::Mana> {
    use crate::ManaColor;
    [
        (ManaColor::White, pool.white),
        (ManaColor::Blue, pool.blue),
        (ManaColor::Black, pool.black),
        (ManaColor::Red, pool.red),
        (ManaColor::Green, pool.green),
        (ManaColor::Colorless, pool.colorless),
    ]
    .into_iter()
    .flat_map(|(color, count)| {
        std::iter::repeat_n(super::Mana::unrestricted(color), usize::from(count))
    })
    .collect()
}

fn ids(values: &[u32]) -> Vec<GameObjectId> {
    values.iter().copied().map(GameObjectId).collect()
}
fn parse_ids(value: &Value) -> Result<Vec<GameObjectId>, String> {
    array(value)?
        .iter()
        .map(|value| read_u32(value).map(GameObjectId))
        .collect()
}
fn parse_drawn_this_turn(
    checkpoint: &GameSnapshot,
    hidden: &Value,
    viewer: PlayerId,
    hands: &[Vec<CardInstance>; 2],
) -> Result<[Vec<GameObjectId>; 2], String> {
    let mut drawn = [Vec::new(), Vec::new()];
    drawn[viewer.index()] = ids(&checkpoint.drawn_this_turn[viewer.index()]);
    let opponent = viewer.opponent();
    if let Some(indices) = hidden
        .get("drawnThisTurn")
        .and_then(|value| value.get(seat_label(opponent)))
    {
        drawn[opponent.index()] = hidden_hand_indices(indices, &hands[opponent.index()])?;
    }
    Ok(drawn)
}

fn hidden_hand_indices(value: &Value, hand: &[CardInstance]) -> Result<Vec<GameObjectId>, String> {
    array(value)?
        .iter()
        .map(|value| {
            let index = value
                .as_u64()
                .and_then(|index| usize::try_from(index).ok())
                .ok_or_else(|| "hidden hand indices must be unsigned integers".to_owned())?;
            hand.get(index)
                .map(|card| card.id)
                .ok_or_else(|| format!("hidden hand index {index} is out of range"))
        })
        .collect()
}

fn parse_miracle_window(
    checkpoint: &GameSnapshot,
    hidden: &Value,
    viewer: PlayerId,
    hands: &[Vec<CardInstance>; 2],
) -> Result<Option<GameObjectId>, String> {
    if let Some(object) = checkpoint.miracle_window {
        return Ok(Some(GameObjectId(object)));
    }
    let Some(window) = hidden.get("miracleWindow").filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let player = seat_value(field(window, "seat")?)?;
    if player != viewer.opponent() {
        return Err("hidden miracleWindow must belong to the opposing seat".into());
    }
    let index = usize_field(window, "handIndex")?;
    hands[player.index()]
        .get(index)
        .map(|card| Some(card.id))
        .ok_or_else(|| format!("hidden miracle hand index {index} is out of range"))
}
fn optional_id(value: Option<&Value>) -> Option<GameObjectId> {
    value
        .and_then(Value::as_u64)
        .and_then(|id| u32::try_from(id).ok())
        .map(GameObjectId)
}
fn parse_last_seen_hand(value: Option<&Value>) -> Result<super::LastSeenHand, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let player = seat_value(field(value, "seat")?)?;
    let cards = array(field(value, "cards")?)?
        .iter()
        .map(|card| {
            Ok((
                GameObjectId(u32_field(card, "objectId")?),
                CardDefinitionId(
                    u16::try_from(usize_field(card, "definition")?)
                        .map_err(|_| "last-seen definition is too large")?,
                ),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Some((player, cards)))
}
fn parse_step(value: &str) -> Result<Step, String> {
    match value {
        "Upkeep" => Ok(Step::Upkeep),
        "Draw" => Ok(Step::Draw),
        "PrecombatMain" => Ok(Step::PrecombatMain),
        "BeginningOfCombat" => Ok(Step::BeginningOfCombat),
        "DeclareAttackers" => Ok(Step::DeclareAttackers),
        "DeclareBlockers" => Ok(Step::DeclareBlockers),
        "CombatDamage" => Ok(Step::CombatDamage),
        "EndOfCombat" => Ok(Step::EndOfCombat),
        "PostcombatMain" => Ok(Step::PostcombatMain),
        "End" => Ok(Step::End),
        "Cleanup" => Ok(Step::Cleanup),
        _ => Err(format!("unknown step {value}")),
    }
}
fn parse_pregame(value: Option<PregameSnapshot>) -> Result<Option<Pregame>, String> {
    value
        .map(|value| match value {
            PregameSnapshot::Mulligan { seat } => player_from_index(seat).map(Pregame::Mulligan),
            PregameSnapshot::Bottom { seat } => player_from_index(seat).map(Pregame::Bottom),
        })
        .transpose()
}
fn parse_combat_stage(value: &CombatDamageStageSnapshot) -> CombatDamageStage {
    match value {
        CombatDamageStageSnapshot::NotStarted => CombatDamageStage::NotStarted,
        CombatDamageStageSnapshot::Single => CombatDamageStage::Single,
        CombatDamageStageSnapshot::FirstStrike { combatants } => CombatDamageStage::FirstStrike {
            strike_wave_combatants: ids(combatants),
        },
        CombatDamageStageSnapshot::RegularAfterFirstStrike { combatants } => {
            CombatDamageStage::RegularAfterFirstStrike {
                strike_wave_combatants: ids(combatants),
            }
        }
    }
}

fn parse_battlefield(
    observation: &Value,
    snapshots: &[PermanentSnapshot],
    catalog: &CardCatalog,
) -> Result<Vec<Permanent>, String> {
    let visible = array(field(observation, "battlefield")?)?;
    if visible.len() != snapshots.len() {
        return Err("checkpoint battlefield does not match observation".into());
    }
    visible.iter().zip(snapshots).map(|(shown, state)| {
        if state.has_dynamic_characteristics { return Err("checkpoint permanent has dynamic characteristics not yet represented by semantic locators".into()); }
        if !state.combat_damage_assignment.is_empty() { return Err("checkpoint permanent has a combat damage assignment not yet represented structurally".into()); }
        let id = GameObjectId(u32_field(shown, "objectId")?);
        if id.0 != state.object_id { return Err("checkpoint permanent id does not match observation".into()); }
        let definition = CardDefinitionId(u16::try_from(usize_field(shown, "definition")?).map_err(|_| "definition too large")?);
        let owner = player_from_index(state.owner)?;
        let controller = seat_value(field(shown, "controller")?)?;
        if state.counters.len() != CounterKind::COUNT { return Err("counter vector has the wrong length".into()); }
        let mut counters = [0; CounterKind::COUNT]; counters.copy_from_slice(&state.counters);
        let mut permanent = Permanent::entering(card(id, definition, owner, catalog)?, CardPartId(u8::try_from(usize_field(shown, "presentedPartId")?).map_err(|_| "part id too large")?), controller, state.entered_controller_turn);
        permanent.timestamp = ContinuousEffectTimestamp(state.timestamp);
        permanent.tapped = bool_field(shown, "tapped")?;
        permanent.damage = u16::try_from(usize_field(shown, "damage")?).map_err(|_| "damage too large")?;
        permanent.power_bonus = state.power_bonus; permanent.toughness_bonus = state.toughness_bonus;
        permanent.attacking = bool_field(shown, "attacking")?; permanent.blocked = bool_field(shown, "blockedThisCombat")?; permanent.blocking = optional_id(shown.get("blocking"));
        permanent.attack_defender = shown
            .get("attackDefender")
            .filter(|value| !value.is_null())
            .map(parse_attack_defender)
            .transpose()?;
        permanent.activated_loyalty_this_turn = bool_field(shown, "loyaltyAbilityUsedThisTurn")?;
        permanent.unblockable_this_turn = state.unblockable_this_turn; permanent.combat_damage_prevented = state.combat_damage_prevented; permanent.combat_damage_dealt_by_prevented = state.combat_damage_dealt_by_prevented;
        permanent.control_reverts_to = state.control_reverts_to.map(player_from_index).transpose()?; permanent.chosen_player = state.chosen_player.map(player_from_index).transpose()?;
        permanent.chosen_creature_type = shown
            .get("chosenCreatureType")
            .and_then(Value::as_str)
            .map(str::to_owned);
        permanent.chosen_card_name = shown
            .get("chosenCardName")
            .and_then(Value::as_str)
            .map(str::to_owned);
        permanent.animation = state.animation
            .as_ref()
            .map(|value| {
                catalog_animation(catalog, value)
                    .ok_or_else(|| "checkpoint animation is absent from this catalog".to_owned())
            })
            .transpose()?;
        permanent.temporary_keywords = state.temporary_keywords.iter().copied()
            .map(parse_keyword)
            .collect();
        permanent.keywords_until_upkeep_of = state.keywords_until_upkeep_of.iter()
            .map(|entry| {
                Ok((player_from_index(entry.seat)?, parse_keyword(entry.keyword)))
            })
            .collect::<Result<Vec<_>, String>>()?;
        permanent.destroy_at_end = state.destroy_at_end; permanent.counters = counters; permanent.attached_to = state.attached_to.map(GameObjectId); permanent.exile_instead_of_dying = state.exile_instead_of_dying;
        permanent.regeneration_shields = state.regeneration_shields; permanent.attacked_this_turn = state.attacked_this_turn; permanent.attacks_this_turn = state.attacks_this_turn; permanent.damage_sources = ids(&state.damage_sources); permanent.dealt_damage_to_opponent_this_turn = state.dealt_damage_to_opponent_this_turn; permanent.deathtouch_damage = state.deathtouch_damage; permanent.created_by = state.created_by.map(GameObjectId);
        Ok(permanent)
    }).collect()
}

fn player_from_index(index: usize) -> Result<PlayerId, String> {
    match index {
        0 => Ok(PlayerId::One),
        1 => Ok(PlayerId::Two),
        _ => Err("seat index must be 0 or 1".into()),
    }
}

fn parse_attack_defender(value: &Value) -> Result<AttackDefender, String> {
    match str_field(value, "type")? {
        "player" => Ok(AttackDefender::Player(seat_value(field(value, "seat")?)?)),
        "planeswalker" => Ok(AttackDefender::Planeswalker(GameObjectId(u32_field(
            value, "objectId",
        )?))),
        other => Err(format!("unknown attack defender type {other}")),
    }
}

fn parse_cast_signature(value: &Value) -> Result<CastSignature, String> {
    let form_value = field(value, "form")?;
    let form = match str_field(form_value, "kind")? {
        "part" => SpellForm::Part(CardPartId(
            u8::try_from(usize_field(form_value, "partId")?).map_err(|_| "part id too large")?,
        )),
        "combined" => SpellForm::Combined(
            array(field(form_value, "partIds")?)?
                .iter()
                .map(|part| read_u8(part).map(CardPartId))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        other => return Err(format!("unknown spell form {other}")),
    };
    let alternative = value
        .get("alternativeCostId")
        .filter(|v| !v.is_null())
        .map(|v| read_u8(v).map(AlternativeCostId))
        .transpose()?;
    let additional = array(field(value, "additionalCostIds")?)?
        .iter()
        .map(|v| read_u8(v).map(AdditionalCostId))
        .collect::<Result<Vec<_>, _>>()?;
    let modes = array(field(value, "modeIds")?)?
        .iter()
        .map(|v| read_u8(v).map(ModeId))
        .collect::<Result<Vec<_>, _>>()?;
    let selections = array(field(value, "targetSelections")?)?
        .iter()
        .map(parse_target_selection)
        .collect::<Result<Vec<_>, _>>()?;
    let choices = CastChoices::new(PlayOptionId(
        u8::try_from(usize_field(value, "playOptionId")?).map_err(|_| "play option too large")?,
    ))
    .with_modes(modes)
    .with_costs(CostConfiguration::new(alternative, additional))
    .with_x(u16::try_from(usize_field(value, "x")?).map_err(|_| "x too large")?)
    .with_targets(selections);
    Ok(CastSignature::from_validated_choices(form, choices))
}

fn parse_target_selection(value: &Value) -> Result<TargetSelection, String> {
    let slot = TargetSlotId(
        u8::try_from(usize_field(value, "slotId")?).map_err(|_| "target slot too large")?,
    );
    let targets = array(field(value, "targets")?)?
        .iter()
        .map(parse_target)
        .collect::<Result<Vec<_>, _>>()?;
    let amounts = array(field(value, "amounts")?)?
        .iter()
        .map(read_u16)
        .collect::<Result<Vec<_>, _>>()?;
    if amounts.is_empty() {
        Ok(TargetSelection::new(slot, targets))
    } else if amounts.len() == targets.len() {
        Ok(TargetSelection::divided(slot, targets, amounts))
    } else {
        Err("divided target amounts do not match targets".into())
    }
}

fn parse_target(value: &Value) -> Result<Target, String> {
    match str_field(value, "type")? {
        "player" => Ok(Target::Player(seat_value(field(value, "seat")?)?)),
        "card" => Ok(Target::Card(PublicGameObjectId(u32_field(
            value, "objectId",
        )?))),
        "permanent" => Ok(Target::Permanent(PublicGameObjectId(u32_field(
            value, "objectId",
        )?))),
        "spell" => Ok(Target::Spell(PublicGameObjectId(u32_field(
            value, "objectId",
        )?))),
        other => Err(format!("unknown target type {other}")),
    }
}

#[cfg(test)]
mod tests;
