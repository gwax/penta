//! The canonical wire format for bots, and a facade for driving games with it.
//!
//! Everything a bot ever sees crosses this boundary as JSON produced here:
//! the Python bindings, the C FFI, and any future tournament server all call
//! these functions, so a bot trained locally and a bot competing remotely
//! read byte-identical observations. Treat any change to the shapes below as
//! a protocol version bump.
//!
//! Seats are named `"p1"` ([`PlayerId::One`]) and `"p2"` ([`PlayerId::Two`]).
//! A bot acts by sending back the `index` of one of its observation's
//! `legalActions`; the engine validates every index against the legal list,
//! so no illegal move can be expressed at all.

use serde_json::{Value, json};

use crate::card::CardDefinition;
use crate::game::DecisionObservation;
use crate::policy::Policy;
use crate::{
    Action, CardCatalog, CardInstanceId, Deck, Game, GameResult, HandcraftedPolicy, ManaColor,
    PlayerId, PlayerObservation, RandomPolicy, StackObjectKind, Target, WinReason, poc,
};

/// Bumped whenever the JSON shapes in this module change incompatibly.
pub const PROTOCOL_VERSION: u32 = 0;

/// The engine crate version, for pinning trained bots to rules behavior.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One shy of the engine's own replay guard, so a runaway game fails here
/// with a protocol error instead of an engine panic.
const ACTION_LIMIT: usize = 50_000;

/// The deck names accepted by [`deck_by_name`], in menu order.
#[must_use]
pub fn deck_names() -> Vec<&'static str> {
    vec![
        "Goblins",
        "Sligh",
        "Artifacts",
        "Robots",
        "The Deck",
        "Mono Black",
        "White Weenie",
        "Erhnamgeddon",
        "Counterburn",
        "Lions DIB",
        "Lion Dib Bolt",
        "BWR Aggro",
        "GR Aggro",
        "Troll Disk",
        "Jeskai Aggro",
    ]
}

/// Looks up a built-in deck by its display name, case-insensitively.
#[must_use]
pub fn deck_by_name(name: &str) -> Option<Deck> {
    match name.to_ascii_lowercase().as_str() {
        "goblins" => Some(poc::goblins()),
        "sligh" => Some(poc::sligh()),
        "artifacts" => Some(poc::artifacts()),
        "robots" => Some(poc::robots()),
        "the deck" => Some(poc::the_deck()),
        "mono black" => Some(poc::mono_black()),
        "white weenie" => Some(poc::white_weenie()),
        "erhnamgeddon" => Some(poc::erhnamgeddon()),
        "counterburn" => Some(poc::counterburn()),
        "lions/dib" | "lions dib" => Some(poc::lions_dib()),
        "bwr aggro" => Some(poc::bwr_aggro()),
        "gr aggro" => Some(poc::gr_aggro()),
        "troll disk" => Some(poc::troll_disk()),
        "jeskai aggro" => Some(poc::jeskai_aggro()),
        "lion dib bolt" | "lions/dib bolt" | "lions dib bolt" => Some(poc::lions_dib_bolt()),
        _ => None,
    }
}

fn seat_name(player: PlayerId) -> &'static str {
    match player {
        PlayerId::One => "p1",
        PlayerId::Two => "p2",
    }
}

fn seat_by_name(name: &str) -> Option<PlayerId> {
    match name {
        "p1" => Some(PlayerId::One),
        "p2" => Some(PlayerId::Two),
        _ => None,
    }
}

fn target_json(target: Target) -> Value {
    match target {
        Target::Player(player) => json!({ "type": "player", "seat": seat_name(player) }),
        Target::Permanent(id) => json!({ "type": "permanent", "instance": id.0 }),
        Target::Spell(id) => json!({ "type": "spell", "stackId": id.0 }),
    }
}

fn instances_json(cards: &[CardInstanceId]) -> Value {
    Value::from(cards.iter().map(|card| card.0).collect::<Vec<_>>())
}

const fn mana_color_name(color: ManaColor) -> &'static str {
    match color {
        ManaColor::White => "white",
        ManaColor::Blue => "blue",
        ManaColor::Black => "black",
        ManaColor::Red => "red",
        ManaColor::Green => "green",
        ManaColor::Colorless => "colorless",
    }
}

