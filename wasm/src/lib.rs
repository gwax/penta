use osarena::poc;
use osarena::{
    Action, CardCatalog, CardDefinitionId, CardInstanceId, Game, GameEvent, GameResult,
    HandcraftedPolicy, PlayerId, PlayerObservation, Policy, RandomPolicy, Step, Target,
};
use serde_json::{Value, json};
use std::fmt::Write as _;
use wasm_bindgen::prelude::*;

const BOT_ACTION_LIMIT: usize = 50_000;

enum BotPolicy {
    Random(RandomPolicy),
    Handcrafted(HandcraftedPolicy),
}

impl BotPolicy {
    fn choose_action(&mut self, observation: &PlayerObservation) -> Option<Action> {
        match self {
            Self::Random(policy) => policy.choose_action(observation),
            Self::Handcrafted(policy) => policy.choose_action(observation),
        }
    }
}

/// Browser-owned game facade. JavaScript only selects legal action indexes;
/// rules and bot decisions remain inside the Rust engine.
#[wasm_bindgen]
pub struct WebGame {
    game: Game,
    catalog: CardCatalog,
    human: PlayerId,
    bot: BotPolicy,
    opponent_actions: Vec<Value>,
    pending_opponent_mana: Vec<String>,
    mana_undo_history: Vec<Game>,
    phase_stops: Vec<String>,
    autopass_enabled: bool,
}

#[wasm_bindgen]
impl WebGame {
    /// Creates a mirror-format game and advances until the human must decide.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error when a deck or policy name is unknown, game
    /// construction fails, or the bot cannot reach a human decision.
    #[wasm_bindgen(constructor)]
    pub fn new(
        human_deck: &str,
        bot_deck: &str,
        bot_policy: &str,
        human_first: bool,
        seed: u32,
    ) -> Result<WebGame, JsValue> {
        let catalog = poc::catalog().map_err(js_error)?;
        let human_deck = deck_by_name(human_deck)?;
        let bot_deck = deck_by_name(bot_deck)?;
        let human = if human_first {
            PlayerId::One
        } else {
            PlayerId::Two
        };
        let decks = match human {
            PlayerId::One => [human_deck, bot_deck],
            PlayerId::Two => [bot_deck, human_deck],
        };
        let game = Game::new(catalog.clone(), decks, u64::from(seed)).map_err(js_error)?;
        let bot = match bot_policy.to_ascii_lowercase().as_str() {
            "random" => BotPolicy::Random(RandomPolicy::new(u64::from(seed) ^ 0x00b0_7b07)),
            "handcrafted" => BotPolicy::Handcrafted(HandcraftedPolicy::new(catalog.clone())),
            _ => return Err(JsValue::from_str("unknown bot policy")),
        };
        let mut web_game = Self {
            game,
            catalog,
            human,
            bot,
            opponent_actions: Vec::new(),
            pending_opponent_mana: Vec::new(),
            mana_undo_history: Vec::new(),
            phase_stops: Vec::new(),
            autopass_enabled: true,
        };
        web_game.advance_until_human_choice()?;
        Ok(web_game)
    }

    /// Applies one action from the current state's action list.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error when the game is not waiting for the human,
    /// the index is stale, the action is rejected, or the bot cannot finish.
    pub fn act(&mut self, action_index: usize) -> Result<(), JsValue> {
        if self.game.decision_player() != Some(self.human) {
            return Err(JsValue::from_str("the game is not waiting for the human"));
        }
        let observation = self.game.observe(self.human);
        let action = observation
            .legal_actions
            .get(action_index)
            .cloned()
            .ok_or_else(|| JsValue::from_str("unknown legal action"))?;
        let mana_checkpoint = matches!(action, Action::ActivateManaAbility { .. })
            .then(|| self.game.clone());
        if mana_checkpoint.is_none() {
            self.mana_undo_history.clear();
        }
        self.opponent_actions.clear();
        self.pending_opponent_mana.clear();
        self.game.apply(self.human, action).map_err(js_error)?;
        self.advance_until_human_choice()?;
        if let Some(checkpoint) = mana_checkpoint {
            let before = checkpoint.observe(self.human);
            let after = self.game.observe(self.human);
            if self.game.decision_player() == Some(self.human)
                && before.turn == after.turn
                && before.step == after.step
                && before.active_player == after.active_player
                && before.stack == after.stack
            {
                self.mana_undo_history.push(checkpoint);
            } else {
                self.mana_undo_history.clear();
            }
        }
        Ok(())
    }

    /// Declares every currently legal attacker, then finishes attacker declaration.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error unless the human is declaring attackers or
    /// the engine rejects one of the otherwise-legal actions.
    pub fn attack_all(&mut self) -> Result<(), JsValue> {
        if self.game.decision_player() != Some(self.human)
            || self.game.observe(self.human).step != Step::DeclareAttackers
        {
            return Err(JsValue::from_str("the human is not declaring attackers"));
        }
        self.mana_undo_history.clear();
        self.opponent_actions.clear();
        self.pending_opponent_mana.clear();
        loop {
            let action = self
                .game
                .observe(self.human)
                .legal_actions
                .into_iter()
                .find(|action| matches!(action, Action::DeclareAttacker { .. }));
            let Some(action) = action else {
                break;
            };
            self.game.apply(self.human, action).map_err(js_error)?;
        }
        if let Some(finish) = self
            .game
            .observe(self.human)
            .legal_actions
            .into_iter()
            .find(|action| matches!(action, Action::FinishDeclaringAttackers))
        {
            self.game.apply(self.human, finish).map_err(js_error)?;
        }
        self.advance_until_human_choice()
    }

