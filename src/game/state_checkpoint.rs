use std::collections::{BTreeMap, VecDeque};

use serde_json::{Value, json};

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
mod semantics;
mod stack;

use decision::{decision_checkpoint_json, parse_pending_decision};
use semantics::{
    ability_locator_json, animation_json, catalog_ability, catalog_animation, keyword_json,
    parse_keyword,
};
use stack::{parse_stack, stack_ability_checkpoint_json, stack_object_requires_retired};

impl Game {
    /// Hidden-safe rules bookkeeping needed to use an observation as a
    /// current-state checkpoint. Presentation fields stay in the ordinary
    /// observation; this object carries the state which cannot be inferred
    /// reliably from them.
    pub(super) fn checkpoint_json(&self, viewer: PlayerId) -> Value {
        let decision_state = (self.pending_decisions.len() == 1)
            .then(|| decision_checkpoint_json(&self.pending_decisions[0]))
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
        json!({
            "turnsStarted": self.turns_started,
            "consecutivePasses": self.consecutive_passes,
            "attackersDeclared": self.attackers_declared,
            "blockersDeclared": self.blockers_declared,
            "untapPending": self.untap_pending,
            "cleanupPending": self.cleanup_pending,
            "mulligans": self.mulligans,
            "landPlayedThisTurn": [
                self.players[0].land_played_this_turn,
                self.players[1].land_played_this_turn,
            ],
            "triedToDrawFromEmptyLibrary": [
                self.players[0].tried_to_draw_from_empty_library,
                self.players[1].tried_to_draw_from_empty_library,
            ],
            "creatureDiedThisTurn": self.creature_died_this_turn,
            "linkedExiles": self.linked_exiles.iter().map(|(source, card)| {
                json!([source.0, card.0])
            }).collect::<Vec<_>>(),
            "sorceryFlashGrants": self.sorcery_flash_grants,
            "additionalCombatPhases": self.additional_combat_phases,
            "noncreatureCastsLocked": self.noncreature_casts_locked,
            "spellsCastThisTurn": self.spells_cast_this_turn,
            "spellsCastLastTurn": self.spells_cast_last_turn,
            "cardsDrawnThisTurn": self.cards_drawn_this_turn,
            "drawnThisTurn": visible_drawn_this_turn,
            "miracleWindow": self.miracle_window.filter(|id| {
                self.players[viewer.index()].hand.iter().any(|card| card.id == *id)
            }).map(|id| id.0),
            "pendingCombatAttackers": self.pending_combat_attackers.iter().map(|id| id.0).collect::<Vec<_>>(),
            "combatBlockedAttackers": self.combat_blocked_attackers.iter().map(|id| id.0).collect::<Vec<_>>(),
            "extraTurns": self.extra_turns.iter().map(|player| player.index()).collect::<Vec<_>>(),
            "channelActive": self.channel_active,
            "skippedTurns": self.skipped_turns,
            "pregame": self.pregame.map(|pregame| match pregame {
                Pregame::Mulligan(player) => json!({"kind": "mulligan", "seat": player.index()}),
                Pregame::Bottom(player) => json!({"kind": "bottom", "seat": player.index()}),
            }),
            "combatDamageStage": match &self.combat_damage_stage {
                CombatDamageStage::NotStarted => json!({"kind": "notStarted"}),
                CombatDamageStage::Single => json!({"kind": "single"}),
                CombatDamageStage::FirstStrike { strike_wave_combatants } => json!({
                    "kind": "firstStrike",
                    "combatants": strike_wave_combatants.iter().map(|id| id.0).collect::<Vec<_>>(),
                }),
                CombatDamageStage::RegularAfterFirstStrike { strike_wave_combatants } => json!({
                    "kind": "regularAfterFirstStrike",
                    "combatants": strike_wave_combatants.iter().map(|id| id.0).collect::<Vec<_>>(),
                }),
            },
            "battlefield": self.battlefield.iter().map(permanent_checkpoint_json).collect::<Vec<_>>(),
            "stack": self.stack.iter().map(|object| {
                let ability_payload = (object.kind != StackObjectKind::Spell)
                    .then(|| stack_ability_checkpoint_json(self, object));
                json!({
                    "objectId": object.id.0,
                    "owner": object.card.owner.index(),
                    "abilityPayload": ability_payload,
                    "requiresRetiredObject": stack_object_requires_retired(self, object),
                    "hasRuntimeOverrides": !object.applied_effects.is_empty()
                        || !object.text_changes.is_empty()
                        || object.colors.is_some()
                        || object.cast_via_flashback
                        || object.is_copy,
                })
            }).collect::<Vec<_>>(),
            "decisionState": decision_state,
            "hasDeferredState": !self.emblems.is_empty()
                || !self.temporary_ability_grants.is_empty()
                || !self.delayed_triggers.is_empty()
                || !self.floating_triggers.is_empty()
                || has_unsupported_decision
                || !self.pending_events.is_empty()
                || !self.pending_triggers.is_empty()
                || self.players.iter().any(|player| player.mana.iter().any(|mana| {
                    mana.source.is_some()
                        || !mana.restrictions.is_empty()
                        || !mana.spend_effects.is_empty()
                }))
                || self.battlefield.iter().any(|permanent| !permanent.damage_sources.is_empty()),
            // Makes accidental reuse with another seat fail closed in the
            // importer without revealing anything about that other seat.
            "viewer": viewer.index(),
        })
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
        let checkpoint = field(observation, "checkpoint")?;
        if bool_field(checkpoint, "hasDeferredState")? {
            return Err("checkpoint contains a decision, deferred trigger, emblem, restricted mana, or other rules state not yet represented by semantic locators".into());
        }
        let viewer = seat_value(field(observation, "seat")?)?;
        if usize_field(checkpoint, "viewer")? != viewer.index() {
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
        let land_played = bool_pair(field(checkpoint, "landPlayedThisTurn")?)?;
        let tried_empty = bool_pair(field(checkpoint, "triedToDrawFromEmptyLibrary")?)?;
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

        let turns_started = u32_pair(field(checkpoint, "turnsStarted")?)?;
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
            consecutive_passes: u8_field(checkpoint, "consecutivePasses")?,
            step: parse_step(str_field(observation, "step")?)?,
            attackers_declared: bool_field(checkpoint, "attackersDeclared")?,
            creature_died_this_turn: bool_field(checkpoint, "creatureDiedThisTurn")?,
            linked_exiles: parse_id_pairs(field(checkpoint, "linkedExiles")?)?,
            sorcery_flash_grants: u8_pair(field(checkpoint, "sorceryFlashGrants")?)?,
            additional_combat_phases: u8_field(checkpoint, "additionalCombatPhases")?,
            noncreature_casts_locked: bool_pair(field(checkpoint, "noncreatureCastsLocked")?)?,
            emblems: Vec::new(),
            spells_cast_this_turn: u16_pair(field(checkpoint, "spellsCastThisTurn")?)?,
            spells_cast_last_turn: u16_pair(field(checkpoint, "spellsCastLastTurn")?)?,
            cards_drawn_this_turn: u16_pair(field(checkpoint, "cardsDrawnThisTurn")?)?,
            drawn_this_turn: parse_drawn_this_turn(checkpoint, hidden, viewer, &checkpoint_hands)?,
            miracle_window: parse_miracle_window(checkpoint, hidden, viewer, &checkpoint_hands)?,
            delayed_triggers: Vec::new(),
            floating_triggers: Vec::new(),
            blockers_declared: bool_field(checkpoint, "blockersDeclared")?,
            untap_pending: bool_field(checkpoint, "untapPending")?,
            pregame: parse_pregame(checkpoint.get("pregame"))?,
            mulligans: u8_pair(field(checkpoint, "mulligans")?)?,
            cleanup_pending: bool_field(checkpoint, "cleanupPending")?,
            pending_decisions: Vec::new(),
            next_decision_id: 0,
            pending_events: VecDeque::new(),
            pending_triggers: Vec::new(),
            next_trigger_id: 0,
            last_seen_hands: [None, None],
            pending_combat_attackers: parse_ids(field(checkpoint, "pendingCombatAttackers")?)?,
            combat_damage_stage: parse_combat_stage(field(checkpoint, "combatDamageStage")?)?,
            combat_blocked_attackers: parse_ids(field(checkpoint, "combatBlockedAttackers")?)?,
            extra_turns: parse_seat_indices(field(checkpoint, "extraTurns")?)?,
            channel_active: bool_pair(field(checkpoint, "channelActive")?)?,
            skipped_turns: u16_pair(field(checkpoint, "skippedTurns")?)?,
            result: None,
            events: vec![GameEvent::GameStarted { seed: rollout_seed }],
        };
        game.battlefield = parse_battlefield(observation, checkpoint, &game.catalog)?;
        game.stack = parse_stack(observation, checkpoint, &game)?;
        game.pending_decisions = parse_pending_decision(observation, checkpoint)?
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
            .map(|permanent| permanent.timestamp.0)
            .max()
            .unwrap_or(u64::from(game.next_object_id))
            .saturating_add(1);
        Ok(game)
    }
}