/// Serializes one legal action. The `type` tag names the engine's action
/// variant; the remaining fields identify what it operates on.
#[must_use]
pub fn action_json(action: &Action) -> Value {
    match action {
        Action::KeepHand => json!({ "type": "KeepHand" }),
        Action::TakeMulligan => json!({ "type": "TakeMulligan" }),
        Action::BottomCards { cards } => {
            json!({ "type": "BottomCards", "cards": instances_json(cards) })
        }
        Action::DiscardCards { cards } => {
            json!({ "type": "DiscardCards", "cards": instances_json(cards) })
        }
        Action::ChooseDecision { decision, options } => {
            json!({ "type": "ChooseDecision", "decision": decision, "options": options })
        }
        Action::CancelDecision { decision } => {
            json!({ "type": "CancelDecision", "decision": decision })
        }
        Action::ChooseUntap { permanents } => {
            json!({ "type": "ChooseUntap", "permanents": instances_json(permanents) })
        }
        Action::PassPriority => json!({ "type": "PassPriority" }),
        Action::PlayLand { card } => json!({ "type": "PlayLand", "card": card.0 }),
        Action::ActivateManaAbility { source, color } => json!({
            "type": "ActivateManaAbility",
            "source": source.0,
            "color": mana_color_name(*color),
        }),
        Action::PayLifeForMana => json!({ "type": "PayLifeForMana" }),
        Action::CastSpell {
            card,
            targets,
            sacrifices,
            x,
        } => json!({
            "type": "CastSpell",
            "card": card.0,
            "targets": targets.iter().map(|target| target_json(*target)).collect::<Vec<_>>(),
            "sacrifices": instances_json(sacrifices),
            "x": x,
        }),
        Action::ActivateAbility {
            source,
            target,
            sacrifice,
        } => json!({
            "type": "ActivateAbility",
            "source": source.0,
            "target": target.map(target_json),
            "sacrifice": sacrifice.map(|card| card.0),
        }),
        Action::DeclareAttacker { attacker } => {
            json!({ "type": "DeclareAttacker", "attacker": attacker.0 })
        }
        Action::FinishDeclaringAttackers => json!({ "type": "FinishDeclaringAttackers" }),
        Action::DeclareBlocker { blocker, attacker } => {
            json!({ "type": "DeclareBlocker", "blocker": blocker.0, "attacker": attacker.0 })
        }
        Action::FinishDeclaringBlockers => json!({ "type": "FinishDeclaringBlockers" }),
        Action::AssignCombatDamage {
            attacker,
            assignments,
        } => json!({
            "type": "AssignCombatDamage",
            "attacker": attacker.0,
            "assignments": assignments.iter().map(|assignment| json!({
                "recipient": target_json(assignment.recipient),
                "amount": assignment.amount,
            })).collect::<Vec<_>>(),
        }),
        Action::Concede => json!({ "type": "Concede" }),
    }
}

fn card_name(catalog: &CardCatalog, definition: crate::CardDefinitionId) -> Value {
    catalog
        .get(definition)
        .map_or(Value::Null, |card| Value::from(card.name.clone()))
}

fn card_list_json(
    catalog: &CardCatalog,
    cards: &[(CardInstanceId, crate::CardDefinitionId)],
) -> Value {
    Value::from(
        cards
            .iter()
            .map(|(instance, definition)| {
                json!({
                    "instance": instance.0,
                    "definition": definition.0,
                    "name": card_name(catalog, *definition),
                })
            })
            .collect::<Vec<_>>(),
    )
}

fn mana_pool_json(pool: &crate::ManaPool) -> Value {
    json!({
        "white": pool.white,
        "blue": pool.blue,
        "black": pool.black,
        "red": pool.red,
        "green": pool.green,
        "colorless": pool.colorless,
    })
}

fn decision_json(catalog: &CardCatalog, decision: &DecisionObservation) -> Value {
    json!({
        "id": decision.id,
        "seat": seat_name(decision.player),
        "prompt": decision.prompt,
        "visibility": format!("{:?}", decision.visibility),
        "minimum": decision.minimum,
        "maximum": decision.maximum,
        "cancellable": decision.cancellable,
        "options": decision.options.iter().map(|option| json!({
            "id": option.id,
            "label": option.label,
            "card": option.card.map(|(instance, definition)| json!({
                "instance": instance.0,
                "definition": definition.0,
                "name": card_name(catalog, definition),
            })),
            "zone": format!("{:?}", option.zone),
        })).collect::<Vec<_>>(),
    })
}