    /// Commits a complete set of blocker assignments selected by the browser UI.
    ///
    /// Assignments are encoded as JSON pairs of `[blocker_id, attacker_id]` so
    /// the UI can rearrange arrows freely before making one atomic submission.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error unless the human is declaring blockers or an
    /// assignment is duplicated, malformed, or no longer legal.
    pub fn finalize_blocks(&mut self, assignments_json: &str) -> Result<(), JsValue> {
        if self.game.decision_player() != Some(self.human)
            || self.game.observe(self.human).step != Step::DeclareBlockers
        {
            return Err(JsValue::from_str("the human is not declaring blockers"));
        }
        let assignments: Vec<[u32; 2]> =
            serde_json::from_str(assignments_json).map_err(js_error)?;
        let mut used_blockers = Vec::with_capacity(assignments.len());
        let legal_actions = self.game.observe(self.human).legal_actions;
        let mut block_actions = Vec::with_capacity(assignments.len());
        for [blocker, attacker] in assignments {
            let blocker = CardInstanceId(blocker);
            if used_blockers.contains(&blocker) {
                return Err(JsValue::from_str("a blocker can only block one attacker"));
            }
            used_blockers.push(blocker);
            let action = Action::DeclareBlocker {
                blocker,
                attacker: CardInstanceId(attacker),
            };
            if !legal_actions.contains(&action) {
                return Err(JsValue::from_str("a blocker assignment is no longer legal"));
            }
            block_actions.push(action);
        }
        self.mana_undo_history.clear();
        self.opponent_actions.clear();
        self.pending_opponent_mana.clear();
        for action in block_actions {
            self.game
                .apply(self.human, action)
                .map_err(js_error)?;
        }
        self.game
            .apply(self.human, Action::FinishDeclaringBlockers)
            .map_err(js_error)?;
        self.advance_until_human_choice()
    }

    /// Rewinds the most recent manual mana ability while it is still safe to do so.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error when there is no reversible mana activation.
    pub fn undo_mana(&mut self) -> Result<(), JsValue> {
        let previous = self
            .mana_undo_history
            .pop()
            .ok_or_else(|| JsValue::from_str("there is no mana ability to undo"))?;
        self.game = previous;
        self.opponent_actions.clear();
        self.pending_opponent_mana.clear();
        Ok(())
    }

    /// Enables or disables a human-interface stop for one displayed phase.
    /// The rules engine still exposes every individual step.
    pub fn set_phase_stop(&mut self, phase: &str, enabled: bool) -> Result<(), JsValue> {
        if !matches!(phase, "Beginning" | "Main 1" | "Combat" | "Main 2" | "Ending") {
            return Err(JsValue::from_str("unknown displayed phase"));
        }
        self.phase_stops.retain(|candidate| candidate != phase);
        if enabled {
            self.phase_stops.push(phase.into());
        }
        self.opponent_actions.clear();
        self.pending_opponent_mana.clear();
        Ok(())
    }

    /// Enables or disables the browser's smooth automatic priority yields.
    pub fn set_autopass(&mut self, enabled: bool) -> Result<(), JsValue> {
        self.autopass_enabled = enabled;
        self.opponent_actions.clear();
        self.pending_opponent_mana.clear();
        if enabled {
            self.advance_until_human_choice()?;
        }
        Ok(())
    }

    /// Returns the human-visible game state as JSON.
    #[must_use]
    pub fn state_json(&self) -> String {
        self.snapshot().to_string()
    }

