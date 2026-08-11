mod action_view;
mod autopass;
mod hosted;
mod labels;
mod pacing;
mod presentation;
mod session;
mod snapshot;

use penta::card;
use penta::game::{DecisionKind, DecisionOrderSemantics};
use penta::{
    AbilityOrigin, Action, BattlefieldExit, CardCatalog, CardDefinitionId, CardInstanceId, Format,
    Game, GameEvent, GameResult, HandcraftedPolicy, ModeId, PlayOptionId, PlayerId,
    PlayerObservation, Policy, RandomPolicy, Step, Target,
};
use presentation::deck_by_name;
use serde_json::{Value, json};
use session::{Checkpoint, LocalSession};
use wasm_bindgen::prelude::*;

#[cfg(test)]
use action_view::{
    action_ability_origin, action_card, action_target_card, action_target_cards,
    action_target_player, action_target_players, action_target_stack, action_target_stacks,
    cast_signature_value, should_animate_action,
};
#[cfg(test)]
use autopass::{
    AutoPassContext, automatic_human_action, automatic_human_action_for_context,
    automatic_human_action_with_blockers,
};
#[cfg(test)]
use presentation::{
    StackCardPresentation, card_art_value, hand_mana_cost_value, stack_card_presentation,
};

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
    session: LocalSession,
    catalog: CardCatalog,
    human: PlayerId,
    bot: BotPolicy,
    opponent_actions: Vec<Value>,
    pending_opponent_mana: Vec<String>,
    mana_undo_history: Vec<Checkpoint>,
    phase_stops: Vec<String>,
    autopass_enabled: bool,
    attack_undo: Option<Checkpoint>,
    /// The turn the presentation has already announced, so a turn nobody acts
    /// on still gets its banner instead of being skipped over in silence.
    announced_turn: Option<u32>,
    /// The board the moment your own action landed, before the game answered.
    human_action_state: Option<Value>,
}

