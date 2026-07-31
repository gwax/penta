use osarena::poc;
use osarena::{
    Action, CardCatalog, CardDefinitionId, CardInstanceId, Game, GameResult, HandcraftedPolicy,
    PlayerId, PlayerObservation, Policy, RandomPolicy, Target,
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
        self.opponent_actions.clear();
        self.pending_opponent_mana.clear();
        self.game.apply(self.human, action).map_err(js_error)?;
        self.advance_until_human_choice()
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
                let Some(action) = automatic_human_action(&observation.legal_actions) else {
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
                if let Action::ActivateManaAbility { source } = &action {
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
                    let label = self.action_label(&observation, &action);
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
            self.game.apply(player, action).map_err(js_error)?;
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
                })
            })
            .collect::<Vec<_>>();
        let battlefield = observation
            .battlefield
            .iter()
            .map(|permanent| {
                let card = self.catalog.get(permanent.definition);
                let mana_cost = card.map(|card| card.behavior.mana_cost());
                json!({
                    "id": permanent.id.0,
                    "name": self.card_name(permanent.definition),
                    "kind": card.map_or("unknown".into(), |card| {
                        format!("{:?}", card.behavior.kind()).to_ascii_lowercase()
                    }),
                    "manaCost": mana_cost.map(|cost| json!({
                        "generic": cost.generic,
                        "red": cost.red,
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
                    "manaCost": mana_cost.map(|cost| json!({
                        "generic": cost.generic,
                        "red": cost.red,
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
            .take(12)
            .map(|event| format!("{event:?}"))
            .collect::<Vec<_>>();
        let opponent_actions = if include_opponent_actions {
            self.opponent_actions.clone()
        } else {
            Vec::new()
        };

        json!({
            "turn": observation.turn,
            "step": readable_debug(observation.step),
            "active": if observation.active_player == self.human { "You" } else { "Opponent" },
            "priority": if observation.priority == self.human { "You" } else { "Opponent" },
            "human": {
                "life": observation.life_totals[self.human.index()],
                "library": observation.library_sizes[self.human.index()],
                "mana": {
                    "red": observation.mana_pools[self.human.index()].red,
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
                    "red": observation.mana_pools[opponent.index()].red,
                    "colorless": observation.mana_pools[opponent.index()].colorless,
                },
                "graveyard": graveyard(opponent),
            },
            "battlefield": battlefield,
            "stack": stack,
            "actions": actions,
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
                let choice = if *pay { "Pay" } else { "Decline" };
                if new_targets.is_empty() {
                    choice.into()
                } else {
                    format!("{choice} → {}", targets(new_targets))
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
            Action::ActivateManaAbility { source } => {
                format!("Tap {} for mana", self.instance_name(observation, *source))
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
                let mut label = format!("Activate {}", self.instance_name(observation, *source));
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
}

fn deck_by_name(name: &str) -> Result<osarena::Deck, JsValue> {
    match name.to_ascii_lowercase().as_str() {
        "goblins" => Ok(poc::goblins()),
        "sligh" => Ok(poc::sligh()),
        "artifacts" => Ok(poc::artifacts()),
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

fn automatic_human_action(actions: &[Action]) -> Option<Action> {
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

fn action_card(action: &Action) -> Option<CardInstanceId> {
    match action {
        Action::PlayLand { card } | Action::CastSpell { card, .. } => Some(*card),
        Action::ActivateManaAbility { source } | Action::ActivateAbility { source, .. } => {
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
    match action {
        Action::CastSpell { targets, .. } => targets.iter().find_map(|target| match target {
            Target::Permanent(id) => Some(*id),
            Target::Player(_) | Target::Spell(_) => None,
        }),
        Action::ActivateAbility {
            target: Some(Target::Permanent(id)),
            ..
        } => Some(*id),
        _ => None,
    }
}

fn should_animate_action(action: &Action) -> bool {
    !matches!(
        action,
        Action::Concede
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
    fn mana_taps_do_not_stop_an_automatic_priority_pass() {
        let actions = [
            Action::Concede,
            Action::ActivateManaAbility {
                source: CardInstanceId(7),
            },
            Action::PassPriority,
        ];

        assert_eq!(automatic_human_action(&actions), Some(Action::PassPriority));
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

        assert_eq!(automatic_human_action(&actions), None);
    }
}