fn result_json(result: GameResult) -> Value {
    match result {
        GameResult::Draw => json!({ "winner": Value::Null, "reason": "Draw" }),
        GameResult::Winner { winner, reason } => json!({
            "winner": seat_name(winner),
            "reason": match reason {
                WinReason::OpponentConceded => "OpponentConceded",
                WinReason::OpponentLostAllLife => "OpponentLostAllLife",
                WinReason::OpponentTriedToDrawFromEmptyLibrary =>
                    "OpponentTriedToDrawFromEmptyLibrary",
            },
        }),
    }
}

/// Expands the engine's action list into the protocol's.
///
/// The engine lists a pending decision as one template action with empty
/// `options`, expecting the caller to fill in ids from the decision schema.
/// Bots act by index, so the protocol expands the template: a pick-exactly-one
/// decision becomes one concrete action per option, and a multi-pick keeps a
/// default choice of the first `minimum` options so an index-only bot always
/// has a legal move. Bots that want a different multi-pick send it through
/// [`BotGame::choose_decision`].
#[must_use]
pub fn protocol_actions(observation: &PlayerObservation) -> Vec<Action> {
    let mut actions = Vec::with_capacity(observation.legal_actions.len());
    for action in &observation.legal_actions {
        match (action, observation.decision.as_ref()) {
            (Action::ChooseDecision { decision, options }, Some(pending))
                if options.is_empty() && *decision == pending.id =>
            {
                if pending.minimum == 1 && pending.maximum == 1 {
                    for option in &pending.options {
                        actions.push(Action::ChooseDecision {
                            decision: *decision,
                            options: vec![option.id],
                        });
                    }
                } else {
                    actions.push(Action::ChooseDecision {
                        decision: *decision,
                        // The neutral default: the first `minimum` options,
                        // which for a may-choose decision means declining.
                        options: pending
                            .options
                            .iter()
                            .take(pending.minimum)
                            .map(|option| option.id)
                            .collect(),
                    });
                }
            }
            _ => actions.push(action.clone()),
        }
    }
    actions
}

/// Serializes one seat's redacted view of the game.
///
/// This is the observation a bot decides from: public zones in full, the
/// opponent's hand as a count, and `legalActions` carrying the indices the
/// bot answers with. `pregame` is true while mulligans are being settled.
/// `actions` is the protocol action list the indices refer to — normally
/// [`protocol_actions`] of the same observation.
#[must_use]
pub fn observation_json(
    catalog: &CardCatalog,
    observation: &PlayerObservation,
    pregame: bool,
    actions: &[Action],
) -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "engineVersion": ENGINE_VERSION,
        "seat": seat_name(observation.viewer),
        "pregame": pregame,
        "turn": observation.turn,
        "activeTurn": observation.active_turn,
        "activeSeat": seat_name(observation.active_player),
        "prioritySeat": seat_name(observation.priority),
        "step": format!("{:?}", observation.step),
        "life": observation.life_totals,
        "manaPools": [
            mana_pool_json(&observation.mana_pools[0]),
            mana_pool_json(&observation.mana_pools[1]),
        ],
        "hand": card_list_json(catalog, &observation.hand),
        "opponentHandSize": observation.opponent_hand_size,
        "librarySizes": observation.library_sizes,
        "graveyards": [
            card_list_json(catalog, &observation.graveyards[0]),
            card_list_json(catalog, &observation.graveyards[1]),
        ],
        "exiles": [
            card_list_json(catalog, &observation.exiles[0]),
            card_list_json(catalog, &observation.exiles[1]),
        ],
        "battlefield": observation.battlefield.iter().map(|permanent| json!({
            "instance": permanent.id.0,
            "definition": permanent.definition.0,
            "name": card_name(catalog, permanent.definition),
            "controller": seat_name(permanent.controller),
            "tapped": permanent.tapped,
            "power": permanent.power,
            "toughness": permanent.toughness,
            "damage": permanent.damage,
            "attacking": permanent.attacking,
            "blocking": permanent.blocking.map(|id| id.0),
            "flying": permanent.flying,
            "canAttack": permanent.can_attack,
            "enteredThisTurn": permanent.entered_this_turn,
        })).collect::<Vec<_>>(),
        "stack": observation.stack.iter().map(|object| json!({
            "stackId": object.id.0,
            "kind": match object.kind {
                StackObjectKind::Spell => "Spell",
                StackObjectKind::ActivatedAbility => "ActivatedAbility",
            },
            "instance": object.card.0,
            "definition": object.definition.0,
            "name": card_name(catalog, object.definition),
            "controller": seat_name(object.controller),
            "targets": object.targets.iter().map(|target| target_json(*target)).collect::<Vec<_>>(),
            "x": object.x,
        })).collect::<Vec<_>>(),
        "decision": observation.decision.as_ref().map(|decision| decision_json(catalog, decision)),
        "result": observation.result.map(result_json),
        "legalActions": actions.iter().enumerate().map(|(index, action)| {
            let mut value = action_json(action);
            if let Value::Object(map) = &mut value {
                map.insert("index".into(), Value::from(index));
            }
            value
        }).collect::<Vec<_>>(),
    })
}