#[wasm_bindgen]
impl WebGame {
    /// Creates a mirror-format game and advances until the human must decide.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error when a deck or policy name is unknown, game
    /// construction fails, or the bot cannot reach a human decision.
    #[allow(clippy::needless_pass_by_value)] // wasm-bindgen owns optional strings at the ABI.
    #[wasm_bindgen(constructor)]
    pub fn new(
        human_deck: &str,
        bot_deck: &str,
        bot_policy: &str,
        human_first: bool,
        seed: u32,
        format: Option<String>,
    ) -> Result<WebGame, JsValue> {
        let format = penta::protocol::parse_format_slug(
            format.as_deref().unwrap_or(Format::OldSchool9394.slug()),
        )
        .map_err(js_error)?;
        let catalog = card::catalog().map_err(js_error)?;
        let human_deck = deck_by_name(format, human_deck)?;
        let bot_deck = deck_by_name(format, bot_deck)?;
        let human = if human_first {
            PlayerId::One
        } else {
            PlayerId::Two
        };
        let decks = match human {
            PlayerId::One => [human_deck, bot_deck],
            PlayerId::Two => [bot_deck, human_deck],
        };
        let game = Game::new_with_format(format, catalog.clone(), decks, u64::from(seed))
            .map_err(js_error)?;
        let bot = match bot_policy.to_ascii_lowercase().as_str() {
            "random" => BotPolicy::Random(RandomPolicy::new(u64::from(seed) ^ 0x00b0_7b07)),
            "handcrafted" => BotPolicy::Handcrafted(HandcraftedPolicy::new(catalog.clone())),
            _ => return Err(JsValue::from_str("unknown bot policy")),
        };
        let mut web_game = Self {
            session: LocalSession::new(game),
            catalog,
            human,
            bot,
            opponent_actions: Vec::new(),
            pending_opponent_mana: Vec::new(),
            mana_undo_history: Vec::new(),
            phase_stops: Vec::new(),
            autopass_enabled: true,
            attack_undo: None,
            // The opening turn arrives with the board, not as a change to it.
            announced_turn: Some(1),
            human_action_state: None,
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
        if self.session.decision_seat() != Some(self.human) {
            return Err(JsValue::from_str("the game is not waiting for the human"));
        }
        let observation = self.session.observe(self.human);
        let action = observation
            .legal_actions
            .get(action_index)
            .cloned()
            .ok_or_else(|| JsValue::from_str("unknown legal action"))?;
        self.apply_human_action(action)
    }

    /// Submits the selected option IDs for the current generic decision.
    ///
    /// The selection is validated by the engine, so the browser does not need
    /// to receive an eagerly-expanded action for every possible combination.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error when the game is not waiting for the human,
    /// the JSON is malformed, or the engine rejects the selection.
    pub fn choose_decision(&mut self, decision: u32, options_json: &str) -> Result<(), JsValue> {
        if self.session.decision_seat() != Some(self.human) {
            return Err(JsValue::from_str("the game is not waiting for the human"));
        }
        let options: Vec<u32> = serde_json::from_str(options_json).map_err(js_error)?;
        self.apply_human_action(Action::ChooseDecision { decision, options })
    }

    fn apply_human_action(&mut self, action: Action) -> Result<(), JsValue> {
        let mana_checkpoint =
            matches!(action, Action::ActivateManaAbility { .. }).then(|| self.session.checkpoint());
        if mana_checkpoint.is_none() {
            self.mana_undo_history.clear();
        }
        // The first declaration of the combat is the point a cancel returns to.
        if matches!(action, Action::DeclareAttacker { .. }) && self.attack_undo.is_none() {
            self.attack_undo = Some(self.session.checkpoint());
        }
        self.opponent_actions.clear();
        self.pending_opponent_mana.clear();
        let event_start = self.session.event_cursor();
        self.session.apply(self.human, action).map_err(js_error)?;
        // What you just did, before anything the game does in response. The
        // replay is told from here, so a land you played is on the board
        // before the turn it ended is announced.
        self.human_action_state = Some(self.snapshot_value(false));
        // Yielding is how combat damage happens, and ending your own turn hands
        // one to the opponent. Both need showing every bit as much as the
        // actions the bot takes on its own.
        self.record_combat_damage(event_start);
        self.record_draw_step(event_start);
        self.record_turn_change(event_start);
        // Committing the attack invalidates the checkpoint immediately, so no
        // snapshot taken while the turn plays out still offers the cancel.
        self.forget_attack_undo_unless_still_declaring();
        self.advance_until_human_choice()?;
        self.forget_attack_undo_unless_still_declaring();
        if let Some(checkpoint) = mana_checkpoint {
            let before = checkpoint.observed_by(self.human);
            let after = self.session.observe(self.human);
            if self.session.decision_seat() == Some(self.human)
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
        if self.session.decision_seat() != Some(self.human)
            || self.session.observe(self.human).step != Step::DeclareAttackers
        {
            return Err(JsValue::from_str("the human is not declaring attackers"));
        }
        self.mana_undo_history.clear();
        self.opponent_actions.clear();
        self.pending_opponent_mana.clear();
        if self.attack_undo.is_none() {
            self.attack_undo = Some(self.session.checkpoint());
        }
        loop {
            let action = self
                .session
                .observe(self.human)
                .legal_actions
                .into_iter()
                .find(|action| matches!(action, Action::DeclareAttacker { .. }));
            let Some(action) = action else {
                break;
            };
            self.session.apply(self.human, action).map_err(js_error)?;
        }
        if let Some(finish) = self
            .session
            .observe(self.human)
            .legal_actions
            .into_iter()
            .find(|action| matches!(action, Action::FinishDeclaringAttackers))
        {
            self.session.apply(self.human, finish).map_err(js_error)?;
        }
        self.forget_attack_undo_unless_still_declaring();
        self.advance_until_human_choice()?;
        self.forget_attack_undo_unless_still_declaring();
        Ok(())
    }

    /// Takes back every attacker declared so far this combat.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error when the attack has already been committed.
    pub fn cancel_attackers(&mut self) -> Result<(), JsValue> {
        let previous = self
            .attack_undo
            .take()
            .ok_or_else(|| JsValue::from_str("there are no declared attackers to take back"))?;
        self.session.restore(previous);
        self.mana_undo_history.clear();
        self.opponent_actions.clear();
        self.pending_opponent_mana.clear();
        Ok(())
    }

    /// A cancel is only offered while the attack is still being assembled;
    /// once it is committed the declaration is part of the game.
    fn forget_attack_undo_unless_still_declaring(&mut self) {
        if self.attack_undo.is_none() {
            return;
        }
        let still_declaring = self.session.decision_seat() == Some(self.human)
            && self
                .session
                .observe(self.human)
                .legal_actions
                .iter()
                .any(|action| matches!(action, Action::FinishDeclaringAttackers));
        if !still_declaring {
            self.attack_undo = None;
        }
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
        if self.session.decision_seat() != Some(self.human)
            || self.session.observe(self.human).step != Step::DeclareBlockers
        {
            return Err(JsValue::from_str("the human is not declaring blockers"));
        }
        let assignments: Vec<[u32; 2]> =
            serde_json::from_str(assignments_json).map_err(js_error)?;
        let mut used_blockers = Vec::with_capacity(assignments.len());
        let legal_actions = self.session.observe(self.human).legal_actions;
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
            self.session.apply(self.human, action).map_err(js_error)?;
        }
        self.session
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
        self.session.restore(previous);
        self.opponent_actions.clear();
        self.pending_opponent_mana.clear();
        Ok(())
    }

    /// Enables or disables a human-interface stop for one displayed phase.
    /// The rules engine still exposes every individual step.
    /// Sets or clears a UI phase stop.
    ///
    /// # Errors
    ///
    /// Returns an error if advancing the facade encounters an invalid engine action.
    pub fn set_phase_stop(&mut self, phase: &str, enabled: bool) -> Result<(), JsValue> {
        if !matches!(
            phase,
            "Beginning" | "Main 1" | "Combat" | "Main 2" | "Ending"
        ) {
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
    /// Enables or disables routine UI priority passing.
    ///
    /// # Errors
    ///
    /// Returns an error if advancing the facade encounters an invalid engine action.
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

    /// Puts a named card onto a seat's battlefield, for reaching a board state
    /// without playing toward one.
    ///
    /// Compiled only with the `dev-cheats` feature, which the production web
    /// build never enables. `seat` is `"human"` or `"bot"`.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error when the seat name is unknown, no card has
    /// that name, or the game cannot take another object.
    #[cfg(feature = "dev-cheats")]
    pub fn dev_put_onto_battlefield(&mut self, seat: &str, card_name: &str) -> Result<(), JsValue> {
        let player = match seat {
            "human" => self.human,
            "bot" => self.human.opponent(),
            other => {
                return Err(js_error(format!(
                    "seat must be \"human\" or \"bot\", got {other:?}"
                )));
            }
        };
        let definition = self
            .catalog
            .find_by_name(card_name)
            .ok_or_else(|| js_error(format!("no card named {card_name:?}")))?;
        self.session
            .put_onto_battlefield(player, definition)
            .map_err(|error| js_error(error.to_string()))?;
        Ok(())
    }

    /// Puts a named card straight into a seat's graveyard, for testing zones
    /// the browser cannot otherwise reach. Compiled only with `dev-cheats`.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error when the seat name is unknown, no card has
    /// that name, or the game cannot take another object.
    #[cfg(feature = "dev-cheats")]
    pub fn dev_put_into_graveyard(&mut self, seat: &str, card_name: &str) -> Result<(), JsValue> {
        let player = match seat {
            "human" => self.human,
            "bot" => self.human.opponent(),
            other => {
                return Err(js_error(format!(
                    "seat must be \"human\" or \"bot\", got {other:?}"
                )));
            }
        };
        let definition = self
            .catalog
            .find_by_name(card_name)
            .ok_or_else(|| js_error(format!("no card named {card_name:?}")))?;
        self.session
            .put_into_graveyard(player, definition)
            .map_err(|error| js_error(error.to_string()))?;
        Ok(())
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
mod tests;