fn permanent_checkpoint_json(permanent: &Permanent) -> Value {
    json!({
        "objectId": permanent.card.id.0,
        "owner": permanent.card.owner.index(),
        "timestamp": permanent.timestamp.0,
        "enteredControllerTurn": permanent.entered_controller_turn,
        "powerBonus": permanent.power_bonus,
        "toughnessBonus": permanent.toughness_bonus,
        "unblockableThisTurn": permanent.unblockable_this_turn,
        "combatDamagePrevented": permanent.combat_damage_prevented,
        "combatDamageDealtByPrevented": permanent.combat_damage_dealt_by_prevented,
        "controlRevertsTo": permanent.control_reverts_to.map(PlayerId::index),
        "chosenPlayer": permanent.chosen_player.map(PlayerId::index),
        "destroyAtEnd": permanent.destroy_at_end,
        "counters": permanent.counters,
        "attachedTo": permanent.attached_to.map(|id| id.0),
        "exileInsteadOfDying": permanent.exile_instead_of_dying,
        "combatDamageAssignment": permanent.combat_damage_assignment.iter().map(|assignment| {
            json!({"recipient": format!("{:?}", assignment.recipient), "amount": assignment.amount})
        }).collect::<Vec<_>>(),
        "regenerationShields": permanent.regeneration_shields,
        "attackedThisTurn": permanent.attacked_this_turn,
        "attacksThisTurn": permanent.attacks_this_turn,
        "damageSources": permanent.damage_sources.iter().map(|id| id.0).collect::<Vec<_>>(),
        "dealtDamageToOpponentThisTurn": permanent.dealt_damage_to_opponent_this_turn,
        "deathtouchDamage": permanent.deathtouch_damage,
        "createdBy": permanent.created_by.map(|id| id.0),
        "animation": permanent.animation.map(animation_json),
        "temporaryKeywords": permanent.temporary_keywords.iter().copied().map(keyword_json).collect::<Vec<_>>(),
        "keywordsUntilUpkeepOf": permanent.keywords_until_upkeep_of.iter().map(|(player, keyword)| json!({
            "seat": player.index(),
            "keyword": keyword_json(*keyword),
        })).collect::<Vec<_>>(),
        "hasDynamicCharacteristics": !permanent.temporary_granted_abilities.is_empty()
            || !permanent.temporary_removed_abilities.is_empty()
            || permanent.copy_effect.is_some()
            || !permanent.text_changes.is_empty(),
    })
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
fn seat_index(value: &Value) -> Result<PlayerId, String> {
    match value.as_u64() {
        Some(0) => Ok(PlayerId::One),
        Some(1) => Ok(PlayerId::Two),
        _ => Err("seat index must be 0 or 1".into()),
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
    ["hand", "graveyards", "exiles", "battlefield", "stack"]
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

macro_rules! pair {
    ($name:ident, $ty:ty, $read:ident) => {
        fn $name(value: &Value) -> Result<[$ty; 2], String> {
            let values = array(value)?;
            if values.len() != 2 {
                return Err("expected a two-element array".into());
            }
            Ok([$read(&values[0])?, $read(&values[1])?])
        }
    };
}
fn read_bool(v: &Value) -> Result<bool, String> {
    v.as_bool().ok_or_else(|| "expected boolean".into())
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
pair!(bool_pair, bool, read_bool);
pair!(u8_pair, u8, read_u8);
pair!(u16_pair, u16, read_u16);
pair!(u32_pair, u32, read_u32);
pair!(i16_pair, i16, read_i16);

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

fn parse_ids(value: &Value) -> Result<Vec<GameObjectId>, String> {
    array(value)?
        .iter()
        .map(|v| read_u32(v).map(GameObjectId))
        .collect()
}
fn parse_drawn_this_turn(
    checkpoint: &Value,
    hidden: &Value,
    viewer: PlayerId,
    hands: &[Vec<CardInstance>; 2],
) -> Result<[Vec<GameObjectId>; 2], String> {
    let visible = array(field(checkpoint, "drawnThisTurn")?)?;
    if visible.len() != 2 {
        return Err("drawnThisTurn must contain p1 and p2 arrays".into());
    }
    let mut drawn = [Vec::new(), Vec::new()];
    drawn[viewer.index()] = parse_ids(&visible[viewer.index()])?;
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
    checkpoint: &Value,
    hidden: &Value,
    viewer: PlayerId,
    hands: &[Vec<CardInstance>; 2],
) -> Result<Option<GameObjectId>, String> {
    if let Some(object) = optional_id(checkpoint.get("miracleWindow")) {
        return Ok(Some(object));
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
fn parse_id_pairs(value: &Value) -> Result<Vec<(GameObjectId, GameObjectId)>, String> {
    array(value)?
        .iter()
        .map(|pair| {
            let pair = array(pair)?;
            if pair.len() != 2 {
                return Err("linked exile pair must have two ids".into());
            }
            Ok((
                GameObjectId(read_u32(&pair[0])?),
                GameObjectId(read_u32(&pair[1])?),
            ))
        })
        .collect()
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
fn parse_seat_indices(value: &Value) -> Result<Vec<PlayerId>, String> {
    array(value)?.iter().map(seat_index).collect()
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
fn parse_pregame(value: Option<&Value>) -> Result<Option<Pregame>, String> {
    let Some(value) = value.filter(|v| !v.is_null()) else {
        return Ok(None);
    };
    let player = seat_index(field(value, "seat")?)?;
    match str_field(value, "kind")? {
        "mulligan" => Ok(Some(Pregame::Mulligan(player))),
        "bottom" => Ok(Some(Pregame::Bottom(player))),
        other => Err(format!("unknown pregame kind {other}")),
    }
}
fn parse_combat_stage(value: &Value) -> Result<CombatDamageStage, String> {
    let combatants = || field(value, "combatants").and_then(parse_ids);
    match str_field(value, "kind")? {
        "notStarted" => Ok(CombatDamageStage::NotStarted),
        "single" => Ok(CombatDamageStage::Single),
        "firstStrike" => Ok(CombatDamageStage::FirstStrike {
            strike_wave_combatants: combatants()?,
        }),
        "regularAfterFirstStrike" => Ok(CombatDamageStage::RegularAfterFirstStrike {
            strike_wave_combatants: combatants()?,
        }),
        other => Err(format!("unknown combat stage {other}")),
    }
}

fn parse_battlefield(
    observation: &Value,
    checkpoint: &Value,
    catalog: &CardCatalog,
) -> Result<Vec<Permanent>, String> {
    let visible = array(field(observation, "battlefield")?)?;
    let raw = array(field(checkpoint, "battlefield")?)?;
    if visible.len() != raw.len() {
        return Err("checkpoint battlefield does not match observation".into());
    }
    visible.iter().zip(raw).map(|(shown, state)| {
        if bool_field(state, "hasDynamicCharacteristics")? { return Err("checkpoint permanent has dynamic characteristics not yet represented by semantic locators".into()); }
        if !array(field(state, "combatDamageAssignment")?)?.is_empty() { return Err("checkpoint permanent has a combat damage assignment not yet represented structurally".into()); }
        let id = GameObjectId(u32_field(shown, "objectId")?);
        if id.0 != u32_field(state, "objectId")? { return Err("checkpoint permanent id does not match observation".into()); }
        let definition = CardDefinitionId(u16::try_from(usize_field(shown, "definition")?).map_err(|_| "definition too large")?);
        let owner = seat_index(field(state, "owner")?)?;
        let controller = seat_value(field(shown, "controller")?)?;
        let counters_values = array(field(state, "counters")?)?;
        if counters_values.len() != CounterKind::COUNT { return Err("counter vector has the wrong length".into()); }
        let mut counters = [0; CounterKind::COUNT]; for (slot, value) in counters.iter_mut().zip(counters_values) { *slot = read_u16(value)?; }
        let mut permanent = Permanent::entering(card(id, definition, owner, catalog)?, CardPartId(u8::try_from(usize_field(shown, "presentedPartId")?).map_err(|_| "part id too large")?), controller, u32_field(state, "enteredControllerTurn")?);
        permanent.timestamp = ContinuousEffectTimestamp(field(state, "timestamp")?.as_u64().ok_or("timestamp must be u64")?);
        permanent.tapped = bool_field(shown, "tapped")?;
        permanent.damage = u16::try_from(usize_field(shown, "damage")?).map_err(|_| "damage too large")?;
        permanent.power_bonus = read_i16(field(state, "powerBonus")?)?; permanent.toughness_bonus = read_i16(field(state, "toughnessBonus")?)?;
        permanent.attacking = bool_field(shown, "attacking")?; permanent.blocked = bool_field(shown, "blockedThisCombat")?; permanent.blocking = optional_id(shown.get("blocking"));
        permanent.attack_defender = shown
            .get("attackDefender")
            .filter(|value| !value.is_null())
            .map(parse_attack_defender)
            .transpose()?;
        permanent.activated_loyalty_this_turn = bool_field(shown, "loyaltyAbilityUsedThisTurn")?;
        permanent.unblockable_this_turn = bool_field(state, "unblockableThisTurn")?; permanent.combat_damage_prevented = bool_field(state, "combatDamagePrevented")?; permanent.combat_damage_dealt_by_prevented = bool_field(state, "combatDamageDealtByPrevented")?;
        permanent.control_reverts_to = state.get("controlRevertsTo").filter(|v| !v.is_null()).map(seat_index).transpose()?; permanent.chosen_player = state.get("chosenPlayer").filter(|v| !v.is_null()).map(seat_index).transpose()?;
        permanent.chosen_creature_type = shown
            .get("chosenCreatureType")
            .and_then(Value::as_str)
            .map(str::to_owned);
        permanent.chosen_card_name = shown
            .get("chosenCardName")
            .and_then(Value::as_str)
            .map(str::to_owned);
        permanent.animation = state
            .get("animation")
            .filter(|value| !value.is_null())
            .map(|value| {
                catalog_animation(catalog, value)
                    .ok_or_else(|| "checkpoint animation is absent from this catalog".to_owned())
            })
            .transpose()?;
        permanent.temporary_keywords = array(field(state, "temporaryKeywords")?)?
            .iter()
            .map(parse_keyword)
            .collect::<Result<Vec<_>, _>>()?;
        permanent.keywords_until_upkeep_of = array(field(state, "keywordsUntilUpkeepOf")?)?
            .iter()
            .map(|entry| {
                Ok((
                    seat_index(field(entry, "seat")?)?,
                    parse_keyword(field(entry, "keyword")?)?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        permanent.destroy_at_end = bool_field(state, "destroyAtEnd")?; permanent.counters = counters; permanent.attached_to = optional_id(state.get("attachedTo")); permanent.exile_instead_of_dying = bool_field(state, "exileInsteadOfDying")?;
        permanent.regeneration_shields = u8_field(state, "regenerationShields")?; permanent.attacked_this_turn = bool_field(state, "attackedThisTurn")?; permanent.attacks_this_turn = u8_field(state, "attacksThisTurn")?; permanent.damage_sources = parse_ids(field(state, "damageSources")?)?; permanent.dealt_damage_to_opponent_this_turn = bool_field(state, "dealtDamageToOpponentThisTurn")?; permanent.deathtouch_damage = bool_field(state, "deathtouchDamage")?; permanent.created_by = optional_id(state.get("createdBy"));
        Ok(permanent)
    }).collect()
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