/// Serializes every card definition the engine knows, keyed by definition id.
///
/// Observations refer to cards by `definition`; this is the table those ids
/// index into, fetched once at startup rather than repeated per observation.
#[must_use]
pub fn catalog_json(catalog: &CardCatalog) -> Value {
    let definition_json = |card: &CardDefinition| {
        let behavior = card.behavior;
        let cost = behavior.mana_cost();
        let stats = behavior.creature_stats();
        json!({
            "definition": card.id.0,
            "name": card.name,
            "kind": format!("{:?}", behavior.kind()),
            "isBasicLand": card.is_basic_land,
            "manaCost": {
                "generic": cost.generic,
                "white": cost.white,
                "blue": cost.blue,
                "black": cost.black,
                "red": cost.red,
                "green": cost.green,
                "variableX": cost.variable_x,
            },
            "power": stats.map(|stats| stats.power),
            "toughness": stats.map(|stats| stats.toughness),
            "rulesText": behavior.rules_text(),
            "banned": catalog.is_banned(card.id),
            "restricted": catalog.is_restricted(card.id),
        })
    };
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "engineVersion": ENGINE_VERSION,
        "cards": catalog
            .definitions()
            .into_iter()
            .map(definition_json)
            .collect::<Vec<_>>(),
    })
}

/// Which policy, if any, plays the seat a bot is not driving.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Opponent {
    /// No built-in opponent: the caller drives both seats (self-play).
    External,
    /// The seeded uniform-random baseline.
    Random,
    /// The strongest built-in policy.
    Handcrafted,
}

enum OpponentPolicy {
    External,
    // Send + Sync so a BotGame can move across threads — the Python bindings
    // need it, and parallel rollout collection wants it anyway.
    Scripted(Box<dyn Policy + Send + Sync>),
}

/// A game driven through the bot protocol.
///
/// With a scripted opponent, [`BotGame::act`] plays your action and then lets
/// the opponent play until you have a real choice again, exactly like a
/// hotseat against the built-in bot. With [`Opponent::External`] it stops at
/// every decision, whichever seat owns it, so one loop can drive both sides
/// for self-play.
pub struct BotGame {
    game: Game,
    catalog: CardCatalog,
    opponent_seat: PlayerId,
    opponent: OpponentPolicy,
}