    fn advance_until_human_choice(&mut self) -> Result<(), JsValue> {
        let mut pending_animation = None;
        for _ in 0..BOT_ACTION_LIMIT {
            let Some(player) = self.game.decision_player() else {
                self.finish_opponent_animation(&mut pending_animation);
                return Ok(());
            };
            let observation = self.game.observe(player);
            let action = if player == self.human {
                let Some(action) = automatic_human_action_with_blockers(
                    observation.step,
                    observation.active_player == self.human,
                    observation.stack.is_empty(),
                    observation
                        .battlefield
                        .iter()
                        .any(|permanent| permanent.attacking),
                    observation
                        .battlefield
                        .iter()
                        .any(|permanent| permanent.blocking.is_some()),
                    self.should_stop(observation.step),
                    self.autopass_enabled,
                    !observation.stack.is_empty()
                        && observation
                            .stack
                            .iter()
                            .all(|object| object.controller == self.human),
                    observation.mana_pools[self.human.index()].total() > 0,
                    &observation.legal_actions,
                ) else {
                    self.finish_opponent_animation(&mut pending_animation);
                    return Ok(());
                };
                action
            } else {
                self.bot
                    .choose_action(&observation)
                    .ok_or_else(|| JsValue::from_str("bot returned no action"))?
            };
            if player != self.human {
                if let Action::ActivateManaAbility { source, .. } = &action {
                    self.finish_opponent_animation(&mut pending_animation);
                    self.pending_opponent_mana
                        .push(self.instance_name(&observation, *source));
                } else if should_animate_action(&action) {
                    self.finish_opponent_animation(&mut pending_animation);
                    let mana_sources = if matches!(
                        action,
                        Action::CastSpell { .. } | Action::ActivateAbility { .. }
                    ) {
                        std::mem::take(&mut self.pending_opponent_mana)
                    } else {
                        Vec::new()
                    };
                    let label = self.opponent_action_label(&observation, &action);
                    let kind = animated_action_kind(&action);
                    let card_id = action_card(&action);
                    let card = card_id.map(|id| self.instance_name(&observation, id));
                    pending_animation = Some(json!({
                        "label": label,
                        "kind": kind,
                        "card": card,
                        "cardId": card_id.map(|id| id.0),
                        "manaSources": mana_sources,
                    }));
                } else {
                    self.pending_opponent_mana.clear();
                }
            }
            let event_start = self.game.events().len();
            self.game.apply(player, action).map_err(js_error)?;
            if player != self.human
                && let Some(animation) = pending_animation.as_mut()
            {
                let mana_sources = self.game.events()[event_start..]
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::ManaAdded {
                            player: producer,
                            source,
                        } if *producer == player => Some(*source),
                        _ => None,
                    })
                    .map(|source| json!(self.instance_name(&observation, source)))
                    .collect::<Vec<_>>();
                if let Some(existing) = animation["manaSources"].as_array_mut() {
                    existing.extend(mana_sources);
                }
            }
        }
        Err(JsValue::from_str(
            "game exceeded its automatic action limit",
        ))
    }

    fn finish_opponent_animation(&mut self, pending: &mut Option<Value>) {
        if let Some(mut animation) = pending.take() {
            animation["state"] = self.snapshot_value(false);
            self.opponent_actions.push(animation);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn snapshot(&self) -> Value {
        self.snapshot_value(true)
    }

    fn should_stop(&self, step: Step) -> bool {
        let phase = match step {
            Step::Upkeep | Step::Draw => "Beginning",
            Step::PrecombatMain => "Main 1",
            Step::BeginningOfCombat
            | Step::DeclareAttackers
            | Step::DeclareBlockers
            | Step::CombatDamage
            | Step::EndOfCombat => "Combat",
            Step::PostcombatMain => "Main 2",
            Step::End | Step::Cleanup => "Ending",
        };
        self.phase_stops.iter().any(|candidate| candidate == phase)
    }

    fn automatic_mana_sources(&self, action: &Action) -> Vec<u32> {
        if !matches!(action, Action::CastSpell { .. } | Action::ActivateAbility { .. }) {
            return Vec::new();
        }
        let mut preview = self.game.clone();
        let event_start = preview.events().len();
        if preview.apply(self.human, action.clone()).is_err() {
            return Vec::new();
        }
        preview.events()[event_start..]
            .iter()
            .filter_map(|event| match event {
                GameEvent::ManaAdded { player, source } if *player == self.human => Some(source.0),
                _ => None,
            })
            .collect()
    }

    #[allow(clippy::too_many_lines)]
    fn snapshot_value(&self, include_opponent_actions: bool) -> Value {
        let observation = self.game.observe(self.human);
        let opponent = self.human.opponent();
        let actions = observation
            .legal_actions
            .iter()
            .enumerate()
            .map(|(index, action)| {
                json!({
                    "index": index,
                    "label": self.action_label(&observation, action),
                    "kind": action_kind(action),
                    "cardId": action_card(action).map(|id| id.0),
                    "targetCardId": action_target_card(action).map(|id| id.0),
                    "targetPlayer": action_target_player(action, self.human),
                    "targetStackId": action_target_stack(action),
                    "targetCardIds": action_target_cards(action),
                    "targetPlayers": action_target_players(action, self.human),
                    "targetStackIds": action_target_stacks(action),
                    "targetCount": action_targets(action).len(),
                    "manaAbility": matches!(action, Action::ActivateManaAbility { .. }),
                    "spellAction": matches!(action, Action::CastSpell { .. }),
                    "x": match action {
                        Action::CastSpell { x, .. } => Some(*x),
                        _ => None,
                    },
                    "paymentAction": matches!(action, Action::CastSpell { .. } | Action::ActivateAbility { .. }),
                    "manaSourceIds": self.automatic_mana_sources(action),
                })
            })
            .collect::<Vec<_>>();
        let battlefield = observation
            .battlefield
            .iter()
            .map(|permanent| {
                let card = self.catalog.get(permanent.definition);
                let mana_cost = card.map(|card| card.behavior.mana_cost());
                let current_kind = card.map_or("unknown".into(), |card| {
                    if card.behavior == osarena::CardBehavior::MishrasFactory
                        && permanent.power.is_some()
                    {
                        "artifactcreature".into()
                    } else {
                        format!("{:?}", card.behavior.kind()).to_ascii_lowercase()
                    }
                });
                json!({
                    "id": permanent.id.0,
                    "name": self.card_name(permanent.definition),
                    "kind": current_kind,
                    "isLand": card.is_some_and(|card| card.behavior.kind() == osarena::CardKind::Land),
                    "manaCost": mana_cost.map(|cost| json!({
                        "generic": cost.generic,
                        "white": cost.white,
                        "blue": cost.blue,
                        "black": cost.black,
                        "red": cost.red,
                        "green": cost.green,
                        "x": cost.variable_x,
                    })),
                    "rulesText": card.map_or("", |card| card.behavior.rules_text()),
                    "owner": if permanent.controller == self.human { "human" } else { "opponent" },
                    "tapped": permanent.tapped,
                    "power": permanent.power,
                    "toughness": permanent.toughness,
                    "damage": permanent.damage,
                    "attacking": permanent.attacking,
                    "blocking": permanent.blocking.map(|id| id.0),
                    "flying": permanent.flying,
                })
            })
            .collect::<Vec<_>>();
        let hand = observation
            .hand
            .iter()
            .map(|(id, definition)| {
                let card = self.catalog.get(*definition);
                let mana_cost = card.map(|card| card.behavior.mana_cost());
                let creature_stats = card.and_then(|card| card.behavior.creature_stats());
                json!({
                    "id": id.0,
                    "name": self.card_name(*definition),
                    "kind": card.map_or("unknown".into(), |card| {
                        format!("{:?}", card.behavior.kind()).to_ascii_lowercase()
                    }),
                    "isLand": card.is_some_and(|card| card.behavior.kind() == osarena::CardKind::Land),
                    "manaCost": mana_cost.map(|cost| json!({
                        "generic": cost.generic,
                        "white": cost.white,
                        "blue": cost.blue,
                        "black": cost.black,
                        "red": cost.red,
                        "green": cost.green,
                        "x": cost.variable_x,
                    })),
                    "rulesText": card.map_or("", |card| card.behavior.rules_text()),
                    "power": creature_stats.map(|stats| stats.power),
                    "toughness": creature_stats.map(|stats| stats.toughness),
                })
            })
            .collect::<Vec<_>>();
        let stack = observation
            .stack
            .iter()
            .rev()
            .map(|object| {
                json!({
                    "id": object.id.0,
                    "name": self.card_name(object.definition),
                    "owner": if object.controller == self.human { "human" } else { "opponent" },
                    "kind": format!("{:?}", object.kind),
                })
            })
            .collect::<Vec<_>>();
        let graveyard = |player: PlayerId| {
            observation.graveyards[player.index()]
                .iter()
                .rev()
                .map(|(_, definition)| self.card_name(*definition))
                .collect::<Vec<_>>()
        };
        let result = self.game.result().map(|result| match result {
            GameResult::Winner { winner, reason } => json!({
                "outcome": if winner == self.human { "win" } else { "loss" },
                "message": format!(
                    "{} — {}",
                    if winner == self.human { "You win" } else { "You lose" },
                    readable_debug(reason)
                ),
            }),
            GameResult::Draw => json!({"outcome": "draw", "message": "Draw"}),
        });
        let events = self
            .game
            .events()
            .iter()
            .rev()
            .filter_map(|event| self.event_label(&observation, event))
            .take(16)
            .collect::<Vec<_>>();
        let opponent_actions = if include_opponent_actions {
            self.opponent_actions.clone()
        } else {
            Vec::new()
        };

        json!({
            "turn": observation.active_turn,
            "gameTurn": observation.turn,
            "step": readable_debug(observation.step),
            "active": if observation.active_player == self.human { "You" } else { "Opponent" },
            "priority": if observation.priority == self.human { "You" } else { "Opponent" },
            "human": {
                "life": observation.life_totals[self.human.index()],
                "library": observation.library_sizes[self.human.index()],
                "mana": {
                    "white": observation.mana_pools[self.human.index()].white,
                    "blue": observation.mana_pools[self.human.index()].blue,
                    "black": observation.mana_pools[self.human.index()].black,
                    "red": observation.mana_pools[self.human.index()].red,
                    "green": observation.mana_pools[self.human.index()].green,
                    "colorless": observation.mana_pools[self.human.index()].colorless,
                },
                "hand": hand,
                "graveyard": graveyard(self.human),
            },
            "opponent": {
                "life": observation.life_totals[opponent.index()],
                "library": observation.library_sizes[opponent.index()],
                "handSize": observation.opponent_hand_size,
                "mana": {
                    "white": observation.mana_pools[opponent.index()].white,
                    "blue": observation.mana_pools[opponent.index()].blue,
                    "black": observation.mana_pools[opponent.index()].black,
                    "red": observation.mana_pools[opponent.index()].red,
                    "green": observation.mana_pools[opponent.index()].green,
                    "colorless": observation.mana_pools[opponent.index()].colorless,
                },
                "graveyard": graveyard(opponent),
            },
            "battlefield": battlefield,
            "stack": stack,
            "actions": actions,
            "canUndoMana": !self.mana_undo_history.is_empty(),
            "phaseStops": self.phase_stops,
            "autopassEnabled": self.autopass_enabled,
            "opponentActions": opponent_actions,
            "result": result,
            "events": events,
        })
    }

    fn card_name(&self, definition: CardDefinitionId) -> String {
        self.catalog
            .get(definition)
            .map_or_else(|| "Unknown card".into(), |card| card.name.clone())
    }

    fn instance_name(&self, observation: &PlayerObservation, id: CardInstanceId) -> String {
        observation
            .hand
            .iter()
            .find_map(|(candidate, definition)| (*candidate == id).then_some(*definition))
            .or_else(|| {
                observation
                    .battlefield
                    .iter()
                    .find_map(|permanent| (permanent.id == id).then_some(permanent.definition))
            })
            .or_else(|| {
                observation
                    .graveyards
                    .iter()
                    .flatten()
                    .find_map(|(candidate, definition)| (*candidate == id).then_some(*definition))
            })
            .map_or_else(
                || format!("card #{}", id.0),
                |definition| self.card_name(definition),
            )
    }

    fn target_name(&self, observation: &PlayerObservation, target: Target) -> String {
        match target {
            Target::Player(player) if player == self.human => "you".into(),
            Target::Player(_) => "opponent".into(),
            Target::Permanent(id) => self.instance_name(observation, id),
            Target::Spell(id) => observation
                .stack
                .iter()
                .find(|object| object.id == id)
                .map_or_else(
                    || format!("spell #{}", id.0),
                    |object| self.card_name(object.definition),
                ),
        }
    }

    fn player_name(&self, player: PlayerId) -> &'static str {
        if player == self.human { "You" } else { "Opponent" }
    }

    fn event_label(&self, observation: &PlayerObservation, event: &GameEvent) -> Option<String> {
        match event {
            GameEvent::GameStarted { seed } => Some(format!("Game started · seed {seed}")),
            GameEvent::CardDrawn { player, card } if *player == self.human => Some(format!(
                "You drew {}",
                self.instance_name(observation, *card)
            )),
            GameEvent::CardDrawn { .. } => Some("Opponent drew a card".into()),
            GameEvent::LandPlayed { player, card } => Some(format!(
                "{} played {}",
                self.player_name(*player),
                self.instance_name(observation, *card)
            )),
            GameEvent::SpellCast {
                player,
                card,
                targets,
            } => {
                let mut label = format!(
                    "{} cast {}",
                    self.player_name(*player),
                    self.instance_name(observation, *card)
                );
                if !targets.is_empty() {
                    let target_names = targets
                        .iter()
                        .map(|target| self.target_name(observation, *target))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let _ = write!(label, " → {target_names}");
                }
                Some(label)
            }
            GameEvent::AbilityActivated { player, source, .. } => Some(format!(
                "{} activated {}",
                self.player_name(*player),
                self.instance_name(observation, *source)
            )),
            GameEvent::DamageDealt { player, amount } => Some(format!(
                "{} took {amount} damage",
                self.player_name(*player)
            )),
            GameEvent::ManaBurn { player, amount } => Some(format!(
                "{} took {amount} mana burn",
                self.player_name(*player)
            )),
            GameEvent::StepChanged {
                turn,
                active_player,
                step: Step::PrecombatMain,
            } => Some(format!(
                "Turn {} · {} turn",
                (turn + 1) / 2,
                if *active_player == self.human {
                    "your"
                } else {
                    "opponent’s"
                }
            )),
            GameEvent::GameEnded { result } => Some(match result {
                GameResult::Winner { winner, .. } if *winner == self.human => "You won".into(),
                GameResult::Winner { .. } => "Opponent won".into(),
                GameResult::Draw => "Game ended in a draw".into(),
            }),
            GameEvent::ManaAdded { .. }
            | GameEvent::SpellResolved { .. }
            | GameEvent::AbilityResolved { .. }
            | GameEvent::StepChanged { .. } => None,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn action_label(&self, observation: &PlayerObservation, action: &Action) -> String {
        let targets = |values: &[Target]| {
            values
                .iter()
                .map(|target| self.target_name(observation, *target))
                .collect::<Vec<_>>()
                .join(", ")
        };
        match action {
            Action::KeepHand => "Keep this hand".into(),
            Action::TakeMulligan => "Take a mulligan".into(),
            Action::BottomCards { cards } => format!(
                "Bottom {}",
                cards
                    .iter()
                    .map(|id| self.instance_name(observation, *id))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Action::DiscardCards { cards } => format!(
                "Discard {}",
                cards
                    .iter()
                    .map(|id| self.instance_name(observation, *id))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Action::ChooseTriggeredAbility { pay, new_targets } => {
                let chain_lightning = !new_targets.is_empty()
                    || matches!(
                        self.game.events().last(),
                        Some(GameEvent::SpellResolved { card })
                            if self.instance_name(observation, *card) == "Chain Lightning"
                    );
                if chain_lightning {
                    if *pay {
                        format!("Pay RR to copy Chain Lightning → {}", targets(new_targets))
                    } else {
                        "Don't copy Chain Lightning".into()
                    }
                } else if observation.step == Step::Upkeep {
                    if *pay {
                        "Pay 4 to untap Mana Vault".into()
                    } else {
                        "Leave Mana Vault tapped".into()
                    }
                } else if *pay {
                    "Pay 1 to gain 1 life with Iron Star".into()
                } else {
                    "Don't use Iron Star".into()
                }
            }
            Action::ChooseCopyTargets { targets: values } => {
                format!("Copy → {}", targets(values))
            }
            Action::ChooseUntap { permanents } => format!(
                "Untap {}",
                permanents
                    .iter()
                    .map(|id| self.instance_name(observation, *id))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Action::PassPriority => "Pass priority".into(),
            Action::PlayLand { card } => {
                format!("Play {}", self.instance_name(observation, *card))
            }
            Action::ActivateManaAbility { source, color } => {
                format!(
                    "Tap {} for {} mana",
                    self.instance_name(observation, *source),
                    readable_debug(*color)
                )
            }
            Action::CastSpell {
                card,
                targets: values,
                sacrifices,
                x,
            } => {
                let mut label = format!("Cast {}", self.instance_name(observation, *card));
                if *x > 0 {
                    let _ = write!(label, " (X={x})");
                }
                if !sacrifices.is_empty() {
                    let _ = write!(
                        label,
                        " (sacrifice {})",
                        sacrifices
                            .iter()
                            .map(|id| self.instance_name(observation, *id))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                if !values.is_empty() {
                    let _ = write!(label, " → {}", targets(values));
                }
                label
            }
            Action::ActivateAbility {
                source,
                target,
                sacrifice,
            } => {
                let source_name = self.instance_name(observation, *source);
                if source_name == "Mishra's Factory" {
                    return target.map_or_else(
                        || "Make Mishra's Factory a 2/2 creature".into(),
                        |target| {
                            format!(
                                "Give {} +1/+1 with Mishra's Factory",
                                self.target_name(observation, target)
                            )
                        },
                    );
                }
                let mut label = format!("Activate {source_name}");
                if let Some(sacrifice) = sacrifice
                    && sacrifice != source
                {
                    let _ = write!(
                        label,
                        " (sacrifice {})",
                        self.instance_name(observation, *sacrifice)
                    );
                }
                if let Some(target) = target {
                    let _ = write!(label, " → {}", self.target_name(observation, *target));
                }
                label
            }
            Action::DeclareAttacker { attacker } => {
                format!("Attack with {}", self.instance_name(observation, *attacker))
            }
            Action::FinishDeclaringAttackers => "Finish attacking".into(),
            Action::DeclareBlocker { blocker, attacker } => format!(
                "Block {} with {}",
                self.instance_name(observation, *attacker),
                self.instance_name(observation, *blocker)
            ),
            Action::FinishDeclaringBlockers => "Finish blocking".into(),
            Action::AssignCombatDamage {
                attacker,
                assignments,
            } => format!(
                "Assign {} damage from {}",
                assignments
                    .iter()
                    .map(|assignment| format!(
                        "{} to {}",
                        assignment.amount,
                        self.target_name(observation, assignment.recipient)
                    ))
                    .collect::<Vec<_>>()
                    .join(", "),
                self.instance_name(observation, *attacker)
            ),
            Action::Concede => "Concede game".into(),
        }
    }

    fn opponent_action_label(
        &self,
        observation: &PlayerObservation,
        action: &Action,
    ) -> String {
        match action {
            Action::BottomCards { cards } => format!(
                "Bottom {} {}",
                cards.len(),
                if cards.len() == 1 { "card" } else { "cards" }
            ),
            _ => self.action_label(observation, action),
        }
    }
}

fn deck_by_name(name: &str) -> Result<osarena::Deck, JsValue> {
    match name.to_ascii_lowercase().as_str() {
        "goblins" => Ok(poc::goblins()),
        "sligh" => Ok(poc::sligh()),
        "artifacts" => Ok(poc::artifacts()),
        "robots" => Ok(poc::robots()),
        "the deck" => Ok(poc::the_deck()),
        _ => Err(JsValue::from_str("unknown deck")),
    }
}

fn action_kind(action: &Action) -> &'static str {
    match action {
        Action::Concede => "danger",
        Action::PassPriority
        | Action::FinishDeclaringAttackers
        | Action::FinishDeclaringBlockers => "pass",
        Action::DeclareAttacker { .. }
        | Action::DeclareBlocker { .. }
        | Action::AssignCombatDamage { .. } => "combat",
        _ => "primary",
    }
}

fn automatic_human_action_with_blockers(
    step: Step,
    human_is_active: bool,
    stack_is_empty: bool,
    has_attacker: bool,
    has_blocker: bool,
    stop_here: bool,
    autopass_enabled: bool,
    only_human_objects_on_stack: bool,
    human_has_floating_mana: bool,
    actions: &[Action],
) -> Option<Action> {
    if !autopass_enabled {
        return None;
    }
    let mut triggered_choices = actions.iter().filter(|action| {
        matches!(
            action,
            Action::ChooseTriggeredAbility {
                pay: false,
                new_targets,
            } if new_targets.is_empty()
        )
    });
    if let Some(decline) = triggered_choices.next()
        && triggered_choices.next().is_none()
        && !actions.iter().any(|action| {
            matches!(
                action,
                Action::ChooseTriggeredAbility {
                    pay: true,
                    ..
                }
            )
        })
    {
        return Some(decline.clone());
    }
    let has_attack_option = actions
        .iter()
        .any(|action| matches!(action, Action::DeclareAttacker { .. }));
    let empty_combat_after_attackers = !has_attacker
        && (matches!(
            step,
            Step::DeclareBlockers | Step::CombatDamage | Step::EndOfCombat
        ) || (step == Step::DeclareAttackers && !has_attack_option));
    if stop_here && !empty_combat_after_attackers {
        return None;
    }
    if only_human_objects_on_stack
        && let Some(pass) = actions
            .iter()
            .find(|action| matches!(action, Action::PassPriority))
    {
        return Some(pass.clone());
    }
    // Never fast-forward through the human's entire turn. Even with no card
    // actions available, Main 1 is the stable point where the player can see
    // the draw and deliberately advance into combat.
    if human_is_active && step == Step::PrecombatMain && stack_is_empty {
        return None;
    }
    // Arena normally hides routine beginning-phase priority windows on either
    // turn. Stops restore them; floating mana in the end step is a
    // smart-priority case.
    let routine_beginning_step = matches!(step, Step::Upkeep | Step::Draw);
    let routine_own_turn_step = human_is_active
        && (step == Step::BeginningOfCombat
            || (step == Step::End && !human_has_floating_mana));
    let smooth_unblocked_attack = human_is_active
        && has_attacker
        && !has_blocker
        && matches!(
            step,
            Step::DeclareAttackers
                | Step::DeclareBlockers
                | Step::CombatDamage
                | Step::EndOfCombat
        );
    let auto_yield_step = routine_beginning_step
        || routine_own_turn_step
        || smooth_unblocked_attack
        || (!has_attacker
            && matches!(
                step,
                Step::DeclareAttackers
                    | Step::DeclareBlockers
                    | Step::CombatDamage
                    | Step::EndOfCombat
            ));
    if auto_yield_step
        && stack_is_empty
        && let Some(pass) = actions
            .iter()
            .find(|action| matches!(action, Action::PassPriority))
    {
        return Some(pass.clone());
    }
    let has_meaningful_choice = actions.iter().any(|action| {
        !matches!(
            action,
            Action::Concede
                | Action::PassPriority
                | Action::ActivateManaAbility { .. }
                | Action::FinishDeclaringAttackers
                | Action::FinishDeclaringBlockers
        )
    });
    if has_meaningful_choice {
        return None;
    }
    actions
        .iter()
        .find(|action| {
            matches!(
                action,
                Action::PassPriority
                    | Action::FinishDeclaringAttackers
                    | Action::FinishDeclaringBlockers
            )
        })
        .cloned()
}

#[cfg(test)]
fn automatic_human_action(
    step: Step,
    human_is_active: bool,
    stack_is_empty: bool,
    has_attacker: bool,
    stop_here: bool,
    autopass_enabled: bool,
    only_human_objects_on_stack: bool,
    human_has_floating_mana: bool,
    actions: &[Action],
) -> Option<Action> {
    automatic_human_action_with_blockers(
        step,
        human_is_active,
        stack_is_empty,
        has_attacker,
        false,
        stop_here,
        autopass_enabled,
        only_human_objects_on_stack,
        human_has_floating_mana,
        actions,
    )
}

fn action_card(action: &Action) -> Option<CardInstanceId> {
    match action {
        Action::PlayLand { card } | Action::CastSpell { card, .. } => Some(*card),
        Action::ActivateManaAbility { source, .. } | Action::ActivateAbility { source, .. } => {
            Some(*source)
        }
        Action::DeclareAttacker { attacker } | Action::AssignCombatDamage { attacker, .. } => {
            Some(*attacker)
        }
        Action::DeclareBlocker { blocker, .. } => Some(*blocker),
        _ => None,
    }
}

fn action_target_card(action: &Action) -> Option<CardInstanceId> {
    if let Action::DeclareBlocker { attacker, .. } = action {
        return Some(*attacker);
    }
    action_targets(action).iter().find_map(|target| match target {
            Target::Permanent(id) => Some(*id),
            Target::Player(_) | Target::Spell(_) => None,
        })
}

fn action_target_player(action: &Action, human: PlayerId) -> Option<&'static str> {
    action_targets(action).iter().find_map(|target| match target {
        Target::Player(player) => Some(if *player == human { "human" } else { "opponent" }),
        Target::Permanent(_) | Target::Spell(_) => None,
    })
}

fn action_target_stack(action: &Action) -> Option<u32> {
    action_targets(action).iter().find_map(|target| match target {
        Target::Spell(id) => Some(id.0),
        Target::Player(_) | Target::Permanent(_) => None,
    })
}

fn action_targets(action: &Action) -> &[Target] {
    match action {
        Action::CastSpell { targets, .. } => targets.as_slice(),
        Action::ActivateAbility {
            target: Some(target),
            ..
        } => std::slice::from_ref(target),
        _ => &[],
    }
}

fn action_target_cards(action: &Action) -> Vec<u32> {
    action_targets(action)
        .iter()
        .filter_map(|target| match target {
            Target::Permanent(id) => Some(id.0),
            Target::Player(_) | Target::Spell(_) => None,
        })
        .collect()
}

fn action_target_players(action: &Action, human: PlayerId) -> Vec<&'static str> {
    action_targets(action)
        .iter()
        .filter_map(|target| match target {
            Target::Player(player) => {
                Some(if *player == human { "human" } else { "opponent" })
            }
            Target::Permanent(_) | Target::Spell(_) => None,
        })
        .collect()
}

fn action_target_stacks(action: &Action) -> Vec<u32> {
    action_targets(action)
        .iter()
        .filter_map(|target| match target {
            Target::Spell(id) => Some(id.0),
            Target::Player(_) | Target::Permanent(_) => None,
        })
        .collect()
}

fn should_animate_action(action: &Action) -> bool {
    !matches!(
        action,
        Action::KeepHand
            | Action::TakeMulligan
            | Action::BottomCards { .. }
            | Action::Concede
            | Action::PassPriority
            | Action::ActivateManaAbility { .. }
            | Action::FinishDeclaringAttackers
            | Action::FinishDeclaringBlockers
    )
}

fn animated_action_kind(action: &Action) -> &'static str {
    match action {
        Action::PlayLand { .. } => "land",
        Action::CastSpell { .. } => "spell",
        Action::ActivateAbility { .. } => "ability",
        Action::DeclareAttacker { .. }
        | Action::DeclareBlocker { .. }
        | Action::AssignCombatDamage { .. } => "combat",
        Action::KeepHand
        | Action::TakeMulligan
        | Action::BottomCards { .. }
        | Action::DiscardCards { .. }
        | Action::ChooseTriggeredAbility { .. }
        | Action::ChooseCopyTargets { .. }
        | Action::ChooseUntap { .. } => "choice",
        Action::Concede
        | Action::PassPriority
        | Action::ActivateManaAbility { .. }
        | Action::FinishDeclaringAttackers
        | Action::FinishDeclaringBlockers => "quiet",
    }
}

fn readable_debug(value: impl std::fmt::Debug) -> String {
    let source = format!("{value:?}");
    let mut output = String::with_capacity(source.len() + 4);
    for (index, character) in source.chars().enumerate() {
        if index > 0 && character.is_ascii_uppercase() {
            output.push(' ');
        }
        output.push(character);
    }
    output
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocker_actions_expose_the_attacker_as_their_board_target() {
        let attacker = CardInstanceId(7);
        let blocker = CardInstanceId(8);
        let action = Action::DeclareBlocker { blocker, attacker };
        assert_eq!(action_card(&action), Some(blocker));
        assert_eq!(action_target_card(&action), Some(attacker));
    }

    #[test]
    fn human_main_one_stops_even_when_only_mana_actions_are_available() {
        let actions = [
            Action::Concede,
            Action::ActivateManaAbility {
                source: CardInstanceId(7),
                color: osarena::ManaColor::Red,
            },
            Action::PassPriority,
        ];

        assert_eq!(
            automatic_human_action(
                Step::PrecombatMain,
                true,
                true,
                false,
                false,
                true,
                false,
                false,
                &actions,
            ),
            None
        );
        assert_eq!(
            automatic_human_action(
                Step::PrecombatMain,
                false,
                true,
                false,
                false,
                true,
                false,
                false,
                &actions,
            ),
            Some(Action::PassPriority),
            "an actionless opponent main phase can still auto-yield",
        );
    }

    #[test]
    fn a_real_game_action_still_stops_auto_pass() {
        let actions = [
            Action::Concede,
            Action::PlayLand {
                card: CardInstanceId(7),
            },
            Action::PassPriority,
        ];

        assert_eq!(
            automatic_human_action(
                Step::PrecombatMain,
                true,
                true,
                false,
                false,
                true,
                false,
                false,
                &actions,
            ),
            None
        );
    }

    #[test]
    fn an_unpayable_triggered_option_auto_declines() {
        let actions = [
            Action::Concede,
            Action::ChooseTriggeredAbility {
                pay: false,
                new_targets: Vec::new(),
            },
        ];

        assert_eq!(
            automatic_human_action(
                Step::PrecombatMain,
                false,
                true,
                false,
                false,
                true,
                false,
                false,
                &actions,
            ),
            Some(Action::ChooseTriggeredAbility {
                pay: false,
                new_targets: Vec::new(),
            })
        );
    }

    #[test]
    fn a_payable_triggered_option_still_asks_the_player() {
        let actions = [
            Action::Concede,
            Action::ChooseTriggeredAbility {
                pay: false,
                new_targets: Vec::new(),
            },
            Action::ChooseTriggeredAbility {
                pay: true,
                new_targets: vec![Target::Player(PlayerId::Two)],
            },
        ];

        assert_eq!(
            automatic_human_action(
                Step::PrecombatMain,
                false,
                true,
                false,
                false,
                true,
                false,
                false,
                &actions,
            ),
            None
        );
    }

    #[test]
    fn pregame_choices_do_not_enter_the_animation_queue() {
        assert!(!should_animate_action(&Action::KeepHand));
        assert!(!should_animate_action(&Action::TakeMulligan));
        assert!(!should_animate_action(&Action::BottomCards {
            cards: vec![CardInstanceId(4)],
        }));
        assert!(should_animate_action(&Action::PlayLand {
            card: CardInstanceId(4),
        }));
    }

    #[test]
    fn routine_beginning_windows_auto_pass_on_either_turn() {
        let actions = [
            Action::Concede,
            Action::CastSpell {
                card: CardInstanceId(7),
                targets: vec![Target::Player(PlayerId::Two)],
                sacrifices: Vec::new(),
                x: 0,
            },
            Action::PassPriority,
        ];

        assert_eq!(
            automatic_human_action(
                Step::Upkeep,
                true,
                true,
                false,
                false,
                true,
                false,
                false,
                &actions,
            ),
            Some(Action::PassPriority)
        );
        assert_eq!(
            automatic_human_action(
                Step::Draw,
                true,
                true,
                false,
                false,
                true,
                false,
                false,
                &actions,
            ),
            Some(Action::PassPriority)
        );
        assert_eq!(
            automatic_human_action(
                Step::End,
                true,
                true,
                false,
                false,
                true,
                false,
                false,
                &actions,
            ),
            Some(Action::PassPriority)
        );
        assert_eq!(
            automatic_human_action(
                Step::Upkeep,
                false,
                true,
                false,
                false,
                true,
                false,
                false,
                &actions,
            ),
            Some(Action::PassPriority),
            "routine opponent upkeep priority is hidden unless the player sets a stop",
        );
        assert_eq!(
            automatic_human_action(
                Step::Draw,
                false,
                true,
                false,
                false,
                true,
                false,
                false,
                &actions,
            ),
            Some(Action::PassPriority),
            "routine opponent draw-step priority is hidden unless the player sets a stop",
        );
        assert_eq!(
            automatic_human_action(
                Step::End,
                true,
                true,
                false,
                false,
                true,
                false,
                true,
                &actions,
            ),
            None,
            "smart priority preserves floating mana in the human's end step",
        );
    }

    #[test]
    fn empty_and_unblocked_combat_steps_auto_pass() {
        let actions = [
            Action::Concede,
            Action::CastSpell {
                card: CardInstanceId(7),
                targets: vec![Target::Player(PlayerId::Two)],
                sacrifices: Vec::new(),
                x: 0,
            },
            Action::PassPriority,
        ];

        assert_eq!(
            automatic_human_action(
                Step::BeginningOfCombat,
                true,
                true,
                false,
                false,
                true,
                false,
                false,
                &actions,
            ),
            Some(Action::PassPriority)
        );
        assert_eq!(
            automatic_human_action(
                Step::CombatDamage,
                true,
                true,
                false,
                false,
                true,
                false,
                false,
                &actions,
            ),
            Some(Action::PassPriority)
        );
        assert_eq!(
            automatic_human_action(
                Step::CombatDamage,
                true,
                true,
                true,
                false,
                true,
                false,
                false,
                &actions,
            ),
            Some(Action::PassPriority),
            "an unblocked attack runs through combat damage without extra clicks",
        );
        assert_eq!(
            automatic_human_action_with_blockers(
                Step::DeclareBlockers,
                true,
                true,
                true,
                true,
                false,
                true,
                false,
                false,
                &actions,
            ),
            None,
            "a declared blocker interrupts smooth combat",
        );
    }

    #[test]
    fn a_combat_stop_interrupts_an_unblocked_attack() {
        let actions = [Action::Concede, Action::PassPriority];
        assert_eq!(
            automatic_human_action_with_blockers(
                Step::CombatDamage,
                true,
                true,
                true,
                false,
                true,
                true,
                false,
                false,
                &actions,
            ),
            None
        );
    }

    #[test]
    fn a_phase_stop_blocks_the_ui_auto_pass() {
        let actions = [Action::Concede, Action::PassPriority];
        assert_eq!(
            automatic_human_action(
                Step::Upkeep,
                true,
                true,
                false,
                true,
                true,
                false,
                false,
                &actions,
            ),
            None
        );
    }

    #[test]
    fn no_attackers_skip_the_rest_of_combat_even_with_a_combat_stop() {
        let actions = [Action::Concede, Action::PassPriority];
        assert_eq!(
            automatic_human_action(
                Step::CombatDamage,
                true,
                true,
                false,
                true,
                true,
                false,
                false,
                &actions,
            ),
            Some(Action::PassPriority)
        );
        assert_eq!(
            automatic_human_action(
                Step::CombatDamage,
                true,
                true,
                false,
                true,
                false,
                false,
                false,
                &actions,
            ),
            None,
            "turning auto-pass off still exposes the empty combat window",
        );
    }

    #[test]
    fn autopass_yields_when_only_human_objects_are_on_the_stack() {
        let actions = [
            Action::Concede,
            Action::CastSpell {
                card: CardInstanceId(7),
                targets: vec![Target::Player(PlayerId::Two)],
                sacrifices: Vec::new(),
                x: 0,
            },
            Action::PassPriority,
        ];
        assert_eq!(
            automatic_human_action(
                Step::PrecombatMain,
                true,
                false,
                false,
                false,
                true,
                true,
                false,
                &actions,
            ),
            Some(Action::PassPriority)
        );
        assert_eq!(
            automatic_human_action(
                Step::PrecombatMain,
                true,
                false,
                false,
                false,
                false,
                true,
                false,
                &actions,
            ),
            None
        );
    }
}