impl BotGame {
    /// Starts a game. `p1_deck`/`p2_deck` name built-in decks; `opponent`
    /// plays `opponent_seat` unless it is [`Opponent::External`].
    ///
    /// # Errors
    ///
    /// Returns a message when a deck name is unknown, the game cannot be
    /// built, or the scripted opponent cannot reach the first decision.
    pub fn new(
        p1_deck: &str,
        p2_deck: &str,
        opponent: Opponent,
        opponent_seat: PlayerId,
        seed: u64,
    ) -> Result<Self, String> {
        let catalog = poc::catalog().map_err(|error| error.to_string())?;
        let deck_one = deck_by_name(p1_deck).ok_or_else(|| format!("unknown deck: {p1_deck}"))?;
        let deck_two = deck_by_name(p2_deck).ok_or_else(|| format!("unknown deck: {p2_deck}"))?;
        let game = Game::new(catalog.clone(), [deck_one, deck_two], seed)
            .map_err(|error| error.to_string())?;
        let opponent = match opponent {
            Opponent::External => OpponentPolicy::External,
            Opponent::Random => {
                OpponentPolicy::Scripted(Box::new(RandomPolicy::new(seed ^ 0x00b0_7b07)))
            }
            Opponent::Handcrafted => {
                OpponentPolicy::Scripted(Box::new(HandcraftedPolicy::new(catalog.clone())))
            }
        };
        let mut bot_game = Self {
            game,
            catalog,
            opponent_seat,
            opponent,
        };
        bot_game.advance()?;
        Ok(bot_game)
    }

    /// Starts a game from a JSON config, the single entry point the FFI and
    /// Python bindings share:
    ///
    /// ```json
    /// {"p1Deck": "Sligh", "p2Deck": "The Deck",
    ///  "opponent": "handcrafted", "opponentSeat": "p2", "seed": 42}
    /// ```
    ///
    /// `opponent` is `"random"`, `"handcrafted"`, or `"external"`;
    /// `opponentSeat` defaults to `"p2"`.
    ///
    /// # Errors
    ///
    /// Returns a message for malformed JSON, unknown deck or opponent names,
    /// or a game that cannot start.
    pub fn from_config_json(config: &str) -> Result<Self, String> {
        let value: Value =
            serde_json::from_str(config).map_err(|error| format!("bad config JSON: {error}"))?;
        let field = |name: &str| -> Result<&str, String> {
            value[name]
                .as_str()
                .ok_or_else(|| format!("config field {name} must be a string"))
        };
        let opponent = match field("opponent").unwrap_or("handcrafted") {
            "external" => Opponent::External,
            "random" => Opponent::Random,
            "handcrafted" => Opponent::Handcrafted,
            other => return Err(format!("unknown opponent: {other}")),
        };
        let opponent_seat = seat_by_name(field("opponentSeat").unwrap_or("p2"))
            .ok_or_else(|| "opponentSeat must be \"p1\" or \"p2\"".to_string())?;
        let seed = value["seed"].as_u64().unwrap_or(0);
        Self::new(
            field("p1Deck")?,
            field("p2Deck")?,
            opponent,
            opponent_seat,
            seed,
        )
    }

    /// The seat that must act next, or `None` when the game is over.
    #[must_use]
    pub fn decision_seat(&self) -> Option<PlayerId> {
        self.game.decision_player()
    }

    /// [`Self::decision_seat`] as a protocol seat name.
    #[must_use]
    pub fn decision_seat_name(&self) -> Option<&'static str> {
        self.decision_seat().map(seat_name)
    }

    /// The observation for one seat as canonical protocol JSON.
    #[must_use]
    pub fn observe_json(&self, seat: PlayerId) -> String {
        let observation = self.game.observe(seat);
        let actions = protocol_actions(&observation);
        observation_json(
            &self.catalog,
            &observation,
            self.game.in_pregame(),
            &actions,
        )
        .to_string()
    }

    /// The number of legal actions for the seat that must act, so FFI
    /// callers can pick an index without parsing JSON.
    #[must_use]
    pub fn legal_action_count(&self) -> usize {
        self.decision_seat()
            .map_or(0, |seat| protocol_actions(&self.game.observe(seat)).len())
    }

    /// The finished game's result, if any, as protocol JSON.
    #[must_use]
    pub fn result(&self) -> Option<GameResult> {
        self.game.observe(PlayerId::One).result
    }

    /// Plays the given index from the acting seat's `legalActions`, then lets
    /// a scripted opponent play until the driven seat has a real choice.
    ///
    /// # Errors
    ///
    /// Returns a message when the game is over, the index is out of range, or
    /// the engine rejects the action.
    pub fn act(&mut self, action_index: usize) -> Result<(), String> {
        let seat = self.decision_seat().ok_or("the game is over")?;
        let actions = protocol_actions(&self.game.observe(seat));
        let action = actions.get(action_index).cloned().ok_or_else(|| {
            format!(
                "action index {action_index} out of range ({} legal actions)",
                actions.len()
            )
        })?;
        self.game
            .apply(seat, action)
            .map_err(|error| error.to_string())?;
        self.advance()
    }

    /// Answers the pending decision with an explicit set of option ids, for
    /// multi-pick decisions where the default expansion is not what you want.
    /// The observation's `decision` object lists the options and bounds.
    ///
    /// # Errors
    ///
    /// Returns a message when no decision is pending or the engine rejects
    /// the selection.
    pub fn choose_decision(&mut self, option_ids: &[u32]) -> Result<(), String> {
        let seat = self.decision_seat().ok_or("the game is over")?;
        let observation = self.game.observe(seat);
        let decision = observation
            .decision
            .as_ref()
            .ok_or("no decision is pending")?;
        self.game
            .apply(
                seat,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: option_ids.to_vec(),
                },
            )
            .map_err(|error| error.to_string())?;
        self.advance()
    }

    /// Runs the scripted opponent until the driven seat must decide or the
    /// game ends. A no-op with an external opponent.
    fn advance(&mut self) -> Result<(), String> {
        let OpponentPolicy::Scripted(policy) = &mut self.opponent else {
            return Ok(());
        };
        for _ in 0..ACTION_LIMIT {
            let Some(player) = self.game.decision_player() else {
                return Ok(());
            };
            if player != self.opponent_seat {
                return Ok(());
            }
            let observation = self.game.observe(player);
            let action = policy
                .choose_action(&observation)
                .ok_or("the scripted opponent returned no action")?;
            self.game
                .apply(player, action)
                .map_err(|error| error.to_string())?;
        }
        Err("the game exceeded its action limit".to_string())
    }

    /// The catalog as protocol JSON.
    #[must_use]
    pub fn catalog_json(&self) -> String {
        catalog_json(&self.catalog).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finish(mut game: BotGame, mut pick: impl FnMut(usize, &Value) -> usize) -> GameResult {
        for turn in 0..ACTION_LIMIT {
            if let Some(result) = game.result() {
                return result;
            }
            let seat = game.decision_seat().expect("no result means a decision");
            let observation: Value =
                serde_json::from_str(&game.observe_json(seat)).expect("valid JSON");
            let count = observation["legalActions"]
                .as_array()
                .expect("legalActions is an array")
                .len();
            assert!(count > 0, "a decision always has options");
            game.act(pick(turn, &observation))
                .expect("chosen index is legal");
        }
        panic!("game did not finish");
    }

    /// A do-nothing bot written the way the docs tell people to write one:
    /// read `legalActions`, prefer the quiet options by their `type` tags.
    fn pass_bot(observation: &Value) -> usize {
        let actions = observation["legalActions"].as_array().expect("array");
        for preferred in [
            "KeepHand",
            "ChooseDecision",
            "PassPriority",
            "FinishDeclaringAttackers",
            "FinishDeclaringBlockers",
            "AssignCombatDamage",
            "DiscardCards",
            "BottomCards",
            "ChooseUntap",
        ] {
            if let Some(action) = actions.iter().find(|action| action["type"] == preferred) {
                return usize::try_from(action["index"].as_u64().expect("index"))
                    .expect("index fits");
            }
        }
        0
    }

    #[test]
    fn a_scripted_game_runs_to_a_result_through_json_alone() {
        let game = BotGame::new("Sligh", "The Deck", Opponent::Handcrafted, PlayerId::Two, 7)
            .expect("game starts");
        let result = finish(game, |_, observation| pass_bot(observation));
        assert!(matches!(
            result,
            GameResult::Winner { .. } | GameResult::Draw
        ));
    }

    #[test]
    fn an_external_game_lets_one_loop_drive_both_seats() {
        let game = BotGame::new("Goblins", "Sligh", Opponent::External, PlayerId::Two, 11)
            .expect("game starts");
        let result = finish(game, |_, observation| pass_bot(observation));
        assert!(matches!(
            result,
            GameResult::Winner { .. } | GameResult::Draw
        ));
    }

    #[test]
    fn the_same_seed_produces_the_same_bytes() {
        let make = || {
            BotGame::new("Sligh", "Goblins", Opponent::Random, PlayerId::Two, 99)
                .expect("game starts")
        };
        let (mut first, mut second) = (make(), make());
        for _ in 0..40 {
            if first.result().is_some() {
                break;
            }
            let seat = first.decision_seat().expect("still running");
            assert_eq!(first.observe_json(seat), second.observe_json(seat));
            first.act(0).expect("index 0 is legal");
            second.act(0).expect("index 0 is legal");
        }
    }

    #[test]
    fn observations_carry_indexed_legal_actions_and_no_hidden_cards() {
        let game = BotGame::new("Sligh", "The Deck", Opponent::Handcrafted, PlayerId::Two, 3)
            .expect("game starts");
        let seat = game.decision_seat().expect("mulligan decision");
        let observation: Value =
            serde_json::from_str(&game.observe_json(seat)).expect("valid JSON");
        assert_eq!(observation["seat"], "p1");
        assert_eq!(observation["pregame"], true);
        let actions = observation["legalActions"].as_array().expect("array");
        for (index, action) in actions.iter().enumerate() {
            assert_eq!(action["index"], index, "indices match positions");
            assert!(action["type"].is_string(), "every action is tagged");
        }
        // The opponent's hand is a count, never a list of cards.
        assert!(observation["opponentHandSize"].is_u64());
        assert_eq!(observation["hand"].as_array().expect("hand").len(), 7);
    }

    #[test]
    fn decisions_reach_bots_as_concrete_indexed_actions() {
        // The engine's decision template has empty options; the protocol
        // must never show that to a bot. Play games until decisions appear
        // and check every ChooseDecision carries a concrete selection.
        // A bot that plays lands and casts spells, so card effects (Chain
        // Lightning copies, discard effects) actually raise decisions.
        let cast_bot = |observation: &Value| {
            let actions = observation["legalActions"].as_array().expect("array");
            for preferred in ["PlayLand", "CastSpell"] {
                if let Some(action) = actions.iter().find(|action| action["type"] == preferred) {
                    return usize::try_from(action["index"].as_u64().expect("index"))
                        .expect("index fits");
                }
            }
            pass_bot(observation)
        };
        let mut saw_decision = false;
        for seed in 0..20 {
            let mut game = BotGame::new(
                "Sligh",
                "The Deck",
                Opponent::Handcrafted,
                PlayerId::Two,
                seed,
            )
            .expect("game starts");
            for _ in 0..2_000 {
                if game.result().is_some() {
                    break;
                }
                let seat = game.decision_seat().expect("still running");
                let observation: Value =
                    serde_json::from_str(&game.observe_json(seat)).expect("valid JSON");
                for action in observation["legalActions"].as_array().expect("array") {
                    if action["type"] == "ChooseDecision" {
                        saw_decision = true;
                        assert!(
                            !observation["decision"].is_null(),
                            "a decision action implies a decision object",
                        );
                        let minimum = observation["decision"]["minimum"].as_u64().expect("min");
                        let chosen = action["options"].as_array().expect("options").len();
                        assert!(
                            u64::try_from(chosen).expect("fits") >= minimum,
                            "every offered selection is submittable as-is",
                        );
                    }
                }
                game.act(cast_bot(&observation)).expect("legal index");
            }
            if saw_decision {
                break;
            }
        }
        assert!(saw_decision, "the seeded games reached a decision");
    }

    #[test]
    fn the_catalog_lists_every_card_with_names_and_costs() {
        let catalog = poc::catalog().expect("catalog builds");
        let value = catalog_json(&catalog);
        let cards = value["cards"].as_array().expect("cards array");
        assert!(cards.len() > 100, "the pool is substantial");
        assert!(cards.iter().all(|card| card["name"].is_string()));
        assert!(
            cards
                .iter()
                .any(|card| card["name"] == "Lightning Bolt" && card["manaCost"]["red"] == 1)
        );
    }

    #[test]
    fn deck_names_all_resolve() {
        for name in deck_names() {
            assert!(deck_by_name(name).is_some(), "{name} resolves");
        }
        assert!(deck_by_name("Not A Deck").is_none());
    }
}
