use std::error::Error;
use std::fmt;

use crate::action::{Action, ActionError, CombatDamageAssignment, ManaColor, Target};
use crate::card::{CardBehavior, CardCatalog, CardKind, CardSet, ManaCost};
use crate::deck::{Deck, DeckError, ValidatedDeck};
use crate::ids::{CardDefinitionId, CardInstanceId, PlayerId, StackObjectId};
use crate::rng::ReplayRng;
use crate::rules;

#[derive(Clone, Debug, Eq, PartialEq)]
struct CardInstance {
    id: CardInstanceId,
    definition: CardDefinitionId,
    owner: PlayerId,
}

type PublicCard = (CardInstanceId, CardDefinitionId);
type LastSeenHand = Option<(PlayerId, Vec<PublicCard>)>;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DecisionVisibility {
    Public,
    Private,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DecisionPreference {
    HigherCardValue,
    LowerCardValue,
    Neutral,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DecisionZone {
    Hand,
    Graveyard,
    Battlefield,
    Library,
    DrawnThisStep,
    None,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionOption {
    pub id: u32,
    pub label: String,
    pub card: Option<(CardInstanceId, CardDefinitionId)>,
    pub zone: DecisionZone,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionObservation {
    pub id: u32,
    pub player: PlayerId,
    pub prompt: String,
    pub visibility: DecisionVisibility,
    pub preference: DecisionPreference,
    pub minimum: usize,
    pub maximum: usize,
    pub cancellable: bool,
    pub options: Vec<DecisionOption>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
struct Permanent {
    card: CardInstance,
    controller: PlayerId,
    tapped: bool,
    entered_turn: u32,
    damage: u16,
    power_bonus: i16,
    toughness_bonus: i16,
    attacking: bool,
    blocking: Option<CardInstanceId>,
    chosen_player: Option<PlayerId>,
    destroy_at_end: bool,
    flying_until_end: bool,
    factory_animated: bool,
    dragon_whelp_activations: u8,
    plus_one_counters: u16,
    combat_damage_assignment: Vec<CombatDamageAssignment>,
    /// A Copy Artifact remembers the printed behavior it copied when it
    /// entered.  Keeping this on the permanent lets all of the normal rules
    /// (mana, type checks, abilities, and continuous effects) see the copy as
    /// the copied card rather than as the enchantment it started as.
    copied_behavior: Option<CardBehavior>,
    regeneration_shields: u8,
    trample_until_end: bool,
    berserked: bool,
    attacked_this_turn: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StackObject {
    id: StackObjectId,
    kind: StackObjectKind,
    card: CardInstance,
    controller: PlayerId,
    targets: Vec<Target>,
    chosen_permanents: Vec<CardInstanceId>,
    x: u16,
    is_copy: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlayerState {
    life: i16,
    library: Vec<CardInstance>,
    hand: Vec<CardInstance>,
    graveyard: Vec<CardInstance>,
    exile: Vec<CardInstance>,
    mana_pool: ManaPool,
    land_played_this_turn: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Pregame {
    Mulligan(PlayerId),
    Bottom(PlayerId),
}

#[derive(Clone, Debug)]
struct PendingDecision {
    observation: DecisionObservation,
    continuation: DecisionContinuation,
}

#[derive(Clone, Copy, Debug)]
enum BalanceAction {
    Sacrifice,
    Discard,
}

#[derive(Clone, Debug)]
struct BalanceTask {
    player: PlayerId,
    prompt: String,
    zone: DecisionZone,
    cards: Vec<CardInstance>,
    count: usize,
    action: BalanceAction,
}

#[derive(Clone, Debug)]
enum DecisionContinuation {
    Tutor,
    IronStar {
        player: PlayerId,
    },
    ChainLightning {
        player: PlayerId,
        spell: StackObject,
        targets: Vec<Target>,
    },
    Fork {
        player: PlayerId,
        spell: StackObject,
        target_lists: Vec<Vec<Target>>,
    },
    ManaVault {
        player: PlayerId,
        permanent: CardInstanceId,
    },
    RecallCost {
        player: PlayerId,
        card: CardInstanceId,
        targets: Vec<Target>,
        x: u16,
    },
    RecallReturn {
        player: PlayerId,
    },
    Balance {
        task: BalanceTask,
        remaining: Vec<BalanceTask>,
    },
    TimeVault {
        permanent: CardInstanceId,
        remaining: Vec<CardInstanceId>,
    },
    SylvanSelect {
        player: PlayerId,
        candidates: Vec<CardInstanceId>,
        choices_left: usize,
    },
    SylvanMode {
        player: PlayerId,
        card: CardInstanceId,
        candidates: Vec<CardInstanceId>,
        choices_left: usize,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ManaPool {
    pub white: u16,
    pub blue: u16,
    pub black: u16,
    pub red: u16,
    pub green: u16,
    pub colorless: u16,
}

impl ManaPool {
    #[must_use]
    pub const fn total(self) -> u16 {
        self.white + self.blue + self.black + self.red + self.green + self.colorless
    }

    fn add(&mut self, other: Self) {
        self.white = self.white.saturating_add(other.white);
        self.blue = self.blue.saturating_add(other.blue);
        self.black = self.black.saturating_add(other.black);
        self.red = self.red.saturating_add(other.red);
        self.green = self.green.saturating_add(other.green);
        self.colorless = self.colorless.saturating_add(other.colorless);
    }

    const fn amount(self, color: ManaColor) -> u16 {
        match color {
            ManaColor::White => self.white,
            ManaColor::Blue => self.blue,
            ManaColor::Black => self.black,
            ManaColor::Red => self.red,
            ManaColor::Green => self.green,
            ManaColor::Colorless => self.colorless,
        }
    }

    fn add_color(&mut self, color: ManaColor, amount: u16) {
        match color {
            ManaColor::White => self.white = self.white.saturating_add(amount),
            ManaColor::Blue => self.blue = self.blue.saturating_add(amount),
            ManaColor::Black => self.black = self.black.saturating_add(amount),
            ManaColor::Red => self.red = self.red.saturating_add(amount),
            ManaColor::Green => self.green = self.green.saturating_add(amount),
            ManaColor::Colorless => self.colorless = self.colorless.saturating_add(amount),
        }
    }

    fn remove_color(&mut self, color: ManaColor, amount: u16) {
        match color {
            ManaColor::White => self.white -= amount,
            ManaColor::Blue => self.blue -= amount,
            ManaColor::Black => self.black -= amount,
            ManaColor::Red => self.red -= amount,
            ManaColor::Green => self.green -= amount,
            ManaColor::Colorless => self.colorless -= amount,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Step {
    Upkeep,
    Draw,
    PrecombatMain,
    BeginningOfCombat,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    EndOfCombat,
    PostcombatMain,
    End,
    Cleanup,
}

impl Step {
    const fn is_main(self) -> bool {
        matches!(self, Self::PrecombatMain | Self::PostcombatMain)
    }

    const fn ends_phase(self) -> bool {
        matches!(
            self,
            Self::Draw
                | Self::PrecombatMain
                | Self::EndOfCombat
                | Self::PostcombatMain
                | Self::Cleanup
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameResult {
    Winner { winner: PlayerId, reason: WinReason },
    Draw,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WinReason {
    OpponentConceded,
    OpponentLostAllLife,
    OpponentTriedToDrawFromEmptyLibrary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StackObjectKind {
    Spell,
    ActivatedAbility,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameEvent {
    GameStarted {
        seed: u64,
    },
    CardDrawn {
        player: PlayerId,
        card: CardInstanceId,
    },
    LandPlayed {
        player: PlayerId,
        card: CardInstanceId,
    },
    ManaAdded {
        player: PlayerId,
        source: CardInstanceId,
    },
    SpellCast {
        player: PlayerId,
        card: CardInstanceId,
        targets: Vec<Target>,
    },
    SpellResolved {
        card: CardInstanceId,
    },
    AbilityActivated {
        player: PlayerId,
        source: CardInstanceId,
        chosen_permanents: Vec<CardInstanceId>,
    },
    AbilityResolved {
        source: CardInstanceId,
    },
    AttackDeclared {
        player: PlayerId,
        attackers: Vec<CardInstanceId>,
    },
    BlockDeclared {
        player: PlayerId,
        assignments: Vec<(CardInstanceId, CardInstanceId)>,
    },
    DamageDealt {
        player: PlayerId,
        amount: u16,
    },
    ManaBurn {
        player: PlayerId,
        amount: u16,
    },
    StepChanged {
        turn: u32,
        active_player: PlayerId,
        step: Step,
    },
    GameEnded {
        result: GameResult,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PermanentObservation {
    pub id: CardInstanceId,
    pub definition: CardDefinitionId,
    pub controller: PlayerId,
    pub tapped: bool,
    pub power: Option<i16>,
    pub toughness: Option<i16>,
    pub damage: u16,
    pub attacking: bool,
    pub blocking: Option<CardInstanceId>,
    pub flying: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StackObservation {
    pub id: StackObjectId,
    pub kind: StackObjectKind,
    pub card: CardInstanceId,
    pub definition: CardDefinitionId,
    pub controller: PlayerId,
    pub targets: Vec<Target>,
    pub chosen_permanents: Vec<CardInstanceId>,
    pub x: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerObservation {
    pub viewer: PlayerId,
    pub turn: u32,
    /// The number of turns the active player has started, including extras.
    pub active_turn: u32,
    pub active_player: PlayerId,
    pub priority: PlayerId,
    pub step: Step,
    pub life_totals: [i16; 2],
    pub mana_pools: [ManaPool; 2],
    pub hand: Vec<(CardInstanceId, CardDefinitionId)>,
    pub opponent_hand_size: usize,
    pub last_seen_hand: LastSeenHand,
    pub library_sizes: [usize; 2],
    pub graveyards: [Vec<(CardInstanceId, CardDefinitionId)>; 2],
    pub exiles: [Vec<(CardInstanceId, CardDefinitionId)>; 2],
    pub battlefield: Vec<PermanentObservation>,
    pub stack: Vec<StackObservation>,
    pub decision: Option<DecisionObservation>,
    pub result: Option<GameResult>,
    pub legal_actions: Vec<Action>,
}

#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct Game {
    seed: u64,
    rng: ReplayRng,
    catalog: CardCatalog,
    players: [PlayerState; 2],
    battlefield: Vec<Permanent>,
    stack: Vec<StackObject>,
    next_stack_id: u32,
    turn: u32,
    turns_started: [u32; 2],
    active_player: PlayerId,
    priority: PlayerId,
    consecutive_passes: u8,
    step: Step,
    attackers_declared: bool,
    blockers_declared: bool,
    untap_pending: bool,
    pregame: Option<Pregame>,
    mulligans: [u8; 2],
    cleanup_pending: bool,
    pending_decisions: Vec<PendingDecision>,
    next_decision_id: u32,
    last_seen_hands: [LastSeenHand; 2],
    pending_combat_attackers: Vec<CardInstanceId>,
    extra_turns: Vec<PlayerId>,
    mana_drain_pending: [u16; 2],
    channel_active: [bool; 2],
    skipped_turns: [u16; 2],
    result: Option<GameResult>,
    events: Vec<GameEvent>,
}

impl Game {
    /// Creates a game, shuffles both decks, and draws opening hands.
    ///
    /// Player one takes the first turn and skips that turn's draw. Mulligans
    /// are not yet part of this constructor.
    ///
    /// # Errors
    ///
    /// Returns [`GameError`] if a deck references a card absent from the
    /// supplied catalog, card instance IDs are exhausted, or a deck cannot
    /// supply an opening hand.
    pub fn new(catalog: CardCatalog, decks: [Deck; 2], seed: u64) -> Result<Self, GameError> {
        let mut rng = ReplayRng::new(seed);
        let mut next_instance_id = 0_u32;
        let [deck_one, deck_two] = decks;
        let deck_one = deck_one
            .validate(&catalog)
            .map_err(|error| GameError::InvalidDeck {
                player: PlayerId::One,
                error,
            })?;
        let deck_two = deck_two
            .validate(&catalog)
            .map_err(|error| GameError::InvalidDeck {
                player: PlayerId::Two,
                error,
            })?;

        let mut build_player =
            |player: PlayerId, deck: ValidatedDeck| -> Result<PlayerState, GameError> {
                let definitions = deck.into_main();
                let mut library = Vec::with_capacity(definitions.len());
                for definition in definitions {
                    let id = CardInstanceId(next_instance_id);
                    next_instance_id = next_instance_id
                        .checked_add(1)
                        .ok_or(GameError::TooManyCards)?;
                    library.push(CardInstance {
                        id,
                        definition,
                        owner: player,
                    });
                }
                rng.shuffle(&mut library);
                let hand = draw_opening_hand(&mut library)?;
                Ok(PlayerState {
                    life: i16::from(rules::STARTING_LIFE),
                    library,
                    hand,
                    graveyard: Vec::new(),
                    exile: Vec::new(),
                    mana_pool: ManaPool::default(),
                    land_played_this_turn: false,
                })
            };

        let players = [
            build_player(PlayerId::One, deck_one)?,
            build_player(PlayerId::Two, deck_two)?,
        ];

        Ok(Self {
            seed,
            rng,
            catalog,
            players,
            battlefield: Vec::new(),
            stack: Vec::new(),
            next_stack_id: 0,
            turn: 1,
            turns_started: [1, 0],
            active_player: PlayerId::One,
            priority: PlayerId::One,
            consecutive_passes: 0,
            step: Step::Upkeep,
            attackers_declared: false,
            blockers_declared: false,
            untap_pending: false,
            pregame: Some(Pregame::Mulligan(PlayerId::One)),
            mulligans: [0, 0],
            cleanup_pending: false,
            pending_decisions: Vec::new(),
            next_decision_id: 0,
            last_seen_hands: [None, None],
            pending_combat_attackers: Vec::new(),
            extra_turns: Vec::new(),
            mana_drain_pending: [0, 0],
            channel_active: [false, false],
            skipped_turns: [0, 0],
            result: None,
            events: vec![GameEvent::GameStarted { seed }],
        })
    }

    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    #[must_use]
    pub const fn result(&self) -> Option<GameResult> {
        self.result
    }

    /// Returns the player expected to make the engine's next decision.
    ///
    /// This may differ from the player with priority during pregame choices,
    /// turn-based actions such as declaring blockers, and other mandatory
    /// choices. Bot runners should observe this player and submit one of that
    /// observation's legal actions.
    #[must_use]
    pub fn decision_player(&self) -> Option<PlayerId> {
        if self.result.is_some() {
            return None;
        }
        if let Some(decision) = self.pending_decisions.first() {
            return Some(decision.observation.player);
        }
        if !self.pending_combat_attackers.is_empty() {
            return Some(self.active_player);
        }
        if let Some(pregame) = self.pregame {
            return Some(match pregame {
                Pregame::Mulligan(player) | Pregame::Bottom(player) => player,
            });
        }
        if self.cleanup_pending || self.untap_pending {
            return Some(self.active_player);
        }
        if self.step == Step::DeclareAttackers && !self.attackers_declared {
            return Some(self.active_player);
        }
        if self.step == Step::DeclareBlockers && !self.blockers_declared {
            return Some(self.active_player.opponent());
        }
        Some(self.priority)
    }

    #[must_use]
    /// Returns the omniscient event trace.
    ///
    /// This is intended for replays and debugging. Give bots
    /// [`PlayerObservation`] rather than this event stream.
    pub fn events(&self) -> &[GameEvent] {
        &self.events
    }

    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn legal_actions(&self, player: PlayerId) -> Vec<Action> {
        if self.result.is_some() {
            return Vec::new();
        }

        let mut actions = vec![Action::Concede];
        if let Some(decision) = self.pending_decisions.first() {
            if decision.observation.player == player {
                // Bounded selections are represented by the decision observation rather
                // than by an eagerly-expanded Cartesian product. Callers submit the
                // selected option IDs through `ChooseDecision`; `apply` validates the
                // selection directly against this schema.
                actions.push(Action::ChooseDecision {
                    decision: decision.observation.id,
                    options: Vec::new(),
                });
                if decision.observation.cancellable {
                    actions.push(Action::CancelDecision {
                        decision: decision.observation.id,
                    });
                }
            }
            return actions;
        }
        if let Some(attacker) = self.pending_combat_attackers.first().copied() {
            if player == self.active_player {
                actions.extend(self.combat_assignment_actions(attacker));
            }
            return actions;
        }
        if let Some(pregame) = self.pregame {
            match pregame {
                Pregame::Mulligan(deciding) if player == deciding => {
                    actions.push(Action::KeepHand);
                    actions.push(Action::TakeMulligan);
                }
                Pregame::Bottom(deciding) if player == deciding => {
                    let count = usize::from(self.mulligans[player.index()])
                        .min(self.players[player.index()].hand.len());
                    actions.extend(
                        combinations(
                            &self.players[player.index()]
                                .hand
                                .iter()
                                .map(|card| card.id)
                                .collect::<Vec<_>>(),
                            count,
                        )
                        .into_iter()
                        .map(|cards| Action::BottomCards { cards }),
                    );
                }
                Pregame::Mulligan(_) | Pregame::Bottom(_) => {}
            }
            return actions;
        }
        if self.cleanup_pending {
            if player == self.active_player {
                let state = &self.players[player.index()];
                let count = state.hand.len().saturating_sub(7);
                actions.extend(
                    combinations(
                        &state.hand.iter().map(|card| card.id).collect::<Vec<_>>(),
                        count,
                    )
                    .into_iter()
                    .map(|cards| Action::DiscardCards { cards }),
                );
            }
            return actions;
        }
        if self.untap_pending {
            if player == self.active_player {
                actions.extend(self.untap_actions(player));
            }
            return actions;
        }
        if self.step == Step::DeclareAttackers && !self.attackers_declared {
            if player == self.active_player {
                let juggernaut_must_attack = self.battlefield.iter().any(|permanent| {
                    permanent.controller == player
                        && !permanent.tapped
                        && !permanent.attacking
                        && self.can_attack(permanent)
                        && self.effective_behavior(permanent) == Some(CardBehavior::Juggernaut)
                });
                if !juggernaut_must_attack {
                    actions.push(Action::FinishDeclaringAttackers);
                }
                actions.extend(self.attacker_actions(player));
            }
            return actions;
        }
        if self.step == Step::DeclareBlockers && !self.blockers_declared {
            if player == self.active_player.opponent() {
                actions.push(Action::FinishDeclaringBlockers);
                actions.extend(self.blocker_actions(player));
            }
            return actions;
        }
        if player != self.priority {
            return actions;
        }

        actions.push(Action::PassPriority);
        self.add_mana_actions(player, &mut actions);
        if self.channel_active[player.index()] && self.players[player.index()].life > 1 {
            actions.push(Action::PayLifeForMana);
        }
        self.add_land_actions(player, &mut actions);
        self.add_spell_actions(player, &mut actions);
        self.add_ability_actions(player, &mut actions);
        actions
    }

    /// Applies one engine-enumerated action for a player.
    ///
    /// # Errors
    ///
    /// Returns [`ActionError`] when the game is over or the action is not
    /// currently legal for that player.
    pub fn apply(&mut self, player: PlayerId, action: Action) -> Result<(), ActionError> {
        if self.result.is_some() {
            return Err(ActionError::GameAlreadyFinished);
        }
        if !self.is_legal_action(player, &action) {
            return Err(ActionError::NotLegal { player, action });
        }

        match action {
            Action::KeepHand => self.keep_hand(player),
            Action::TakeMulligan => self.take_mulligan(player),
            Action::BottomCards { cards } => self.bottom_cards(player, &cards),
            Action::DiscardCards { cards } => self.discard_cards(player, &cards),
            Action::ChooseDecision { decision, options } => {
                self.choose_decision(player, decision, &options);
            }
            Action::CancelDecision { decision } => self.cancel_decision(decision),
            Action::ChooseUntap { permanents } => self.choose_untap(player, &permanents),
            Action::PassPriority => self.pass_priority(player),
            Action::PlayLand { card } => self.play_land(player, card),
            Action::ActivateManaAbility { source, color } => {
                self.activate_mana_source(player, source, color);
            }
            Action::PayLifeForMana => {
                self.players[player.index()].life -= 1;
                self.players[player.index()]
                    .mana_pool
                    .add_color(ManaColor::Colorless, 1);
                self.consecutive_passes = 0;
            }
            Action::CastSpell {
                card,
                targets,
                sacrifices,
                x,
            } => self.cast_spell(player, card, targets, &sacrifices, x),
            Action::ActivateAbility {
                source,
                target,
                sacrifice,
            } => self.activate_ability(player, source, target, sacrifice),
            Action::DeclareAttacker { attacker } => self.declare_attacker(attacker),
            Action::FinishDeclaringAttackers => self.finish_declaring_attackers(),
            Action::DeclareBlocker { blocker, attacker } => {
                self.declare_blocker(blocker, attacker);
            }
            Action::FinishDeclaringBlockers => self.finish_declaring_blockers(),
            Action::AssignCombatDamage {
                attacker,
                assignments,
            } => self.assign_combat_damage(attacker, assignments),
            Action::Concede => self.finish(GameResult::Winner {
                winner: player.opponent(),
                reason: WinReason::OpponentConceded,
            }),
        }
        Ok(())
    }

    /// Validates an action against the current state without mutating the game.
    ///
    /// Unlike [`legal_actions`], this also validates the option IDs supplied to
    /// a bounded [`Action::ChooseDecision`] selection without expanding every
    /// possible combination into a vector.
    #[must_use]
    pub fn is_legal_action(&self, player: PlayerId, action: &Action) -> bool {
        if let Action::ChooseDecision { decision, options } = action {
            let Some(pending) = self.pending_decisions.first() else {
                return false;
            };
            let observation = &pending.observation;
            if observation.player != player || observation.id != *decision {
                return false;
            }
            let available = observation
                .options
                .iter()
                .map(|option| option.id)
                .collect::<std::collections::HashSet<_>>();
            let unique = options
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>();
            options.len() == unique.len()
                && options.len() >= observation.minimum
                && options.len() <= observation.maximum
                && options.iter().all(|option| available.contains(option))
        } else {
            self.legal_actions(player).contains(action)
        }
    }

    #[must_use]
    pub fn observe(&self, viewer: PlayerId) -> PlayerObservation {
        let player = &self.players[viewer.index()];
        let opponent = &self.players[viewer.opponent().index()];
        PlayerObservation {
            viewer,
            turn: self.turn,
            active_turn: self.turns_started[self.active_player.index()],
            active_player: self.active_player,
            priority: self.priority,
            step: self.step,
            life_totals: [self.players[0].life, self.players[1].life],
            mana_pools: [self.players[0].mana_pool, self.players[1].mana_pool],
            hand: player
                .hand
                .iter()
                .map(|card| (card.id, card.definition))
                .collect(),
            opponent_hand_size: opponent.hand.len(),
            last_seen_hand: self.last_seen_hands[viewer.index()].clone(),
            library_sizes: [self.players[0].library.len(), self.players[1].library.len()],
            graveyards: [
                public_cards(&self.players[0].graveyard),
                public_cards(&self.players[1].graveyard),
            ],
            exiles: [
                public_cards(&self.players[0].exile),
                public_cards(&self.players[1].exile),
            ],
            battlefield: self
                .battlefield
                .iter()
                .map(|permanent| PermanentObservation {
                    id: permanent.card.id,
                    definition: permanent.card.definition,
                    controller: permanent.controller,
                    tapped: permanent.tapped,
                    power: self.power(permanent),
                    toughness: self.toughness(permanent),
                    damage: permanent.damage,
                    attacking: permanent.attacking,
                    blocking: permanent.blocking,
                    flying: self.has_flying(permanent),
                })
                .collect(),
            stack: self
                .stack
                .iter()
                .map(|object| StackObservation {
                    id: object.id,
                    kind: object.kind,
                    card: object.card.id,
                    definition: object.card.definition,
                    controller: object.controller,
                    targets: object.targets.clone(),
                    chosen_permanents: object.chosen_permanents.clone(),
                    x: object.x,
                })
                .collect(),
            decision: self.pending_decisions.first().and_then(|decision| {
                (decision.observation.visibility == DecisionVisibility::Public
                    || decision.observation.player == viewer)
                    .then(|| decision.observation.clone())
            }),
            result: self.result,
            legal_actions: self.legal_actions(viewer),
        }
    }

    fn add_mana_actions(&self, player: PlayerId, actions: &mut Vec<Action>) {
        for permanent in self.battlefield.iter().filter(|permanent| {
            permanent.controller == player
                && !permanent.tapped
                && self.can_use_tap_ability(permanent)
        }) {
            actions.extend(self.mana_colors(permanent).into_iter().map(|color| {
                Action::ActivateManaAbility {
                    source: permanent.card.id,
                    color,
                }
            }));
        }
    }

    fn keep_hand(&mut self, player: PlayerId) {
        if self.mulligans[player.index()] > 0 {
            self.pregame = Some(Pregame::Bottom(player));
        } else {
            self.advance_pregame(player);
        }
    }

    fn take_mulligan(&mut self, player: PlayerId) {
        let state = &mut self.players[player.index()];
        state.library.append(&mut state.hand);
        self.rng.shuffle(&mut state.library);
        state.hand = draw_opening_hand(&mut state.library)
            .expect("a validated deck always contains at least seven cards");
        self.mulligans[player.index()] = self.mulligans[player.index()].saturating_add(1);
    }

    fn bottom_cards(&mut self, player: PlayerId, cards: &[CardInstanceId]) {
        for id in cards.iter().rev() {
            if let Some(card) = remove_card(&mut self.players[player.index()].hand, *id) {
                self.players[player.index()].library.insert(0, card);
            }
        }
        self.advance_pregame(player);
    }

    fn advance_pregame(&mut self, player: PlayerId) {
        if player == PlayerId::One {
            self.pregame = Some(Pregame::Mulligan(PlayerId::Two));
            self.priority = PlayerId::Two;
        } else {
            self.pregame = None;
            self.priority = PlayerId::One;
        }
    }

    fn discard_cards(&mut self, player: PlayerId, cards: &[CardInstanceId]) {
        for id in cards {
            if let Some(card) = remove_card(&mut self.players[player.index()].hand, *id) {
                self.players[player.index()].graveyard.push(card);
            }
        }
        self.cleanup_pending = false;
        self.complete_cleanup();
        if self.result.is_none() {
            self.priority = self.active_player;
            self.events.push(GameEvent::StepChanged {
                turn: self.turn,
                active_player: self.active_player,
                step: self.step,
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn queue_decision(
        &mut self,
        player: PlayerId,
        prompt: impl Into<String>,
        visibility: DecisionVisibility,
        preference: DecisionPreference,
        bounds: std::ops::RangeInclusive<usize>,
        cancellable: bool,
        options: Vec<DecisionOption>,
        continuation: DecisionContinuation,
    ) {
        let id = self.next_decision_id;
        self.next_decision_id = self.next_decision_id.saturating_add(1);
        self.pending_decisions.push(PendingDecision {
            observation: DecisionObservation {
                id,
                player,
                prompt: prompt.into(),
                visibility,
                preference,
                minimum: *bounds.start(),
                maximum: *bounds.end(),
                cancellable,
                options,
            },
            continuation,
        });
    }

    fn queue_iron_star_decision(&mut self, player: PlayerId) {
        let mut options = vec![DecisionOption {
            id: 0,
            label: "Don't use Iron Star".into(),
            card: None,
            zone: DecisionZone::None,
        }];
        if self.can_pay_cost(player, ManaCost::new(1, 0), 0) {
            options.push(DecisionOption {
                id: 1,
                label: "Pay 1 to gain 1 life with Iron Star".into(),
                card: None,
                zone: DecisionZone::None,
            });
        }
        self.queue_decision(
            player,
            "Use Iron Star?",
            DecisionVisibility::Private,
            DecisionPreference::Neutral,
            1..=1,
            false,
            options,
            DecisionContinuation::IronStar { player },
        );
    }

    fn target_label(&self, viewer: PlayerId, target: Target) -> String {
        match target {
            Target::Player(player) if player == viewer => "you".into(),
            Target::Player(_) => "your opponent".into(),
            Target::Permanent(id) => self
                .battlefield
                .iter()
                .find(|permanent| permanent.card.id == id)
                .and_then(|permanent| self.catalog.get(permanent.card.definition))
                .map_or_else(|| "that permanent".into(), |card| card.name.clone()),
            Target::Spell(id) => self
                .stack
                .iter()
                .find(|object| object.id == id)
                .and_then(|object| self.catalog.get(object.card.definition))
                .map_or_else(|| "that spell".into(), |card| card.name.clone()),
        }
    }

    fn queue_chain_lightning_decision(&mut self, player: PlayerId, spell: StackObject) {
        let mut targets = self.damage_targets();
        if let Some(target) = spell.targets.first()
            && !targets.contains(target)
        {
            targets.push(*target);
        }
        let mut options = vec![DecisionOption {
            id: 0,
            label: "Don't copy Chain Lightning".into(),
            card: None,
            zone: DecisionZone::None,
        }];
        if self.can_pay_cost(player, ManaCost::new(0, 2), 0) {
            options.extend(
                targets
                    .iter()
                    .enumerate()
                    .map(|(index, target)| DecisionOption {
                        id: u32::try_from(index + 1).unwrap_or(u32::MAX),
                        label: format!(
                            "Copy Chain Lightning → {}",
                            self.target_label(player, *target)
                        ),
                        card: None,
                        zone: DecisionZone::None,
                    }),
            );
        }
        self.queue_decision(
            player,
            "Copy Chain Lightning?",
            DecisionVisibility::Private,
            DecisionPreference::Neutral,
            1..=1,
            false,
            options,
            DecisionContinuation::ChainLightning {
                player,
                spell,
                targets,
            },
        );
    }

    fn queue_fork_decision(&mut self, player: PlayerId, spell: StackObject) {
        let mut target_lists =
            self.behavior(spell.card.definition)
                .map_or_else(Vec::new, |behavior| {
                    self.legal_target_lists(behavior, spell.x, player, Some(spell.targets.len()))
                });
        target_lists.push(spell.targets.clone());
        target_lists.sort_unstable();
        target_lists.dedup();
        let options = target_lists
            .iter()
            .enumerate()
            .map(|(index, targets)| DecisionOption {
                id: u32::try_from(index).unwrap_or(u32::MAX),
                label: if *targets == spell.targets {
                    "Keep original targets".into()
                } else {
                    let labels = targets
                        .iter()
                        .map(|target| self.target_label(player, *target))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("Copy with targets {labels}")
                },
                card: None,
                zone: DecisionZone::None,
            })
            .collect();
        self.queue_decision(
            player,
            "Choose targets for Fork's copy",
            DecisionVisibility::Private,
            DecisionPreference::Neutral,
            1..=1,
            false,
            options,
            DecisionContinuation::Fork {
                player,
                spell,
                target_lists,
            },
        );
    }

    fn queue_mana_vault_decision(&mut self, player: PlayerId, permanent: CardInstanceId) {
        let mut options = vec![DecisionOption {
            id: 0,
            label: "Leave Mana Vault tapped".into(),
            card: None,
            zone: DecisionZone::None,
        }];
        if self.can_pay_cost(player, ManaCost::new(4, 0), 0) {
            options.push(DecisionOption {
                id: 1,
                label: "Pay 4 to untap Mana Vault".into(),
                card: None,
                zone: DecisionZone::None,
            });
        }
        self.queue_decision(
            player,
            "Mana Vault would remain tapped",
            DecisionVisibility::Private,
            DecisionPreference::Neutral,
            1..=1,
            false,
            options,
            DecisionContinuation::ManaVault { player, permanent },
        );
    }

    fn push_copy(&mut self, mut spell: StackObject, player: PlayerId, targets: Vec<Target>) {
        spell.id = StackObjectId(self.next_stack_id);
        self.next_stack_id += 1;
        spell.controller = player;
        spell.targets = targets;
        spell.is_copy = true;
        self.stack.push(spell);
    }

    fn card_decision_options(
        &self,
        cards: &[CardInstance],
        zone: DecisionZone,
    ) -> Vec<DecisionOption> {
        cards
            .iter()
            .enumerate()
            .map(|(index, card)| DecisionOption {
                id: u32::try_from(index).unwrap_or(u32::MAX),
                label: self.catalog.get(card.definition).map_or_else(
                    || "Unknown card".into(),
                    |definition| definition.name.clone(),
                ),
                card: Some((card.id, card.definition)),
                zone,
            })
            .collect()
    }

    fn queue_balance_task(&mut self, task: BalanceTask, remaining: Vec<BalanceTask>) {
        let options = self.card_decision_options(&task.cards, task.zone);
        self.queue_decision(
            task.player,
            task.prompt.clone(),
            if task.zone == DecisionZone::Hand {
                DecisionVisibility::Private
            } else {
                DecisionVisibility::Public
            },
            DecisionPreference::LowerCardValue,
            task.count..=task.count,
            false,
            options,
            DecisionContinuation::Balance { task, remaining },
        );
    }

    fn queue_time_vault_decision(
        &mut self,
        permanent: CardInstanceId,
        remaining: Vec<CardInstanceId>,
    ) {
        let card = self
            .battlefield
            .iter()
            .find(|candidate| candidate.card.id == permanent)
            .map(|permanent| (permanent.card.id, permanent.card.definition));
        self.queue_decision(
            self.active_player,
            "Time Vault would remain tapped",
            DecisionVisibility::Public,
            DecisionPreference::Neutral,
            1..=1,
            false,
            vec![
                DecisionOption {
                    id: 0,
                    label: "Leave Time Vault tapped".into(),
                    card,
                    zone: DecisionZone::Battlefield,
                },
                DecisionOption {
                    id: 1,
                    label: "Untap Time Vault and skip your next turn".into(),
                    card,
                    zone: DecisionZone::Battlefield,
                },
            ],
            DecisionContinuation::TimeVault {
                permanent,
                remaining,
            },
        );
    }

    fn finish_untap_choices(&mut self) {
        let mut vaults = self
            .battlefield
            .iter()
            .filter(|permanent| {
                permanent.controller == self.active_player
                    && permanent.tapped
                    && self.effective_behavior(permanent) == Some(CardBehavior::TimeVault)
            })
            .map(|permanent| permanent.card.id)
            .collect::<Vec<_>>();
        if vaults.is_empty() {
            self.handle_upkeep_triggers();
        } else {
            let first = vaults.remove(0);
            self.queue_time_vault_decision(first, vaults);
        }
    }

    fn queue_sylvan_select(
        &mut self,
        player: PlayerId,
        candidates: Vec<CardInstanceId>,
        choices_left: usize,
    ) {
        let cards = self.players[player.index()]
            .hand
            .iter()
            .filter(|card| candidates.contains(&card.id))
            .cloned()
            .collect::<Vec<_>>();
        let options = self.card_decision_options(&cards, DecisionZone::DrawnThisStep);
        self.queue_decision(
            player,
            format!("Choose a card drawn this step ({choices_left} remaining)"),
            DecisionVisibility::Private,
            DecisionPreference::LowerCardValue,
            1..=1,
            false,
            options,
            DecisionContinuation::SylvanSelect {
                player,
                candidates,
                choices_left,
            },
        );
    }

    fn queue_sylvan_mode(
        &mut self,
        player: PlayerId,
        card: CardInstanceId,
        candidates: Vec<CardInstanceId>,
        choices_left: usize,
    ) {
        let card_info = self.players[player.index()]
            .hand
            .iter()
            .find(|candidate| candidate.id == card)
            .map(|card| (card.id, card.definition));
        let card_name = card_info
            .and_then(|(_, definition)| self.catalog.get(definition))
            .map_or("this card", |card| card.name.as_str());
        let mut options = vec![DecisionOption {
            id: 0,
            label: format!("Put {card_name} back on top"),
            card: card_info,
            zone: DecisionZone::DrawnThisStep,
        }];
        if self.players[player.index()].life >= 4 {
            options.push(DecisionOption {
                id: 1,
                label: format!("Pay 4 life to keep {card_name}"),
                card: card_info,
                zone: DecisionZone::DrawnThisStep,
            });
        }
        self.queue_decision(
            player,
            format!("Keep {card_name}?"),
            DecisionVisibility::Private,
            DecisionPreference::Neutral,
            1..=1,
            false,
            options,
            DecisionContinuation::SylvanMode {
                player,
                card,
                candidates,
                choices_left,
            },
        );
    }

    #[allow(clippy::too_many_lines)]
    fn choose_decision(&mut self, player: PlayerId, decision: u32, options: &[u32]) {
        let pending = self.pending_decisions.remove(0);
        debug_assert_eq!(pending.observation.id, decision);
        match pending.continuation {
            DecisionContinuation::IronStar { player } => {
                if options.contains(&1) {
                    let cost = ManaCost::new(1, 0);
                    self.activate_mana_for_cost(player, cost, 0);
                    pay_cost(&mut self.players[player.index()].mana_pool, cost, 0);
                    self.players[player.index()].life += 1;
                }
            }
            DecisionContinuation::ChainLightning {
                player,
                spell,
                targets,
            } => {
                if let Some(option) = options.first().copied()
                    && option > 0
                    && let Some(target) = targets.get(usize::try_from(option - 1).unwrap_or(0))
                {
                    let cost = ManaCost::new(0, 2);
                    self.activate_mana_for_cost(player, cost, 0);
                    pay_cost(&mut self.players[player.index()].mana_pool, cost, 0);
                    self.push_copy(spell, player, vec![*target]);
                }
            }
            DecisionContinuation::Fork {
                player,
                spell,
                target_lists,
            } => {
                if let Some(option) = options.first().copied()
                    && let Some(targets) = target_lists.get(usize::try_from(option).unwrap_or(0))
                {
                    self.push_copy(spell, player, targets.clone());
                }
            }
            DecisionContinuation::ManaVault { player, permanent } => {
                let cost = ManaCost::new(4, 0);
                // Multiple tapped Mana Vaults queue their upkeep decisions at
                // once.  Paying for an earlier vault can make a later
                // decision's previously-offered payment option stale.
                if options.contains(&1) && self.can_pay_cost(player, cost, 0) {
                    self.activate_mana_for_cost(player, cost, 0);
                    pay_cost(&mut self.players[player.index()].mana_pool, cost, 0);
                    if let Some(vault) = self
                        .battlefield
                        .iter_mut()
                        .find(|candidate| candidate.card.id == permanent)
                    {
                        vault.tapped = false;
                    }
                }
            }
            DecisionContinuation::Tutor => {
                let Some(option) = pending
                    .observation
                    .options
                    .iter()
                    .find(|option| options.contains(&option.id))
                else {
                    return;
                };
                let Some((card, _)) = option.card else {
                    return;
                };
                if let Some(card) = remove_card(&mut self.players[player.index()].library, card) {
                    self.players[player.index()].hand.push(card);
                    self.rng.shuffle(&mut self.players[player.index()].library);
                }
            }
            DecisionContinuation::RecallCost {
                player,
                card,
                targets,
                x,
            } => {
                for option in &pending.observation.options {
                    if options.contains(&option.id)
                        && let Some((card, _)) = option.card
                        && let Some(card) =
                            remove_card(&mut self.players[player.index()].hand, card)
                    {
                        self.players[player.index()].graveyard.push(card);
                    }
                }
                self.finish_cast_spell(player, card, targets, &[], x);
            }
            DecisionContinuation::RecallReturn { player } => {
                for option in &pending.observation.options {
                    if options.contains(&option.id)
                        && let Some((card, _)) = option.card
                        && let Some(card) =
                            remove_card(&mut self.players[player.index()].graveyard, card)
                    {
                        self.players[player.index()].hand.push(card);
                    }
                }
            }
            DecisionContinuation::Balance {
                task,
                mut remaining,
            } => {
                for option in &pending.observation.options {
                    if !options.contains(&option.id) {
                        continue;
                    }
                    let Some((card, _)) = option.card else {
                        continue;
                    };
                    match task.action {
                        BalanceAction::Sacrifice => self.destroy_permanent(card),
                        BalanceAction::Discard => {
                            if let Some(card) =
                                remove_card(&mut self.players[task.player.index()].hand, card)
                            {
                                self.players[task.player.index()].graveyard.push(card);
                            }
                        }
                    }
                }
                if !remaining.is_empty() {
                    let next = remaining.remove(0);
                    self.queue_balance_task(next, remaining);
                }
            }
            DecisionContinuation::TimeVault {
                permanent,
                mut remaining,
            } => {
                if options.contains(&1) {
                    if let Some(vault) = self
                        .battlefield
                        .iter_mut()
                        .find(|candidate| candidate.card.id == permanent)
                    {
                        vault.tapped = false;
                    }
                    self.skipped_turns[player.index()] =
                        self.skipped_turns[player.index()].saturating_add(1);
                }
                if remaining.is_empty() {
                    self.handle_upkeep_triggers();
                } else {
                    let next = remaining.remove(0);
                    self.queue_time_vault_decision(next, remaining);
                }
            }
            DecisionContinuation::SylvanSelect {
                player,
                mut candidates,
                choices_left,
            } => {
                let selected = pending
                    .observation
                    .options
                    .iter()
                    .find(|option| options.contains(&option.id))
                    .and_then(|option| option.card)
                    .map(|(card, _)| card);
                if let Some(card) = selected {
                    candidates.retain(|candidate| *candidate != card);
                    self.queue_sylvan_mode(player, card, candidates, choices_left);
                }
            }
            DecisionContinuation::SylvanMode {
                player,
                card,
                candidates,
                choices_left,
            } => {
                if options.contains(&1) {
                    self.players[player.index()].life -= 4;
                    self.check_life_totals();
                } else if let Some(card) = remove_card(&mut self.players[player.index()].hand, card)
                {
                    self.players[player.index()].library.push(card);
                }
                if choices_left > 1 && self.result.is_none() {
                    self.queue_sylvan_select(player, candidates, choices_left - 1);
                }
            }
        }
    }

    fn cancel_decision(&mut self, decision: u32) {
        debug_assert_eq!(self.pending_decisions[0].observation.id, decision);
        self.pending_decisions.remove(0);
    }

    fn add_land_actions(&self, player: PlayerId, actions: &mut Vec<Action>) {
        let state = &self.players[player.index()];
        if player != self.active_player
            || !self.step.is_main()
            || !self.stack.is_empty()
            || state.land_played_this_turn
        {
            return;
        }
        actions.extend(
            state
                .hand
                .iter()
                .filter(|card| self.kind(card.definition) == Some(CardKind::Land))
                .map(|card| Action::PlayLand { card: card.id }),
        );
    }

    fn add_spell_actions(&self, player: PlayerId, actions: &mut Vec<Action>) {
        let state = &self.players[player.index()];
        for card in &state.hand {
            let Some(behavior) = self.behavior(card.definition) else {
                continue;
            };
            let kind = behavior.kind();
            if behavior == CardBehavior::Unsupported || kind == CardKind::Land {
                continue;
            }
            if !matches!(kind, CardKind::Instant)
                && (player != self.active_player || !self.step.is_main() || !self.stack.is_empty())
            {
                continue;
            }
            let cost = behavior.mana_cost();
            let max_x = if cost.variable_x {
                self.maximum_x(player, cost)
            } else {
                0
            };
            for x in 0..=max_x {
                if behavior == CardBehavior::Recall
                    && usize::from(x) > state.hand.len().saturating_sub(1)
                {
                    continue;
                }
                let target_counts: Vec<_> = if behavior == CardBehavior::Fireball {
                    (1..=self.damage_targets().len())
                        .filter(|count| {
                            self.can_pay_cost(
                                player,
                                add_generic(cost, fireball_extra_cost(behavior, *count)),
                                x,
                            )
                        })
                        .map(Some)
                        .collect()
                } else {
                    vec![None]
                };
                for target_count in target_counts {
                    for targets in self.legal_target_lists(behavior, x, player, target_count) {
                        let extra = fireball_extra_cost(behavior, targets.len());
                        if !self.can_pay_cost(player, add_generic(cost, extra), x) {
                            continue;
                        }
                        let sacrifice_choices = if behavior == CardBehavior::GoblinGrenade {
                            self.battlefield
                                .iter()
                                .filter(|permanent| {
                                    permanent.controller == player
                                        && self
                                            .behavior(permanent.card.definition)
                                            .is_some_and(CardBehavior::is_goblin)
                                })
                                .map(|permanent| vec![permanent.card.id])
                                .collect()
                        } else {
                            vec![Vec::new()]
                        };
                        for sacrifices in sacrifice_choices {
                            actions.push(Action::CastSpell {
                                card: card.id,
                                targets: targets.clone(),
                                sacrifices,
                                x,
                            });
                        }
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn legal_target_lists(
        &self,
        behavior: CardBehavior,
        x: u16,
        player: PlayerId,
        exact_count: Option<usize>,
    ) -> Vec<Vec<Target>> {
        match behavior {
            CardBehavior::AncestralRecall | CardBehavior::Braingeyser => {
                vec![
                    vec![Target::Player(PlayerId::One)],
                    vec![Target::Player(PlayerId::Two)],
                ]
            }
            CardBehavior::LightningBolt
            | CardBehavior::ChainLightning
            | CardBehavior::GoblinGrenade
            | CardBehavior::DrainLife
            | CardBehavior::PsionicBlast => self
                .damage_targets()
                .into_iter()
                .map(|target| vec![target])
                .collect(),
            CardBehavior::Fireball => {
                let targets = self.damage_targets();
                let counts: Vec<_> =
                    exact_count.map_or_else(|| (1..=targets.len()).collect(), |count| vec![count]);
                counts
                    .into_iter()
                    .flat_map(|count| target_combinations(&targets, count))
                    .collect()
            }
            CardBehavior::Shatter | CardBehavior::DivineOffering => self
                .battlefield
                .iter()
                .filter(|permanent| self.is_artifact_permanent(permanent))
                .map(|permanent| vec![Target::Permanent(permanent.card.id)])
                .collect(),
            CardBehavior::CopyArtifact => self
                .battlefield
                .iter()
                .filter(|permanent| self.is_artifact_permanent(permanent))
                .map(|permanent| vec![Target::Permanent(permanent.card.id)])
                .collect(),
            CardBehavior::DustToDust => {
                let artifacts: Vec<_> = self
                    .battlefield
                    .iter()
                    .filter(|permanent| self.is_artifact_permanent(permanent))
                    .map(|permanent| Target::Permanent(permanent.card.id))
                    .collect();
                target_combinations(&artifacts, 2)
            }
            CardBehavior::Disenchant => self
                .battlefield
                .iter()
                .filter(|permanent| {
                    self.is_artifact_permanent(permanent)
                        || self.kind(permanent.card.definition) == Some(CardKind::Enchantment)
                })
                .map(|permanent| vec![Target::Permanent(permanent.card.id)])
                .collect(),
            CardBehavior::SwordsToPlowshares => self
                .battlefield
                .iter()
                .filter(|permanent| self.power(permanent).is_some())
                .map(|permanent| vec![Target::Permanent(permanent.card.id)])
                .collect(),
            CardBehavior::Terror => self
                .battlefield
                .iter()
                .filter(|permanent| {
                    self.power(permanent).is_some()
                        && !self.is_artifact_permanent(permanent)
                        && !self
                            .behavior(permanent.card.definition)
                            .is_some_and(CardBehavior::is_black)
                })
                .map(|permanent| vec![Target::Permanent(permanent.card.id)])
                .collect(),
            CardBehavior::Sinkhole | CardBehavior::StoneRain => self
                .battlefield
                .iter()
                .filter(|permanent| self.kind(permanent.card.definition) == Some(CardKind::Land))
                .map(|permanent| vec![Target::Permanent(permanent.card.id)])
                .collect(),
            CardBehavior::GiantGrowth | CardBehavior::Berserk => self
                .battlefield
                .iter()
                .filter(|permanent| {
                    permanent.controller == player && self.power(permanent).is_some()
                })
                .map(|permanent| vec![Target::Permanent(permanent.card.id)])
                .collect(),
            CardBehavior::HurkylsRecall => vec![
                vec![Target::Player(PlayerId::One)],
                vec![Target::Player(PlayerId::Two)],
            ],
            CardBehavior::Detonate => self
                .battlefield
                .iter()
                .filter(|permanent| {
                    self.is_artifact_permanent(permanent)
                        && self.mana_value(permanent.card.definition) == x
                })
                .map(|permanent| vec![Target::Permanent(permanent.card.id)])
                .collect(),
            CardBehavior::Fork => self
                .stack
                .iter()
                .filter(|object| {
                    object.kind == StackObjectKind::Spell
                        && matches!(
                            self.kind(object.card.definition),
                            Some(CardKind::Instant | CardKind::Sorcery)
                        )
                })
                .map(|object| vec![Target::Spell(object.id)])
                .collect(),
            CardBehavior::Counterspell | CardBehavior::ManaDrain => self
                .stack
                .iter()
                .filter(|object| object.kind == StackObjectKind::Spell)
                .map(|object| vec![Target::Spell(object.id)])
                .collect(),
            CardBehavior::RedElementalBlast => {
                let mut targets = self
                    .stack
                    .iter()
                    .filter(|object| {
                        object.kind == StackObjectKind::Spell
                            && self
                                .behavior(object.card.definition)
                                .is_some_and(CardBehavior::is_blue)
                    })
                    .map(|object| vec![Target::Spell(object.id)])
                    .collect::<Vec<_>>();
                targets.extend(
                    self.battlefield
                        .iter()
                        .filter(|permanent| {
                            self.effective_behavior(permanent)
                                .is_some_and(CardBehavior::is_blue)
                        })
                        .map(|permanent| vec![Target::Permanent(permanent.card.id)]),
                );
                targets
            }
            CardBehavior::BlueElementalBlast => {
                let mut targets = self
                    .stack
                    .iter()
                    .filter(|object| {
                        object.kind == StackObjectKind::Spell
                            && self
                                .behavior(object.card.definition)
                                .is_some_and(CardBehavior::is_red)
                    })
                    .map(|object| vec![Target::Spell(object.id)])
                    .collect::<Vec<_>>();
                targets.extend(
                    self.battlefield
                        .iter()
                        .filter(|permanent| {
                            self.effective_behavior(permanent)
                                .is_some_and(CardBehavior::is_red)
                        })
                        .map(|permanent| vec![Target::Permanent(permanent.card.id)]),
                );
                targets
            }
            CardBehavior::BlackVise => vec![vec![Target::Player(player.opponent())]],
            _ => vec![Vec::new()],
        }
    }

    #[allow(clippy::too_many_lines)]
    fn add_ability_actions(&self, player: PlayerId, actions: &mut Vec<Action>) {
        for permanent in self
            .battlefield
            .iter()
            .filter(|permanent| permanent.controller == player)
        {
            match self.effective_behavior(permanent) {
                Some(CardBehavior::Atog) => {
                    actions.extend(
                        self.battlefield
                            .iter()
                            .filter(|candidate| {
                                candidate.controller == player
                                    && self.is_artifact_permanent(candidate)
                            })
                            .map(|candidate| Action::ActivateAbility {
                                source: permanent.card.id,
                                target: None,
                                sacrifice: Some(candidate.card.id),
                            }),
                    );
                }
                Some(CardBehavior::GlassesOfUrza) if !permanent.tapped => {
                    for target in [PlayerId::One, PlayerId::Two] {
                        actions.push(Action::ActivateAbility {
                            source: permanent.card.id,
                            target: Some(Target::Player(target)),
                            sacrifice: None,
                        });
                    }
                }
                Some(CardBehavior::IcyManipulator)
                    if !permanent.tapped
                        && self.can_use_tap_ability(permanent)
                        && self.can_pay_cost(player, ManaCost::new(1, 0), 0) =>
                {
                    actions.extend(self.battlefield.iter().map(|candidate| {
                        Action::ActivateAbility {
                            source: permanent.card.id,
                            target: Some(Target::Permanent(candidate.card.id)),
                            sacrifice: None,
                        }
                    }));
                }
                Some(CardBehavior::RelicBarrier)
                    if !permanent.tapped && self.can_use_tap_ability(permanent) =>
                {
                    actions.extend(
                        self.battlefield
                            .iter()
                            .filter(|candidate| self.is_artifact_permanent(candidate))
                            .map(|candidate| Action::ActivateAbility {
                                source: permanent.card.id,
                                target: Some(Target::Permanent(candidate.card.id)),
                                sacrifice: None,
                            }),
                    );
                }
                Some(CardBehavior::Pendelhaven)
                    if !permanent.tapped && self.can_use_tap_ability(permanent) =>
                {
                    actions.extend(
                        self.battlefield
                            .iter()
                            .filter(|candidate| {
                                self.power(candidate) == Some(1)
                                    && self.toughness(candidate) == Some(1)
                            })
                            .map(|candidate| Action::ActivateAbility {
                                source: permanent.card.id,
                                target: Some(Target::Permanent(candidate.card.id)),
                                sacrifice: None,
                            }),
                    );
                }
                Some(CardBehavior::SageOfLatNam)
                    if !permanent.tapped && self.can_use_tap_ability(permanent) =>
                {
                    actions.extend(
                        self.battlefield
                            .iter()
                            .filter(|candidate| {
                                candidate.controller == player
                                    && self.is_artifact_permanent(candidate)
                            })
                            .map(|candidate| Action::ActivateAbility {
                                source: permanent.card.id,
                                target: None,
                                sacrifice: Some(candidate.card.id),
                            }),
                    );
                }
                Some(CardBehavior::SedgeTroll)
                    if self.can_pay_cost(player, ManaCost::colored(0, 0, 0, 0, 1, 0), 0) =>
                {
                    actions.push(Action::ActivateAbility {
                        source: permanent.card.id,
                        target: None,
                        sacrifice: None,
                    });
                }
                Some(CardBehavior::StoneGiant)
                    if !permanent.tapped && self.can_use_tap_ability(permanent) =>
                {
                    let power = self.power(permanent).unwrap_or(0);
                    actions.extend(
                        self.battlefield
                            .iter()
                            .filter(|candidate| {
                                candidate.controller == player
                                    && self.toughness(candidate).is_some_and(|value| value < power)
                            })
                            .map(|candidate| Action::ActivateAbility {
                                source: permanent.card.id,
                                target: Some(Target::Permanent(candidate.card.id)),
                                sacrifice: None,
                            }),
                    );
                }
                Some(
                    CardBehavior::GoblinBalloonBrigade
                    | CardBehavior::GraniteGargoyle
                    | CardBehavior::DragonWhelp,
                ) if self.players[player.index()].mana_pool.red > 0 => {
                    actions.push(Action::ActivateAbility {
                        source: permanent.card.id,
                        target: None,
                        sacrifice: None,
                    });
                }
                Some(CardBehavior::MishrasFactory)
                    if self.can_pay_cost(player, ManaCost::new(1, 0), 0) =>
                {
                    actions.push(Action::ActivateAbility {
                        source: permanent.card.id,
                        target: None,
                        sacrifice: None,
                    });
                    if !permanent.tapped && self.can_use_tap_ability(permanent) {
                        actions.extend(
                            self.battlefield
                                .iter()
                                .filter(|candidate| {
                                    candidate.controller == player
                                        && candidate.factory_animated
                                        && self.effective_behavior(candidate)
                                            == Some(CardBehavior::MishrasFactory)
                                })
                                .map(|candidate| Action::ActivateAbility {
                                    source: permanent.card.id,
                                    target: Some(Target::Permanent(candidate.card.id)),
                                    sacrifice: None,
                                }),
                        );
                    }
                }
                Some(CardBehavior::MishrasFactory)
                    if !permanent.tapped && self.can_use_tap_ability(permanent) =>
                {
                    actions.extend(
                        self.battlefield
                            .iter()
                            .filter(|candidate| {
                                candidate.controller == player
                                    && candidate.factory_animated
                                    && self.effective_behavior(candidate)
                                        == Some(CardBehavior::MishrasFactory)
                            })
                            .map(|candidate| Action::ActivateAbility {
                                source: permanent.card.id,
                                target: Some(Target::Permanent(candidate.card.id)),
                                sacrifice: None,
                            }),
                    );
                }
                Some(CardBehavior::StripMine)
                    if !permanent.tapped && self.can_use_tap_ability(permanent) =>
                {
                    actions.extend(
                        self.battlefield
                            .iter()
                            .filter(|candidate| {
                                self.kind(candidate.card.definition) == Some(CardKind::Land)
                            })
                            .map(|candidate| Action::ActivateAbility {
                                source: permanent.card.id,
                                target: Some(Target::Permanent(candidate.card.id)),
                                sacrifice: Some(permanent.card.id),
                            }),
                    );
                }
                Some(CardBehavior::ChaosOrb)
                    if !permanent.tapped && self.can_pay_cost(player, ManaCost::new(1, 0), 0) =>
                {
                    actions.extend(
                        self.battlefield
                            .iter()
                            .filter(|candidate| candidate.card.id != permanent.card.id)
                            .map(|candidate| Action::ActivateAbility {
                                source: permanent.card.id,
                                target: Some(Target::Permanent(candidate.card.id)),
                                sacrifice: None,
                            }),
                    );
                }
                Some(CardBehavior::OrcishMechanics)
                    if !permanent.tapped && self.can_use_tap_ability(permanent) =>
                {
                    for sacrificed in self.battlefield.iter().filter(|candidate| {
                        candidate.controller == player
                            && candidate.card.id != permanent.card.id
                            && self.is_artifact_permanent(candidate)
                    }) {
                        actions.extend(self.damage_targets().into_iter().map(|target| {
                            Action::ActivateAbility {
                                source: permanent.card.id,
                                target: Some(target),
                                sacrifice: Some(sacrificed.card.id),
                            }
                        }));
                    }
                }
                Some(CardBehavior::Triskelion) if permanent.plus_one_counters > 0 => {
                    actions.extend(self.damage_targets().into_iter().map(|target| {
                        Action::ActivateAbility {
                            source: permanent.card.id,
                            target: Some(target),
                            sacrifice: None,
                        }
                    }));
                }
                Some(CardBehavior::JayemdaeTome)
                    if !permanent.tapped
                        && self.can_use_tap_ability(permanent)
                        && self.can_pay_cost(player, ManaCost::new(4, 0), 0) =>
                {
                    actions.push(Action::ActivateAbility {
                        source: permanent.card.id,
                        target: None,
                        sacrifice: None,
                    });
                }
                Some(CardBehavior::LibraryOfAlexandria)
                    if !permanent.tapped
                        && self.can_use_tap_ability(permanent)
                        && self.players[player.index()].hand.len() == 7 =>
                {
                    actions.push(Action::ActivateAbility {
                        source: permanent.card.id,
                        target: None,
                        sacrifice: None,
                    });
                }
                Some(CardBehavior::MazeOfIth)
                    if !permanent.tapped && self.can_use_tap_ability(permanent) =>
                {
                    actions.extend(
                        self.battlefield
                            .iter()
                            .filter(|candidate| candidate.attacking)
                            .map(|candidate| Action::ActivateAbility {
                                source: permanent.card.id,
                                target: Some(Target::Permanent(candidate.card.id)),
                                sacrifice: None,
                            }),
                    );
                }
                Some(CardBehavior::NevinyrralsDisk)
                    if !permanent.tapped
                        && self.can_use_tap_ability(permanent)
                        && self.can_pay_cost(player, ManaCost::new(1, 0), 0) =>
                {
                    actions.push(Action::ActivateAbility {
                        source: permanent.card.id,
                        target: None,
                        sacrifice: None,
                    });
                }
                Some(CardBehavior::IcatianJavelineers)
                    if !permanent.tapped
                        && self.can_use_tap_ability(permanent)
                        && permanent.plus_one_counters > 0 =>
                {
                    actions.extend(self.damage_targets().into_iter().map(|target| {
                        Action::ActivateAbility {
                            source: permanent.card.id,
                            target: Some(target),
                            sacrifice: None,
                        }
                    }));
                }
                Some(CardBehavior::TimeVault)
                    if !permanent.tapped && self.can_use_tap_ability(permanent) =>
                {
                    actions.push(Action::ActivateAbility {
                        source: permanent.card.id,
                        target: None,
                        sacrifice: None,
                    });
                }
                _ => {}
            }
        }
    }

    fn behavior(&self, definition: CardDefinitionId) -> Option<CardBehavior> {
        self.catalog.get(definition).map(|card| card.behavior)
    }

    fn kind(&self, definition: CardDefinitionId) -> Option<CardKind> {
        self.behavior(definition).map(CardBehavior::kind)
    }

    fn mana_value(&self, definition: CardDefinitionId) -> u16 {
        self.behavior(definition)
            .map(CardBehavior::mana_cost)
            .map_or(0, |cost| cost.generic + colored_cost_total(cost))
    }

    fn play_land(&mut self, player: PlayerId, card_id: CardInstanceId) {
        let card = remove_card(&mut self.players[player.index()].hand, card_id)
            .expect("legal land action references a card in hand");
        self.players[player.index()].land_played_this_turn = true;
        self.battlefield.push(Permanent {
            card,
            controller: player,
            tapped: false,
            entered_turn: self.turn,
            damage: 0,
            power_bonus: 0,
            toughness_bonus: 0,
            attacking: false,
            blocking: None,
            chosen_player: None,
            destroy_at_end: false,
            flying_until_end: false,
            factory_animated: false,
            dragon_whelp_activations: 0,
            plus_one_counters: 0,
            combat_damage_assignment: Vec::new(),
            copied_behavior: None,
            regeneration_shields: 0,
            trample_until_end: false,
            berserked: false,
            attacked_this_turn: false,
        });
        self.consecutive_passes = 0;
        self.events.push(GameEvent::LandPlayed {
            player,
            card: card_id,
        });
        let ankhs = self.count_behavior(CardBehavior::AnkhOfMishra);
        if ankhs > 0 {
            self.deal_damage(player, 2 * ankhs);
            self.check_life_totals();
        }
    }

    fn activate_mana_source(&mut self, player: PlayerId, source: CardInstanceId, color: ManaColor) {
        let production = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == source)
            .and_then(|permanent| self.mana_production(permanent, color))
            .expect("legal mana action references a mana source");
        let is_lotus = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == source)
            .is_some_and(|permanent| {
                self.effective_behavior(permanent) == Some(CardBehavior::BlackLotus)
            });
        let is_city = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == source)
            .is_some_and(|permanent| {
                self.effective_behavior(permanent) == Some(CardBehavior::CityOfBrass)
            });
        if is_lotus {
            self.destroy_permanent(source);
        } else if let Some(permanent) = self
            .battlefield
            .iter_mut()
            .find(|permanent| permanent.card.id == source)
        {
            permanent.tapped = true;
        }
        self.players[player.index()].mana_pool.add(production);
        if is_city {
            self.deal_damage(player, 1);
            self.check_life_totals();
        }
        self.consecutive_passes = 0;
        self.events.push(GameEvent::ManaAdded { player, source });
    }

    fn cast_spell(
        &mut self,
        player: PlayerId,
        card_id: CardInstanceId,
        targets: Vec<Target>,
        sacrifices: &[CardInstanceId],
        x: u16,
    ) {
        let behavior = self.players[player.index()]
            .hand
            .iter()
            .find(|card| card.id == card_id)
            .and_then(|card| self.behavior(card.definition))
            .expect("legal cast action references a cataloged card");
        if behavior == CardBehavior::Recall && x > 0 {
            let eligible = self.players[player.index()]
                .hand
                .iter()
                .filter(|card| card.id != card_id)
                .cloned()
                .collect::<Vec<_>>();
            let options = self.card_decision_options(&eligible, DecisionZone::Hand);
            self.queue_decision(
                player,
                format!("Discard {x} card(s) to cast Recall"),
                DecisionVisibility::Private,
                DecisionPreference::LowerCardValue,
                usize::from(x)..=usize::from(x),
                true,
                options,
                DecisionContinuation::RecallCost {
                    player,
                    card: card_id,
                    targets,
                    x,
                },
            );
            return;
        }
        self.finish_cast_spell(player, card_id, targets, sacrifices, x);
    }

    fn finish_cast_spell(
        &mut self,
        player: PlayerId,
        card_id: CardInstanceId,
        targets: Vec<Target>,
        sacrifices: &[CardInstanceId],
        x: u16,
    ) {
        let card = remove_card(&mut self.players[player.index()].hand, card_id)
            .expect("legal cast action references a card in hand");
        let behavior = self.behavior(card.definition).expect("cataloged card");
        let cost = add_generic(
            behavior.mana_cost(),
            fireball_extra_cost(behavior, targets.len()),
        );
        self.activate_mana_for_cost(player, cost, x);
        pay_cost(&mut self.players[player.index()].mana_pool, cost, x);
        for sacrificed in sacrifices {
            self.sacrifice_permanent(*sacrificed);
        }
        let stack_id = StackObjectId(self.next_stack_id);
        self.next_stack_id += 1;
        self.stack.push(StackObject {
            id: stack_id,
            kind: StackObjectKind::Spell,
            card,
            controller: player,
            targets: targets.clone(),
            chosen_permanents: Vec::new(),
            x,
            is_copy: false,
        });
        self.consecutive_passes = 0;
        self.events.push(GameEvent::SpellCast {
            player,
            card: card_id,
            targets,
        });
        if behavior.is_red() {
            let iron_star_controllers = self
                .battlefield
                .iter()
                .filter(|permanent| {
                    self.effective_behavior(permanent) == Some(CardBehavior::IronStar)
                })
                .map(|permanent| permanent.controller)
                .collect::<Vec<_>>();
            for controller in iron_star_controllers {
                self.queue_iron_star_decision(controller);
            }
        }
    }

    fn pass_priority(&mut self, _player: PlayerId) {
        self.consecutive_passes += 1;
        if self.consecutive_passes == 1 {
            self.priority = self.priority.opponent();
            return;
        }

        self.consecutive_passes = 0;
        if self.stack.is_empty() {
            self.advance_step();
        } else {
            self.resolve_stack_top();
            if self.result.is_none() {
                self.priority = self.active_player;
            }
        }
    }

    fn resolve_stack_top(&mut self) {
        let object = self
            .stack
            .pop()
            .expect("resolution is requested only for a nonempty stack");
        if object.kind == StackObjectKind::ActivatedAbility {
            self.resolve_activated_ability(&object);
            self.events.push(GameEvent::AbilityResolved {
                source: object.card.id,
            });
            self.check_state_based_actions();
            return;
        }
        let behavior = self
            .behavior(object.card.definition)
            .expect("stack cards are cataloged");
        if behavior.kind().is_permanent() {
            let chosen_player = match object.targets.first() {
                Some(Target::Player(player)) => Some(*player),
                _ => None,
            };
            let copied_behavior = if behavior == CardBehavior::CopyArtifact {
                object.targets.first().and_then(|target| match target {
                    Target::Permanent(id) => self
                        .battlefield
                        .iter()
                        .find(|permanent| permanent.card.id == *id)
                        .and_then(|permanent| self.effective_behavior(permanent))
                        .filter(|copied| copied.kind().is_artifact()),
                    Target::Player(_) | Target::Spell(_) => None,
                })
            } else {
                None
            };
            self.battlefield.push(Permanent {
                card: object.card.clone(),
                controller: object.controller,
                tapped: matches!(
                    behavior,
                    CardBehavior::NevinyrralsDisk | CardBehavior::TimeVault
                ),
                entered_turn: self.turn,
                damage: 0,
                power_bonus: 0,
                toughness_bonus: 0,
                attacking: false,
                blocking: None,
                chosen_player,
                destroy_at_end: false,
                flying_until_end: false,
                factory_animated: false,
                dragon_whelp_activations: 0,
                plus_one_counters: match behavior {
                    CardBehavior::Triskelion | CardBehavior::Tetravus => 3,
                    CardBehavior::IcatianJavelineers => 1,
                    _ => 0,
                },
                combat_damage_assignment: Vec::new(),
                copied_behavior: None,
                regeneration_shields: 0,
                trample_until_end: false,
                berserked: false,
                attacked_this_turn: false,
            });
            if let Some(copied_behavior) = copied_behavior
                && let Some(permanent) = self.battlefield.last_mut()
            {
                permanent.copied_behavior = Some(copied_behavior);
                if copied_behavior == CardBehavior::Tetravus {
                    permanent.plus_one_counters = 3;
                }
            }
        } else {
            self.resolve_spell_effect(&object, behavior);
        }
        let card_id = object.card.id;
        if !behavior.kind().is_permanent() && !object.is_copy {
            if behavior == CardBehavior::Recall {
                self.players[object.card.owner.index()]
                    .exile
                    .push(object.card);
            } else {
                self.players[object.card.owner.index()]
                    .graveyard
                    .push(object.card);
            }
        }
        self.events.push(GameEvent::SpellResolved { card: card_id });
        self.check_state_based_actions();
    }

    fn resolve_activated_ability(&mut self, object: &StackObject) {
        match self.behavior(object.card.definition) {
            Some(CardBehavior::StripMine) => {
                if let Some(Target::Permanent(target)) = object.targets.first().copied() {
                    self.destroy_permanent(target);
                }
            }
            Some(CardBehavior::ChaosOrb)
                if self
                    .battlefield
                    .iter()
                    .any(|permanent| permanent.card.id == object.card.id) =>
            {
                if let Some(chosen) = object.chosen_permanents.first().copied() {
                    self.destroy_permanent(chosen);
                }
                self.destroy_permanent(object.card.id);
            }
            Some(CardBehavior::OrcishMechanics) => {
                self.damage_target(object.targets.first().copied(), 2);
            }
            Some(CardBehavior::IcyManipulator | CardBehavior::RelicBarrier) => {
                if let Some(Target::Permanent(target)) = object.targets.first().copied()
                    && let Some(permanent) = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == target)
                {
                    permanent.tapped = true;
                }
            }
            Some(CardBehavior::Pendelhaven) => {
                if let Some(Target::Permanent(target)) = object.targets.first().copied()
                    && let Some(permanent) = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == target)
                {
                    permanent.power_bonus += 1;
                    permanent.toughness_bonus += 2;
                }
            }
            Some(CardBehavior::SageOfLatNam) => self.draw_cards(object.controller, 1),
            Some(CardBehavior::SedgeTroll) => {
                if let Some(permanent) = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == object.card.id)
                {
                    permanent.regeneration_shields =
                        permanent.regeneration_shields.saturating_add(1);
                }
            }
            Some(CardBehavior::Triskelion | CardBehavior::IcatianJavelineers) => {
                self.damage_target(object.targets.first().copied(), 1);
            }
            Some(CardBehavior::JayemdaeTome | CardBehavior::LibraryOfAlexandria) => {
                self.draw_cards(object.controller, 1);
            }
            Some(CardBehavior::MazeOfIth) => {
                if let Some(Target::Permanent(target)) = object.targets.first()
                    && let Some(creature) = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == *target)
                {
                    creature.tapped = false;
                    creature.attacking = false;
                    creature.combat_damage_assignment.clear();
                }
            }
            Some(CardBehavior::NevinyrralsDisk) => {
                let doomed = self
                    .battlefield
                    .iter()
                    .filter(|permanent| {
                        matches!(
                            self.effective_behavior(permanent).map(CardBehavior::kind),
                            Some(
                                CardKind::Creature
                                    | CardKind::Artifact
                                    | CardKind::ArtifactCreature
                                    | CardKind::Enchantment
                            )
                        )
                    })
                    .map(|permanent| permanent.card.id)
                    .collect::<Vec<_>>();
                for permanent in doomed {
                    self.destroy_permanent(permanent);
                }
            }
            Some(CardBehavior::TimeVault) => self.extra_turns.push(object.controller),
            _ => {}
        }
    }

    #[allow(clippy::too_many_lines)]
    fn resolve_spell_effect(&mut self, object: &StackObject, behavior: CardBehavior) {
        match behavior {
            CardBehavior::AncestralRecall => {
                if let Some(Target::Player(player)) = object.targets.first() {
                    self.draw_cards(*player, 3);
                }
            }
            CardBehavior::Braingeyser => {
                if let Some(Target::Player(player)) = object.targets.first() {
                    self.draw_cards(*player, object.x);
                }
            }
            CardBehavior::Counterspell | CardBehavior::ManaDrain => {
                if let Some(Target::Spell(target)) = object.targets.first() {
                    let drained = self
                        .stack
                        .iter()
                        .find(|candidate| candidate.id == *target)
                        .map_or(0, |candidate| self.mana_value(candidate.card.definition));
                    self.counter_spell(*target);
                    if behavior == CardBehavior::ManaDrain {
                        self.mana_drain_pending[object.controller.index()] = self
                            .mana_drain_pending[object.controller.index()]
                        .saturating_add(drained);
                    }
                }
            }
            CardBehavior::LightningBolt => {
                self.damage_target(object.targets.first().copied(), 3);
            }
            CardBehavior::GiantGrowth => {
                if let Some(Target::Permanent(target)) = object.targets.first().copied()
                    && let Some(permanent) = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == target)
                {
                    permanent.power_bonus += 3;
                    permanent.toughness_bonus += 3;
                }
            }
            CardBehavior::Berserk => {
                if let Some(Target::Permanent(target)) = object.targets.first().copied() {
                    let current_power = self
                        .battlefield
                        .iter()
                        .find(|permanent| permanent.card.id == target)
                        .and_then(|permanent| self.power(permanent))
                        .unwrap_or(0)
                        .max(0);
                    if let Some(permanent) = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == target)
                    {
                        permanent.power_bonus += current_power;
                        permanent.trample_until_end = true;
                        permanent.berserked = true;
                    }
                }
            }
            CardBehavior::GoblinGrenade => {
                self.damage_target(object.targets.first().copied(), 5);
            }
            CardBehavior::ChainLightning => {
                let deciding = match object.targets.first() {
                    Some(Target::Player(player)) => Some(*player),
                    Some(Target::Permanent(id)) => self.permanent_controller(*id),
                    Some(Target::Spell(_)) | None => None,
                };
                self.damage_target(object.targets.first().copied(), 3);
                if let Some(player) = deciding {
                    self.queue_chain_lightning_decision(player, object.clone());
                }
            }
            CardBehavior::Fireball => {
                let divisor = u16::try_from(object.targets.len()).unwrap_or(u16::MAX);
                let amount = object.x.checked_div(divisor).unwrap_or(0);
                for target in &object.targets {
                    self.damage_target(Some(*target), amount);
                }
            }
            CardBehavior::PsionicBlast => {
                self.damage_target(object.targets.first().copied(), 4);
                self.deal_damage(object.controller, 2);
            }
            CardBehavior::DrainLife => {
                self.damage_target(object.targets.first().copied(), object.x);
                self.players[object.controller.index()].life = self.players
                    [object.controller.index()]
                .life
                .saturating_add(i16::try_from(object.x).unwrap_or(i16::MAX));
            }
            CardBehavior::Earthquake => {
                for player in [PlayerId::One, PlayerId::Two] {
                    self.deal_damage(player, object.x);
                }
                let targets = self
                    .battlefield
                    .iter()
                    .filter(|permanent| {
                        self.power(permanent).is_some() && !self.has_flying(permanent)
                    })
                    .map(|permanent| permanent.card.id)
                    .collect::<Vec<_>>();
                for target in targets {
                    self.damage_target(Some(Target::Permanent(target)), object.x);
                }
            }
            CardBehavior::Shatter
            | CardBehavior::Disenchant
            | CardBehavior::Sinkhole
            | CardBehavior::StoneRain => {
                if let Some(Target::Permanent(target)) = object.targets.first() {
                    self.destroy_permanent(*target);
                }
            }
            CardBehavior::Terror => {
                if let Some(Target::Permanent(target)) = object.targets.first() {
                    self.destroy_permanent_without_regeneration(*target);
                }
            }
            CardBehavior::DustToDust => {
                for target in object.targets.iter().filter_map(|target| match target {
                    Target::Permanent(id) => Some(*id),
                    Target::Player(_) | Target::Spell(_) => None,
                }) {
                    self.exile_permanent(target);
                }
            }
            CardBehavior::HurkylsRecall => {
                if let Some(Target::Player(player)) = object.targets.first().copied() {
                    let artifacts: Vec<_> = self
                        .battlefield
                        .iter()
                        .filter(|permanent| {
                            permanent.controller == player && self.is_artifact_permanent(permanent)
                        })
                        .map(|permanent| permanent.card.id)
                        .collect();
                    for artifact in artifacts {
                        self.return_permanent_to_hand(artifact);
                    }
                }
            }
            CardBehavior::WrathOfGod => {
                let creatures: Vec<_> = self
                    .battlefield
                    .iter()
                    .filter(|permanent| self.power(permanent).is_some())
                    .map(|permanent| permanent.card.id)
                    .collect();
                for creature in creatures {
                    self.destroy_permanent_without_regeneration(creature);
                }
            }
            CardBehavior::DivineOffering => {
                if let Some(Target::Permanent(target)) = object.targets.first()
                    && let Some(permanent) = self
                        .battlefield
                        .iter()
                        .find(|permanent| permanent.card.id == *target)
                {
                    let life = self.mana_value(permanent.card.definition);
                    self.destroy_permanent(*target);
                    self.players[object.controller.index()].life = self.players
                        [object.controller.index()]
                    .life
                    .saturating_add(i16::try_from(life).unwrap_or(i16::MAX));
                }
            }
            CardBehavior::SwordsToPlowshares => {
                if let Some(Target::Permanent(target)) = object.targets.first()
                    && let Some(index) = self
                        .battlefield
                        .iter()
                        .position(|permanent| permanent.card.id == *target)
                {
                    let controller = self.battlefield[index].controller;
                    let life = self.power(&self.battlefield[index]).unwrap_or(0).max(0);
                    self.exile_permanent(*target);
                    self.players[controller.index()].life += life;
                }
            }
            CardBehavior::RedElementalBlast => match object.targets.first() {
                Some(Target::Spell(target)) => self.counter_spell(*target),
                Some(Target::Permanent(target)) => self.destroy_permanent(*target),
                Some(Target::Player(_)) | None => {}
            },
            CardBehavior::BlueElementalBlast => match object.targets.first() {
                Some(Target::Spell(target)) => self.counter_spell(*target),
                Some(Target::Permanent(target)) => self.destroy_permanent(*target),
                Some(Target::Player(_)) | None => {}
            },
            CardBehavior::Detonate => {
                if let Some(Target::Permanent(target)) = object.targets.first()
                    && let Some(controller) = self.permanent_controller(*target)
                {
                    self.destroy_permanent(*target);
                    self.deal_damage(controller, object.x);
                }
            }
            CardBehavior::Fork => {
                if let Some(Target::Spell(target)) = object.targets.first()
                    && let Some(original) =
                        self.stack.iter().find(|item| item.id == *target).cloned()
                {
                    self.queue_fork_decision(object.controller, original);
                }
            }
            CardBehavior::WheelOfFortune => self.resolve_wheel_of_fortune(),
            CardBehavior::Timetwister => self.resolve_timetwister(),
            CardBehavior::TimeWalk => self.extra_turns.push(object.controller),
            CardBehavior::DarkRitual => self.players[object.controller.index()]
                .mana_pool
                .add_color(ManaColor::Black, 3),
            CardBehavior::Channel => self.channel_active[object.controller.index()] = true,
            CardBehavior::DemonicTutor => {
                let options = self.players[object.controller.index()]
                    .library
                    .iter()
                    .enumerate()
                    .map(|(index, card)| DecisionOption {
                        id: u32::try_from(index).unwrap_or(u32::MAX),
                        label: self
                            .catalog
                            .get(card.definition)
                            .map_or_else(|| "Unknown card".into(), |card| card.name.clone()),
                        card: Some((card.id, card.definition)),
                        zone: DecisionZone::Library,
                    })
                    .collect();
                self.queue_decision(
                    object.controller,
                    "Choose a card to put into your hand",
                    DecisionVisibility::Private,
                    DecisionPreference::HigherCardValue,
                    1..=1,
                    false,
                    options,
                    DecisionContinuation::Tutor,
                );
            }
            CardBehavior::HymnToTourach => self.discard_random(object.controller.opponent(), 2),
            CardBehavior::MindTwist => self.discard_random(object.controller.opponent(), object.x),
            CardBehavior::Armageddon => self.destroy_all_matching(|kind| kind == CardKind::Land),
            CardBehavior::Balance => self.resolve_balance(),
            CardBehavior::Regrowth => {
                if let Some(card) = self.players[object.controller.index()].graveyard.pop() {
                    self.players[object.controller.index()].hand.push(card);
                }
            }
            CardBehavior::Recall => {
                let options = self.card_decision_options(
                    &self.players[object.controller.index()].graveyard,
                    DecisionZone::Graveyard,
                );
                let count = usize::from(object.x).min(options.len());
                self.queue_decision(
                    object.controller,
                    format!("Return {count} card(s) from your graveyard"),
                    DecisionVisibility::Private,
                    DecisionPreference::HigherCardValue,
                    count..=count,
                    false,
                    options,
                    DecisionContinuation::RecallReturn {
                        player: object.controller,
                    },
                );
            }
            _ => {}
        }
    }

    fn discard_random(&mut self, player: PlayerId, count: u16) {
        self.rng.shuffle(&mut self.players[player.index()].hand);
        for _ in 0..usize::from(count).min(self.players[player.index()].hand.len()) {
            if let Some(card) = self.players[player.index()].hand.pop() {
                self.players[player.index()].graveyard.push(card);
            }
        }
    }

    fn destroy_all_matching(&mut self, predicate: impl Fn(CardKind) -> bool) {
        let doomed = self
            .battlefield
            .iter()
            .filter(|permanent| {
                self.effective_behavior(permanent)
                    .map(CardBehavior::kind)
                    .is_some_and(&predicate)
            })
            .map(|permanent| permanent.card.id)
            .collect::<Vec<_>>();
        for permanent in doomed {
            self.destroy_permanent(permanent);
        }
    }

    fn resolve_balance(&mut self) {
        let mut tasks = Vec::new();
        for kind in [CardKind::Land, CardKind::Creature] {
            let counts = [PlayerId::One, PlayerId::Two].map(|player| {
                self.battlefield
                    .iter()
                    .filter(|permanent| {
                        permanent.controller == player
                            && if kind == CardKind::Creature {
                                self.power(permanent).is_some()
                            } else {
                                self.kind(permanent.card.definition) == Some(CardKind::Land)
                            }
                    })
                    .count()
            });
            let keep = counts[0].min(counts[1]);
            for player in [PlayerId::One, PlayerId::Two] {
                let cards = self
                    .battlefield
                    .iter()
                    .filter(|permanent| {
                        permanent.controller == player
                            && if kind == CardKind::Creature {
                                self.power(permanent).is_some()
                            } else {
                                self.kind(permanent.card.definition) == Some(CardKind::Land)
                            }
                    })
                    .map(|permanent| permanent.card.clone())
                    .collect::<Vec<_>>();
                let count = cards.len().saturating_sub(keep);
                if count > 0 {
                    tasks.push(BalanceTask {
                        player,
                        prompt: format!(
                            "Choose {count} {} to sacrifice to Balance",
                            if kind == CardKind::Land {
                                "land(s)"
                            } else {
                                "creature(s)"
                            }
                        ),
                        zone: DecisionZone::Battlefield,
                        cards,
                        count,
                        action: BalanceAction::Sacrifice,
                    });
                }
            }
        }
        let keep = self.players[0].hand.len().min(self.players[1].hand.len());
        for player in [PlayerId::One, PlayerId::Two] {
            let count = self.players[player.index()].hand.len().saturating_sub(keep);
            if count > 0 {
                tasks.push(BalanceTask {
                    player,
                    prompt: format!("Choose {count} card(s) to discard to Balance"),
                    zone: DecisionZone::Hand,
                    cards: self.players[player.index()].hand.clone(),
                    count,
                    action: BalanceAction::Discard,
                });
            }
        }
        if !tasks.is_empty() {
            let first = tasks.remove(0);
            self.queue_balance_task(first, tasks);
        }
    }

    fn resolve_timetwister(&mut self) {
        for player in [PlayerId::One, PlayerId::Two] {
            let state = &mut self.players[player.index()];
            state.library.append(&mut state.hand);
            state.library.append(&mut state.graveyard);
            self.rng.shuffle(&mut state.library);
        }
        for player in [PlayerId::One, PlayerId::Two] {
            self.draw_cards(player, 7);
        }
    }

    fn resolve_wheel_of_fortune(&mut self) {
        for player in [PlayerId::One, PlayerId::Two] {
            let state = &mut self.players[player.index()];
            state.graveyard.append(&mut state.hand);
        }
        let can_draw = [
            self.players[0].library.len() >= 7,
            self.players[1].library.len() >= 7,
        ];
        match can_draw {
            [false, false] => {
                self.finish(GameResult::Draw);
                return;
            }
            [false, true] => {
                self.finish(GameResult::Winner {
                    winner: PlayerId::Two,
                    reason: WinReason::OpponentTriedToDrawFromEmptyLibrary,
                });
                return;
            }
            [true, false] => {
                self.finish(GameResult::Winner {
                    winner: PlayerId::One,
                    reason: WinReason::OpponentTriedToDrawFromEmptyLibrary,
                });
                return;
            }
            [true, true] => {}
        }
        for player in [PlayerId::One, PlayerId::Two] {
            for _ in 0..7 {
                let card = self.players[player.index()]
                    .library
                    .pop()
                    .expect("library size was checked");
                let card_id = card.id;
                self.players[player.index()].hand.push(card);
                self.events.push(GameEvent::CardDrawn {
                    player,
                    card: card_id,
                });
            }
        }
    }

    fn damage_target(&mut self, target: Option<Target>, amount: u16) {
        match target {
            Some(Target::Player(player)) => self.deal_damage(player, amount),
            Some(Target::Permanent(id)) => {
                if let Some(permanent) = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == id)
                {
                    permanent.damage = permanent.damage.saturating_add(amount);
                }
            }
            Some(Target::Spell(_)) | None => {}
        }
    }

    fn damage_targets(&self) -> Vec<Target> {
        let mut targets = vec![Target::Player(PlayerId::One), Target::Player(PlayerId::Two)];
        targets.extend(
            self.battlefield
                .iter()
                .filter(|permanent| self.power(permanent).is_some())
                .map(|permanent| Target::Permanent(permanent.card.id)),
        );
        targets
    }

    fn count_behavior(&self, behavior: CardBehavior) -> u16 {
        u16::try_from(
            self.battlefield
                .iter()
                .filter(|permanent| {
                    if behavior == CardBehavior::BloodMoon {
                        self.behavior(permanent.card.definition) == Some(behavior)
                    } else {
                        self.effective_behavior(permanent) == Some(behavior)
                    }
                })
                .count(),
        )
        .unwrap_or(u16::MAX)
    }

    fn blood_moon_active(&self) -> bool {
        self.count_behavior(CardBehavior::BloodMoon) > 0
    }

    fn is_nonbasic_land(&self, permanent: &Permanent) -> bool {
        self.kind(permanent.card.definition) == Some(CardKind::Land)
            && self
                .catalog
                .get(permanent.card.definition)
                .is_some_and(|card| !card.is_basic_land)
    }

    fn is_artifact_permanent(&self, permanent: &Permanent) -> bool {
        self.effective_behavior(permanent)
            .is_some_and(|behavior| behavior.kind().is_artifact())
            || (permanent.factory_animated
                && self.behavior(permanent.card.definition) == Some(CardBehavior::MishrasFactory))
    }

    fn effective_behavior(&self, permanent: &Permanent) -> Option<CardBehavior> {
        if self.blood_moon_active() && self.is_nonbasic_land(permanent) {
            Some(CardBehavior::Mountain)
        } else {
            permanent
                .copied_behavior
                .or_else(|| self.behavior(permanent.card.definition))
        }
    }

    fn mana_colors(&self, permanent: &Permanent) -> Vec<ManaColor> {
        match self.effective_behavior(permanent) {
            Some(CardBehavior::Mountain | CardBehavior::MoxRuby) => vec![ManaColor::Red],
            Some(CardBehavior::Island | CardBehavior::MoxSapphire) => vec![ManaColor::Blue],
            Some(CardBehavior::Plains | CardBehavior::MoxPearl) => vec![ManaColor::White],
            Some(CardBehavior::Swamp | CardBehavior::MoxJet) => vec![ManaColor::Black],
            Some(CardBehavior::Forest | CardBehavior::MoxEmerald | CardBehavior::Pendelhaven) => {
                vec![ManaColor::Green]
            }
            Some(CardBehavior::Tundra) => vec![ManaColor::White, ManaColor::Blue],
            Some(CardBehavior::Badlands) => vec![ManaColor::Black, ManaColor::Red],
            Some(CardBehavior::Bayou) => vec![ManaColor::Black, ManaColor::Green],
            Some(CardBehavior::Plateau) => vec![ManaColor::White, ManaColor::Red],
            Some(CardBehavior::Savannah) => vec![ManaColor::White, ManaColor::Green],
            Some(CardBehavior::Scrubland) => vec![ManaColor::White, ManaColor::Black],
            Some(CardBehavior::Taiga) => vec![ManaColor::Red, ManaColor::Green],
            Some(CardBehavior::TropicalIsland) => vec![ManaColor::Blue, ManaColor::Green],
            Some(CardBehavior::UndergroundSea) => vec![ManaColor::Blue, ManaColor::Black],
            Some(
                CardBehavior::BlackLotus
                | CardBehavior::BirdsOfParadise
                | CardBehavior::CityOfBrass,
            ) => colored_mana(),
            Some(CardBehavior::LlanowarElves) => vec![ManaColor::Green],
            Some(CardBehavior::VolcanicIsland) => vec![ManaColor::Blue, ManaColor::Red],
            Some(
                CardBehavior::LibraryOfAlexandria
                | CardBehavior::MishrasFactory
                | CardBehavior::MishrasWorkshop
                | CardBehavior::StripMine
                | CardBehavior::SolRing
                | CardBehavior::ManaVault,
            ) => vec![ManaColor::Colorless],
            Some(CardBehavior::FellwarStone) => {
                let mut colors = self
                    .battlefield
                    .iter()
                    .filter(|candidate| {
                        candidate.controller == permanent.controller.opponent()
                            && self.kind(candidate.card.definition) == Some(CardKind::Land)
                    })
                    .flat_map(|candidate| self.mana_colors(candidate))
                    .filter(|color| *color != ManaColor::Colorless)
                    .collect::<Vec<_>>();
                colors.sort_unstable();
                colors.dedup();
                colors
            }
            _ => Vec::new(),
        }
    }

    fn mana_production(&self, permanent: &Permanent, color: ManaColor) -> Option<ManaPool> {
        if !self.mana_colors(permanent).contains(&color) {
            return None;
        }
        let amount = match self.effective_behavior(permanent) {
            Some(
                CardBehavior::BlackLotus | CardBehavior::ManaVault | CardBehavior::MishrasWorkshop,
            ) => 3,
            Some(CardBehavior::SolRing) => 2,
            _ => 1,
        };
        let mut pool = ManaPool::default();
        pool.add_color(color, amount);
        Some(pool)
    }

    fn can_pay_cost(&self, player: PlayerId, cost: ManaCost, x: u16) -> bool {
        let mut fixed = self.players[player.index()].mana_pool;
        let mut flexible = Vec::new();
        for permanent in self.battlefield.iter().filter(|permanent| {
            permanent.controller == player
                && !permanent.tapped
                && self.can_use_tap_ability(permanent)
        }) {
            let outputs = self
                .mana_colors(permanent)
                .into_iter()
                .filter_map(|color| self.mana_production(permanent, color))
                .collect::<Vec<_>>();
            if outputs.is_empty() {
                continue;
            }
            if outputs.len() == 1 {
                fixed.add(outputs[0]);
            } else {
                flexible.push(outputs);
            }
        }
        flexible_can_pay(&flexible, 0, fixed, cost, x)
    }

    /// Returns the mana sources the engine's default payment policy would tap
    /// for an action. This is a read-only preview for clients; applying the
    /// action still performs the authoritative payment and validation.
    #[must_use]
    pub fn mana_sources_for_action(
        &self,
        player: PlayerId,
        action: &Action,
    ) -> Vec<CardInstanceId> {
        let Some((cost, x, avoid)) = self.mana_requirement(action) else {
            return Vec::new();
        };
        self.plan_mana_sources(player, cost, x, avoid)
    }

    fn mana_requirement(&self, action: &Action) -> Option<(ManaCost, u16, Option<CardInstanceId>)> {
        match action {
            Action::CastSpell {
                card, targets, x, ..
            } => {
                let behavior = self
                    .players
                    .iter()
                    .flat_map(|player| &player.hand)
                    .find(|candidate| candidate.id == *card)
                    .and_then(|candidate| self.behavior(candidate.definition))?;
                Some((
                    add_generic(
                        behavior.mana_cost(),
                        fireball_extra_cost(behavior, targets.len()),
                    ),
                    *x,
                    None,
                ))
            }
            Action::ActivateAbility { source, target, .. } => {
                let behavior = self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == *source)
                    .and_then(|permanent| self.effective_behavior(permanent))?;
                let cost = match behavior {
                    CardBehavior::MishrasFactory if target.is_none() => ManaCost::new(1, 0),
                    CardBehavior::ChaosOrb
                    | CardBehavior::NevinyrralsDisk
                    | CardBehavior::IcyManipulator => ManaCost::new(1, 0),
                    CardBehavior::SedgeTroll => ManaCost::colored(0, 0, 0, 0, 1, 0),
                    CardBehavior::JayemdaeTome => ManaCost::new(4, 0),
                    _ => return None,
                };
                let avoid = (behavior == CardBehavior::MishrasFactory).then_some(*source);
                Some((cost, 0, avoid))
            }
            _ => None,
        }
    }

    fn plan_mana_sources(
        &self,
        player: PlayerId,
        cost: ManaCost,
        x: u16,
        avoid: Option<CardInstanceId>,
    ) -> Vec<CardInstanceId> {
        let mut pool = self.players[player.index()].mana_pool;
        let mut selected = Vec::new();
        for color in colored_mana() {
            let required = mana_cost_amount(cost, color);
            while pool.amount(color) < required {
                let Some((source, _)) = self
                    .mana_source_candidates(player, &selected)
                    .filter_map(|permanent| {
                        let colors = self.mana_colors(permanent);
                        colors
                            .contains(&color)
                            .then_some((permanent.card.id, colors.len()))
                    })
                    .min_by_key(|(source, flexibility)| (Some(*source) == avoid, *flexibility))
                else {
                    return Vec::new();
                };
                let Some(permanent) = self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == source)
                else {
                    return Vec::new();
                };
                let Some(production) = self.mana_production(permanent, color) else {
                    return Vec::new();
                };
                pool.add(production);
                selected.push(source);
            }
        }

        let required_total = colored_cost_total(cost)
            .saturating_add(cost.generic)
            .saturating_add(x.saturating_mul(cost.x_multiplier));
        while pool.total() < required_total {
            let Some((source, _color, production)) = self
                .mana_source_candidates(player, &selected)
                .filter_map(|permanent| {
                    let colors = self.mana_colors(permanent);
                    let color = if colors.contains(&ManaColor::Colorless) {
                        ManaColor::Colorless
                    } else {
                        *colors.first()?
                    };
                    let production = self.mana_production(permanent, color)?;
                    Some((permanent.card.id, color, production))
                })
                .min_by_key(|(source, color, production)| {
                    (
                        Some(*source) == avoid,
                        *color != ManaColor::Colorless,
                        production.total(),
                    )
                })
            else {
                return Vec::new();
            };
            pool.add(production);
            selected.push(source);
        }
        selected
    }

    fn mana_source_candidates<'a>(
        &'a self,
        player: PlayerId,
        selected: &'a [CardInstanceId],
    ) -> impl Iterator<Item = &'a Permanent> {
        self.battlefield.iter().filter(move |permanent| {
            permanent.controller == player
                && !permanent.tapped
                && !selected.contains(&permanent.card.id)
                && self.can_use_tap_ability(permanent)
        })
    }

    fn maximum_x(&self, player: PlayerId, cost: ManaCost) -> u16 {
        let maximum = self.players[player.index()]
            .mana_pool
            .total()
            .saturating_add(
                self.battlefield
                    .iter()
                    .filter(|permanent| {
                        permanent.controller == player
                            && !permanent.tapped
                            && self.can_use_tap_ability(permanent)
                    })
                    .filter_map(|permanent| {
                        self.mana_colors(permanent)
                            .first()
                            .and_then(|color| self.mana_production(permanent, *color))
                    })
                    .map(ManaPool::total)
                    .sum(),
            );
        (0..=maximum)
            .rev()
            .find(|x| self.can_pay_cost(player, cost, *x))
            .unwrap_or(0)
    }

    fn activate_mana_for_cost(&mut self, player: PlayerId, cost: ManaCost, x: u16) {
        self.activate_mana_for_cost_avoiding(player, cost, x, None);
    }

    fn activate_mana_for_cost_avoiding(
        &mut self,
        player: PlayerId,
        cost: ManaCost,
        x: u16,
        avoid: Option<CardInstanceId>,
    ) {
        for color in colored_mana() {
            let required = mana_cost_amount(cost, color);
            while self.players[player.index()].mana_pool.amount(color) < required {
                let (source, _) = self
                    .battlefield
                    .iter()
                    .filter(|permanent| {
                        permanent.controller == player
                            && !permanent.tapped
                            && self.can_use_tap_ability(permanent)
                    })
                    .filter_map(|permanent| {
                        let colors = self.mana_colors(permanent);
                        colors
                            .contains(&color)
                            .then_some((permanent.card.id, colors.len()))
                    })
                    .min_by_key(|(source, flexibility)| (Some(*source) == avoid, *flexibility))
                    .expect("legal spell has enough colored mana sources");
                self.activate_mana_source(player, source, color);
            }
        }

        let required_total = colored_cost_total(cost)
            .saturating_add(cost.generic)
            .saturating_add(x.saturating_mul(cost.x_multiplier));
        while self.players[player.index()].mana_pool.total() < required_total {
            let (source, color) = self
                .battlefield
                .iter()
                .filter(|permanent| {
                    permanent.controller == player
                        && !permanent.tapped
                        && self.can_use_tap_ability(permanent)
                })
                .filter_map(|permanent| {
                    let colors = self.mana_colors(permanent);
                    let color = if colors.contains(&ManaColor::Colorless) {
                        ManaColor::Colorless
                    } else {
                        *colors.first()?
                    };
                    let production = self.mana_production(permanent, color)?;
                    Some((permanent.card.id, color, production.total()))
                })
                .min_by_key(|(source, color, amount)| {
                    (
                        Some(*source) == avoid,
                        *color != ManaColor::Colorless,
                        *amount,
                    )
                })
                .map(|(source, color, _)| (source, color))
                .expect("legal spell has enough mana sources");
            self.activate_mana_source(player, source, color);
        }
    }

    fn base_stats(&self, permanent: &Permanent) -> Option<crate::CreatureStats> {
        let behavior = self.effective_behavior(permanent);
        if behavior == Some(CardBehavior::MishrasFactory) && permanent.factory_animated {
            Some(crate::CreatureStats {
                power: 2,
                toughness: 2,
                haste: false,
                trample: false,
            })
        } else {
            behavior.and_then(CardBehavior::creature_stats)
        }
    }

    fn controls_basic_land(&self, player: PlayerId, behavior: CardBehavior) -> bool {
        self.battlefield.iter().any(|permanent| {
            permanent.controller == player && self.effective_behavior(permanent) == Some(behavior)
        })
    }

    fn goblin_bonus(&self, permanent: &Permanent) -> i16 {
        let Some(behavior) = self.effective_behavior(permanent) else {
            return 0;
        };
        if !behavior.is_goblin() {
            return 0;
        }
        let kings = self
            .battlefield
            .iter()
            .filter(|candidate| {
                candidate.controller == permanent.controller
                    && candidate.card.id != permanent.card.id
                    && self.effective_behavior(candidate) == Some(CardBehavior::GoblinKing)
            })
            .count();
        i16::try_from(kings).unwrap_or(i16::MAX)
    }

    fn crusade_bonus(&self, permanent: &Permanent) -> i16 {
        if !self
            .effective_behavior(permanent)
            .is_some_and(CardBehavior::is_white)
        {
            return 0;
        }
        i16::try_from(self.count_behavior(CardBehavior::Crusade)).unwrap_or(i16::MAX)
    }

    fn power(&self, permanent: &Permanent) -> Option<i16> {
        self.base_stats(permanent).map(|stats| {
            let conditional_bonus = match self.effective_behavior(permanent) {
                Some(CardBehavior::KirdApe)
                    if self.controls_basic_land(permanent.controller, CardBehavior::Forest) =>
                {
                    1
                }
                Some(CardBehavior::SedgeTroll)
                    if self.controls_basic_land(permanent.controller, CardBehavior::Swamp) =>
                {
                    1
                }
                _ => 0,
            };
            stats.power
                + permanent.power_bonus
                + self.goblin_bonus(permanent)
                + self.crusade_bonus(permanent)
                + conditional_bonus
                + i16::try_from(permanent.plus_one_counters).unwrap_or(i16::MAX)
        })
    }

    fn toughness(&self, permanent: &Permanent) -> Option<i16> {
        self.base_stats(permanent).map(|stats| {
            let conditional_bonus = match self.effective_behavior(permanent) {
                Some(CardBehavior::KirdApe)
                    if self.controls_basic_land(permanent.controller, CardBehavior::Forest) =>
                {
                    2
                }
                Some(CardBehavior::SedgeTroll)
                    if self.controls_basic_land(permanent.controller, CardBehavior::Swamp) =>
                {
                    1
                }
                _ => 0,
            };
            stats.toughness
                + permanent.toughness_bonus
                + self.goblin_bonus(permanent)
                + self.crusade_bonus(permanent)
                + conditional_bonus
                + i16::try_from(permanent.plus_one_counters).unwrap_or(i16::MAX)
        })
    }

    fn has_flying(&self, permanent: &Permanent) -> bool {
        permanent.flying_until_end
            || self
                .effective_behavior(permanent)
                .is_some_and(CardBehavior::has_flying)
    }

    fn has_trample(&self, permanent: &Permanent) -> bool {
        permanent.trample_until_end
            || self
                .base_stats(permanent)
                .is_some_and(|stats| stats.trample)
    }

    fn has_mountainwalk(&self, permanent: &Permanent) -> bool {
        let printed = self
            .effective_behavior(permanent)
            .is_some_and(CardBehavior::has_mountainwalk);
        let king = self
            .effective_behavior(permanent)
            .is_some_and(CardBehavior::is_goblin)
            && self.battlefield.iter().any(|candidate| {
                candidate.controller == permanent.controller
                    && candidate.card.id != permanent.card.id
                    && self.effective_behavior(candidate) == Some(CardBehavior::GoblinKing)
            });
        printed || king
    }

    fn controls_mountain(&self, player: PlayerId) -> bool {
        self.battlefield.iter().any(|permanent| {
            permanent.controller == player
                && self.effective_behavior(permanent) == Some(CardBehavior::Mountain)
        })
    }

    fn can_use_tap_ability(&self, permanent: &Permanent) -> bool {
        self.base_stats(permanent)
            .is_none_or(|stats| stats.haste || permanent.entered_turn < self.turn)
    }

    #[allow(clippy::too_many_lines)]
    fn activate_ability(
        &mut self,
        player: PlayerId,
        source: CardInstanceId,
        target: Option<Target>,
        sacrifice: Option<CardInstanceId>,
    ) {
        let behavior = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == source)
            .and_then(|permanent| self.effective_behavior(permanent));
        match behavior {
            Some(CardBehavior::Atog) => {
                if let Some(sacrificed) = sacrifice {
                    self.sacrifice_permanent(sacrificed);
                    if let Some(atog) = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == source)
                    {
                        atog.power_bonus += 2;
                        atog.toughness_bonus += 2;
                    }
                }
            }
            Some(CardBehavior::GlassesOfUrza) => {
                if let Some(permanent) = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                {
                    permanent.tapped = true;
                }
                if let Some(Target::Player(target)) = target {
                    self.last_seen_hands[player.index()] =
                        Some((target, public_cards(&self.players[target.index()].hand)));
                }
            }
            Some(
                CardBehavior::IcyManipulator
                | CardBehavior::RelicBarrier
                | CardBehavior::Pendelhaven,
            ) => {
                let cost = if behavior == Some(CardBehavior::IcyManipulator) {
                    ManaCost::new(1, 0)
                } else {
                    ManaCost::new(0, 0)
                };
                if cost.generic > 0 {
                    self.activate_mana_for_cost(player, cost, 0);
                    pay_cost(&mut self.players[player.index()].mana_pool, cost, 0);
                }
                let card = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                    .map(|permanent| {
                        permanent.tapped = true;
                        permanent.card.clone()
                    })
                    .expect("legal tap ability has a source");
                let stack_id = StackObjectId(self.next_stack_id);
                self.next_stack_id += 1;
                self.stack.push(StackObject {
                    id: stack_id,
                    kind: StackObjectKind::ActivatedAbility,
                    card,
                    controller: player,
                    targets: target.into_iter().collect(),
                    chosen_permanents: Vec::new(),
                    x: 0,
                    is_copy: false,
                });
                self.events.push(GameEvent::AbilityActivated {
                    player,
                    source,
                    chosen_permanents: Vec::new(),
                });
            }
            Some(CardBehavior::SageOfLatNam) => {
                let card = self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == source)
                    .map(|permanent| permanent.card.clone())
                    .expect("legal Sage of Lat-Nam activation has a source");
                if let Some(sacrificed) = sacrifice {
                    self.sacrifice_permanent(sacrificed);
                }
                if let Some(permanent) = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                {
                    permanent.tapped = true;
                }
                let stack_id = StackObjectId(self.next_stack_id);
                self.next_stack_id += 1;
                self.stack.push(StackObject {
                    id: stack_id,
                    kind: StackObjectKind::ActivatedAbility,
                    card,
                    controller: player,
                    targets: Vec::new(),
                    chosen_permanents: sacrifice.into_iter().collect(),
                    x: 0,
                    is_copy: false,
                });
                self.events.push(GameEvent::AbilityActivated {
                    player,
                    source,
                    chosen_permanents: sacrifice.into_iter().collect(),
                });
            }
            Some(CardBehavior::SedgeTroll) => {
                let cost = ManaCost::colored(0, 0, 0, 0, 1, 0);
                self.activate_mana_for_cost(player, cost, 0);
                pay_cost(&mut self.players[player.index()].mana_pool, cost, 0);
                let card = self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == source)
                    .map(|permanent| permanent.card.clone())
                    .expect("legal Sedge Troll activation has a source");
                let stack_id = StackObjectId(self.next_stack_id);
                self.next_stack_id += 1;
                self.stack.push(StackObject {
                    id: stack_id,
                    kind: StackObjectKind::ActivatedAbility,
                    card,
                    controller: player,
                    targets: Vec::new(),
                    chosen_permanents: Vec::new(),
                    x: 0,
                    is_copy: false,
                });
                self.events.push(GameEvent::AbilityActivated {
                    player,
                    source,
                    chosen_permanents: Vec::new(),
                });
            }
            Some(CardBehavior::StoneGiant) => {
                if let Some(permanent) = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                {
                    permanent.tapped = true;
                }
                if let Some(Target::Permanent(target)) = target
                    && let Some(creature) = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == target)
                {
                    creature.flying_until_end = true;
                    creature.destroy_at_end = true;
                }
            }
            Some(CardBehavior::GoblinBalloonBrigade) => {
                self.players[player.index()].mana_pool.red -= 1;
                if let Some(permanent) = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                {
                    permanent.flying_until_end = true;
                }
            }
            Some(CardBehavior::GraniteGargoyle) => {
                self.players[player.index()].mana_pool.red -= 1;
                if let Some(permanent) = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                {
                    permanent.toughness_bonus += 1;
                }
            }
            Some(CardBehavior::DragonWhelp) => {
                self.players[player.index()].mana_pool.red -= 1;
                if let Some(permanent) = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                {
                    permanent.power_bonus += 1;
                    permanent.dragon_whelp_activations =
                        permanent.dragon_whelp_activations.saturating_add(1);
                    if permanent.dragon_whelp_activations >= 4 {
                        permanent.destroy_at_end = true;
                    }
                }
            }
            Some(CardBehavior::MishrasFactory) => {
                if let Some(Target::Permanent(target)) = target {
                    if let Some(permanent) = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == source)
                    {
                        permanent.tapped = true;
                    }
                    if let Some(worker) = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == target)
                    {
                        worker.power_bonus += 1;
                        worker.toughness_bonus += 1;
                    }
                } else {
                    let cost = ManaCost::new(1, 0);
                    self.activate_mana_for_cost_avoiding(player, cost, 0, Some(source));
                    pay_cost(&mut self.players[player.index()].mana_pool, cost, 0);
                    if let Some(permanent) = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == source)
                    {
                        permanent.factory_animated = true;
                    }
                }
            }
            Some(CardBehavior::StripMine) => {
                if let Some(Target::Permanent(target)) = target {
                    let card = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == source)
                        .map(|permanent| {
                            permanent.tapped = true;
                            permanent.card.clone()
                        })
                        .expect("legal Strip Mine activation has a source");
                    self.sacrifice_permanent(source);
                    let chosen_permanents = vec![target];
                    let targets = vec![Target::Permanent(target)];
                    let stack_id = StackObjectId(self.next_stack_id);
                    self.next_stack_id += 1;
                    self.stack.push(StackObject {
                        id: stack_id,
                        kind: StackObjectKind::ActivatedAbility,
                        card,
                        controller: player,
                        targets,
                        chosen_permanents: Vec::new(),
                        x: 0,
                        is_copy: false,
                    });
                    self.events.push(GameEvent::AbilityActivated {
                        player,
                        source,
                        chosen_permanents,
                    });
                }
            }
            Some(CardBehavior::ChaosOrb) => {
                let cost = ManaCost::new(1, 0);
                self.activate_mana_for_cost(player, cost, 0);
                pay_cost(&mut self.players[player.index()].mana_pool, cost, 0);
                let card = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                    .map(|permanent| {
                        permanent.tapped = true;
                        permanent.card.clone()
                    })
                    .expect("legal Chaos Orb activation has a source");
                let chosen_permanents = match target {
                    Some(Target::Permanent(chosen)) => vec![chosen],
                    Some(Target::Player(_) | Target::Spell(_)) | None => Vec::new(),
                };
                let stack_id = StackObjectId(self.next_stack_id);
                self.next_stack_id += 1;
                self.stack.push(StackObject {
                    id: stack_id,
                    kind: StackObjectKind::ActivatedAbility,
                    card,
                    controller: player,
                    targets: Vec::new(),
                    chosen_permanents: chosen_permanents.clone(),
                    x: 0,
                    is_copy: false,
                });
                self.events.push(GameEvent::AbilityActivated {
                    player,
                    source,
                    chosen_permanents,
                });
            }
            Some(CardBehavior::OrcishMechanics) => {
                let card = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                    .map(|permanent| {
                        permanent.tapped = true;
                        permanent.card.clone()
                    })
                    .expect("legal Orcish Mechanics activation has a source");
                if let Some(sacrificed) = sacrifice {
                    self.sacrifice_permanent(sacrificed);
                }
                let targets = target.into_iter().collect();
                let chosen_permanents: Vec<_> = sacrifice.into_iter().collect();
                let stack_id = StackObjectId(self.next_stack_id);
                self.next_stack_id += 1;
                self.stack.push(StackObject {
                    id: stack_id,
                    kind: StackObjectKind::ActivatedAbility,
                    card,
                    controller: player,
                    targets,
                    chosen_permanents: chosen_permanents.clone(),
                    x: 0,
                    is_copy: false,
                });
                self.events.push(GameEvent::AbilityActivated {
                    player,
                    source,
                    chosen_permanents,
                });
            }
            Some(CardBehavior::Triskelion) => {
                let card = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                    .map(|permanent| {
                        permanent.plus_one_counters -= 1;
                        permanent.card.clone()
                    })
                    .expect("legal Triskelion activation has a source");
                let targets = target.into_iter().collect();
                let stack_id = StackObjectId(self.next_stack_id);
                self.next_stack_id += 1;
                self.stack.push(StackObject {
                    id: stack_id,
                    kind: StackObjectKind::ActivatedAbility,
                    card,
                    controller: player,
                    targets,
                    chosen_permanents: Vec::new(),
                    x: 0,
                    is_copy: false,
                });
                self.events.push(GameEvent::AbilityActivated {
                    player,
                    source,
                    chosen_permanents: Vec::new(),
                });
            }
            Some(CardBehavior::JayemdaeTome) => {
                let cost = ManaCost::new(4, 0);
                self.activate_mana_for_cost(player, cost, 0);
                pay_cost(&mut self.players[player.index()].mana_pool, cost, 0);
                let card = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                    .map(|permanent| {
                        permanent.tapped = true;
                        permanent.card.clone()
                    })
                    .expect("legal Jayemdae Tome activation has a source");
                let stack_id = StackObjectId(self.next_stack_id);
                self.next_stack_id += 1;
                self.stack.push(StackObject {
                    id: stack_id,
                    kind: StackObjectKind::ActivatedAbility,
                    card,
                    controller: player,
                    targets: Vec::new(),
                    chosen_permanents: Vec::new(),
                    x: 0,
                    is_copy: false,
                });
                self.events.push(GameEvent::AbilityActivated {
                    player,
                    source,
                    chosen_permanents: Vec::new(),
                });
            }
            Some(
                behavior @ (CardBehavior::LibraryOfAlexandria
                | CardBehavior::MazeOfIth
                | CardBehavior::NevinyrralsDisk
                | CardBehavior::IcatianJavelineers
                | CardBehavior::TimeVault),
            ) => {
                if behavior == CardBehavior::NevinyrralsDisk {
                    let cost = ManaCost::new(1, 0);
                    self.activate_mana_for_cost(player, cost, 0);
                    pay_cost(&mut self.players[player.index()].mana_pool, cost, 0);
                }
                let card = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                    .map(|permanent| {
                        permanent.tapped = true;
                        if behavior == CardBehavior::IcatianJavelineers {
                            permanent.plus_one_counters -= 1;
                        }
                        permanent.card.clone()
                    })
                    .expect("legal activation has a source");
                let stack_id = StackObjectId(self.next_stack_id);
                self.next_stack_id += 1;
                self.stack.push(StackObject {
                    id: stack_id,
                    kind: StackObjectKind::ActivatedAbility,
                    card,
                    controller: player,
                    targets: target.into_iter().collect(),
                    chosen_permanents: Vec::new(),
                    x: 0,
                    is_copy: false,
                });
                self.events.push(GameEvent::AbilityActivated {
                    player,
                    source,
                    chosen_permanents: Vec::new(),
                });
            }
            _ => {}
        }
        self.consecutive_passes = 0;
        self.check_state_based_actions();
    }

    fn attacker_actions(&self, player: PlayerId) -> Vec<Action> {
        self.battlefield
            .iter()
            .filter(|permanent| {
                permanent.controller == player
                    && !permanent.tapped
                    && !permanent.attacking
                    && self.power(permanent).is_some()
                    && self.can_attack(permanent)
            })
            .map(|permanent| Action::DeclareAttacker {
                attacker: permanent.card.id,
            })
            .collect()
    }

    fn can_attack(&self, permanent: &Permanent) -> bool {
        if self.count_behavior(CardBehavior::Moat) > 0 && !self.has_flying(permanent) {
            return false;
        }
        self.base_stats(permanent)
            .is_some_and(|stats| stats.haste || permanent.entered_turn < self.turn)
    }

    fn declare_attacker(&mut self, attacker: CardInstanceId) {
        let vigilance = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == attacker)
            .and_then(|permanent| self.effective_behavior(permanent))
            .is_some_and(CardBehavior::has_vigilance);
        if let Some(permanent) = self
            .battlefield
            .iter_mut()
            .find(|permanent| permanent.card.id == attacker)
        {
            permanent.attacking = true;
            permanent.attacked_this_turn = true;
            if !vigilance {
                permanent.tapped = true;
            }
        }
    }

    fn finish_declaring_attackers(&mut self) {
        self.attackers_declared = true;
        self.priority = self.active_player;
        self.consecutive_passes = 0;
        let attackers = self
            .battlefield
            .iter()
            .filter(|permanent| permanent.controller == self.active_player && permanent.attacking)
            .map(|permanent| permanent.card.id)
            .collect::<Vec<_>>();
        if !attackers.is_empty() {
            self.events.push(GameEvent::AttackDeclared {
                player: self.active_player,
                attackers,
            });
        }
    }

    fn blocker_actions(&self, player: PlayerId) -> Vec<Action> {
        let blockers: Vec<_> = self
            .battlefield
            .iter()
            .filter(|permanent| {
                permanent.controller == player
                    && !permanent.tapped
                    && permanent.blocking.is_none()
                    && self.power(permanent).is_some()
            })
            .map(|permanent| permanent.card.id)
            .collect();
        let attackers: Vec<_> = self
            .battlefield
            .iter()
            .filter(|permanent| permanent.attacking)
            .map(|permanent| {
                (
                    permanent.card.id,
                    self.has_flying(permanent),
                    self.has_mountainwalk(permanent)
                        && self.controls_mountain(permanent.controller.opponent()),
                    self.power(permanent).unwrap_or(0),
                )
            })
            .collect();
        blockers
            .into_iter()
            .flat_map(|blocker| {
                let blocker_permanent = self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == blocker)
                    .expect("blocker is on the battlefield");
                let blocker_flying = self.has_flying(blocker_permanent);
                let ironclaw =
                    self.effective_behavior(blocker_permanent) == Some(CardBehavior::IronclawOrcs);
                attackers
                    .iter()
                    .filter_map(move |(attacker, flying, unblockable, power)| {
                        let pixies = self
                            .battlefield
                            .iter()
                            .find(|permanent| permanent.card.id == *attacker)
                            .is_some_and(|permanent| {
                                self.effective_behavior(permanent)
                                    == Some(CardBehavior::ArgothianPixies)
                            });
                        let can_block = !(*unblockable
                            || *flying && !blocker_flying
                            || ironclaw && *power >= 2
                            || pixies && self.is_artifact_permanent(blocker_permanent));
                        can_block.then_some(Action::DeclareBlocker {
                            blocker,
                            attacker: *attacker,
                        })
                    })
            })
            .collect()
    }

    fn declare_blocker(&mut self, blocker: CardInstanceId, attacker: CardInstanceId) {
        if let Some(permanent) = self
            .battlefield
            .iter_mut()
            .find(|permanent| permanent.card.id == blocker)
        {
            permanent.blocking = Some(attacker);
        }
    }

    fn finish_declaring_blockers(&mut self) {
        self.blockers_declared = true;
        self.priority = self.active_player;
        self.consecutive_passes = 0;
        let assignments = self
            .battlefield
            .iter()
            .filter_map(|permanent| {
                permanent
                    .blocking
                    .map(|attacker| (permanent.card.id, attacker))
            })
            .collect::<Vec<_>>();
        if !assignments.is_empty() {
            self.events.push(GameEvent::BlockDeclared {
                player: self.active_player.opponent(),
                assignments,
            });
        }
    }

    fn begin_combat_damage_assignment(&mut self) {
        self.pending_combat_attackers = self
            .battlefield
            .iter()
            .filter(|attacker| attacker.attacking)
            .filter(|attacker| {
                let blocker_count = self
                    .battlefield
                    .iter()
                    .filter(|blocker| blocker.blocking == Some(attacker.card.id))
                    .count();
                let trample = self.has_trample(attacker);
                blocker_count > 1 || (trample && blocker_count > 0)
            })
            .map(|attacker| attacker.card.id)
            .collect();
        if self.pending_combat_attackers.is_empty() {
            self.deal_combat_damage();
        }
    }

    fn combat_assignment_actions(&self, attacker_id: CardInstanceId) -> Vec<Action> {
        let Some(attacker) = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == attacker_id)
        else {
            return Vec::new();
        };
        let power = self.power(attacker).unwrap_or(0).max(0).cast_unsigned();
        let trample = self.has_trample(attacker);
        let mut recipients: Vec<_> = self
            .battlefield
            .iter()
            .filter(|permanent| permanent.blocking == Some(attacker_id))
            .map(|permanent| Target::Permanent(permanent.card.id))
            .collect();
        recipients.sort_unstable();
        if trample {
            recipients.push(Target::Player(self.active_player.opponent()));
        }

        damage_distributions(recipients.len(), power)
            .into_iter()
            .filter(|amounts| {
                if !trample || amounts.last().copied().unwrap_or(0) == 0 {
                    return true;
                }
                recipients
                    .iter()
                    .zip(amounts)
                    .filter_map(|(target, amount)| match target {
                        Target::Permanent(id) => Some((*id, *amount)),
                        Target::Player(_) | Target::Spell(_) => None,
                    })
                    .all(|(id, amount)| amount >= self.lethal_damage(id))
            })
            .map(|amounts| Action::AssignCombatDamage {
                attacker: attacker_id,
                assignments: recipients
                    .iter()
                    .copied()
                    .zip(amounts)
                    .map(|(recipient, amount)| CombatDamageAssignment { recipient, amount })
                    .collect(),
            })
            .collect()
    }

    fn lethal_damage(&self, permanent_id: CardInstanceId) -> u16 {
        self.battlefield
            .iter()
            .find(|permanent| permanent.card.id == permanent_id)
            .map_or(0, |permanent| {
                self.toughness(permanent)
                    .unwrap_or(0)
                    .max(0)
                    .cast_unsigned()
                    .saturating_sub(permanent.damage)
            })
    }

    fn assign_combat_damage(
        &mut self,
        attacker: CardInstanceId,
        assignments: Vec<CombatDamageAssignment>,
    ) {
        if let Some(permanent) = self
            .battlefield
            .iter_mut()
            .find(|permanent| permanent.card.id == attacker)
        {
            permanent.combat_damage_assignment = assignments;
        }
        self.pending_combat_attackers.remove(0);
        if self.pending_combat_attackers.is_empty() {
            self.deal_combat_damage();
        }
    }

    fn deal_combat_damage(&mut self) {
        let attackers: Vec<_> = self
            .battlefield
            .iter()
            .filter(|permanent| permanent.attacking)
            .map(|permanent| permanent.card.id)
            .collect();
        for attacker_id in attackers {
            let Some(attacker_index) = self
                .battlefield
                .iter()
                .position(|permanent| permanent.card.id == attacker_id)
            else {
                continue;
            };
            let power = self
                .power(&self.battlefield[attacker_index])
                .unwrap_or(0)
                .max(0)
                .cast_unsigned();
            let blockers: Vec<_> = self
                .battlefield
                .iter()
                .filter(|permanent| permanent.blocking == Some(attacker_id))
                .map(|permanent| permanent.card.id)
                .collect();
            if blockers.is_empty() {
                self.deal_damage(self.active_player.opponent(), power);
                match self.effective_behavior(&self.battlefield[attacker_index]) {
                    Some(CardBehavior::HypnoticSpecter) => {
                        self.discard_random(self.active_player.opponent(), 1);
                    }
                    Some(CardBehavior::WhirlingDervish) => {
                        self.battlefield[attacker_index].plus_one_counters = self.battlefield
                            [attacker_index]
                            .plus_one_counters
                            .saturating_add(1);
                    }
                    _ => {}
                }
            } else {
                let assignments = self.battlefield[attacker_index]
                    .combat_damage_assignment
                    .clone();
                if assignments.is_empty() {
                    self.damage_target(Some(Target::Permanent(blockers[0])), power);
                } else {
                    for assignment in assignments {
                        self.damage_target(Some(assignment.recipient), assignment.amount);
                    }
                }
                let return_damage: u16 = blockers
                    .iter()
                    .filter_map(|id| {
                        self.battlefield
                            .iter()
                            .find(|permanent| permanent.card.id == *id)
                            .and_then(|permanent| self.power(permanent))
                    })
                    .map(|value| value.max(0).cast_unsigned())
                    .sum();
                self.damage_target(Some(Target::Permanent(attacker_id)), return_damage);
            }
        }
        self.check_state_based_actions();
    }

    fn permanent_controller(&self, id: CardInstanceId) -> Option<PlayerId> {
        self.battlefield
            .iter()
            .find(|permanent| permanent.card.id == id)
            .map(|permanent| permanent.controller)
    }

    fn destroy_permanent(&mut self, id: CardInstanceId) {
        let Some(index) = self
            .battlefield
            .iter()
            .position(|permanent| permanent.card.id == id)
        else {
            return;
        };
        if self.battlefield[index].regeneration_shields > 0 {
            let permanent = &mut self.battlefield[index];
            permanent.regeneration_shields -= 1;
            permanent.tapped = true;
            permanent.damage = 0;
            permanent.attacking = false;
            permanent.blocking = None;
            permanent.combat_damage_assignment.clear();
            for other in &mut self.battlefield {
                if other.card.id != id && other.blocking == Some(id) {
                    other.blocking = None;
                }
            }
            return;
        }
        self.remove_permanent_to_graveyard(index);
    }

    fn destroy_permanent_without_regeneration(&mut self, id: CardInstanceId) {
        let Some(index) = self
            .battlefield
            .iter()
            .position(|permanent| permanent.card.id == id)
        else {
            return;
        };
        self.remove_permanent_to_graveyard(index);
    }

    fn sacrifice_permanent(&mut self, id: CardInstanceId) {
        self.destroy_permanent_without_regeneration(id);
    }

    fn remove_permanent_to_graveyard(&mut self, index: usize) {
        let permanent = self.battlefield.remove(index);
        if self.effective_behavior(&permanent) == Some(CardBehavior::SuChi) {
            self.players[permanent.controller.index()]
                .mana_pool
                .colorless += 4;
        }
        self.players[permanent.card.owner.index()]
            .graveyard
            .push(permanent.card);
    }

    fn exile_permanent(&mut self, id: CardInstanceId) {
        let Some(index) = self
            .battlefield
            .iter()
            .position(|permanent| permanent.card.id == id)
        else {
            return;
        };
        let permanent = self.battlefield.remove(index);
        self.players[permanent.card.owner.index()]
            .exile
            .push(permanent.card);
    }

    fn return_permanent_to_hand(&mut self, id: CardInstanceId) {
        let Some(index) = self
            .battlefield
            .iter()
            .position(|permanent| permanent.card.id == id)
        else {
            return;
        };
        let permanent = self.battlefield.remove(index);
        self.players[permanent.card.owner.index()]
            .hand
            .push(permanent.card);
    }

    fn counter_spell(&mut self, id: StackObjectId) {
        let Some(index) = self.stack.iter().position(|object| object.id == id) else {
            return;
        };
        let object = self.stack.remove(index);
        if !object.is_copy {
            self.players[object.card.owner.index()]
                .graveyard
                .push(object.card);
        }
    }

    fn check_state_based_actions(&mut self) {
        let dead: Vec<_> = self
            .battlefield
            .iter()
            .filter(|permanent| {
                self.toughness(permanent).is_some_and(|toughness| {
                    toughness <= 0 || i32::from(permanent.damage) >= i32::from(toughness)
                })
            })
            .map(|permanent| permanent.card.id)
            .collect();
        for id in dead {
            self.destroy_permanent(id);
        }
        self.check_life_totals();
    }

    fn untap_actions(&self, player: PlayerId) -> Vec<Action> {
        let lands: Vec<_> = self
            .battlefield
            .iter()
            .filter(|permanent| {
                permanent.controller == player
                    && permanent.tapped
                    && self.kind(permanent.card.definition) == Some(CardKind::Land)
            })
            .map(|permanent| permanent.card.id)
            .collect();
        let creatures: Vec<_> = self
            .battlefield
            .iter()
            .filter(|permanent| {
                permanent.controller == player
                    && permanent.tapped
                    && self.power(permanent).is_some()
            })
            .map(|permanent| permanent.card.id)
            .collect();
        let land_choices = if self.winter_orb_active() {
            one_or_none(&lands)
        } else {
            vec![lands]
        };
        let creature_choices = if self.count_behavior(CardBehavior::Smoke) > 0 {
            one_or_none(&creatures)
        } else {
            vec![creatures]
        };
        let mut actions = Vec::new();
        for land in &land_choices {
            for creature in &creature_choices {
                let mut permanents = land.clone();
                permanents.extend(creature);
                permanents.sort_unstable();
                permanents.dedup();
                actions.push(Action::ChooseUntap { permanents });
            }
        }
        actions
    }

    fn choose_untap(&mut self, player: PlayerId, selected: &[CardInstanceId]) {
        for permanent in &mut self.battlefield {
            if permanent.controller == player && selected.contains(&permanent.card.id) {
                permanent.tapped = false;
            }
        }
        self.untap_pending = false;
        self.priority = self.active_player;
        self.finish_untap_choices();
    }

    fn deal_damage(&mut self, player: PlayerId, amount: u16) {
        let amount_as_i16 = i16::try_from(amount).unwrap_or(i16::MAX);
        self.players[player.index()].life -= amount_as_i16;
        self.events.push(GameEvent::DamageDealt { player, amount });
    }

    fn advance_step(&mut self) {
        if self.step.ends_phase() {
            self.apply_mana_burn();
            if self.result.is_some() {
                return;
            }
        }

        match self.step {
            Step::Upkeep => {
                self.step = Step::Draw;
                let vault_damage = u16::try_from(
                    self.battlefield
                        .iter()
                        .filter(|permanent| {
                            permanent.controller == self.active_player
                                && permanent.tapped
                                && self.effective_behavior(permanent)
                                    == Some(CardBehavior::ManaVault)
                        })
                        .count(),
                )
                .unwrap_or(u16::MAX);
                if vault_damage > 0 {
                    self.deal_damage(self.active_player, vault_damage);
                    self.check_life_totals();
                    if self.result.is_some() {
                        return;
                    }
                }
                if !(self.turn == 1 && self.active_player == PlayerId::One) {
                    let mut drawn = self
                        .draw_card(self.active_player)
                        .into_iter()
                        .collect::<Vec<_>>();
                    if self.battlefield.iter().any(|permanent| {
                        permanent.controller == self.active_player
                            && self.effective_behavior(permanent)
                                == Some(CardBehavior::SylvanLibrary)
                    }) && self.result.is_none()
                    {
                        if let Some(card) = self.draw_card(self.active_player) {
                            drawn.push(card);
                        }
                        if let Some(card) = self.draw_card(self.active_player) {
                            drawn.push(card);
                        }
                        if drawn.len() >= 2 && self.result.is_none() {
                            self.queue_sylvan_select(self.active_player, drawn, 2);
                        }
                    }
                }
            }
            Step::Draw => {
                self.step = Step::PrecombatMain;
                let amount =
                    std::mem::take(&mut self.mana_drain_pending[self.active_player.index()]);
                self.players[self.active_player.index()]
                    .mana_pool
                    .add_color(ManaColor::Colorless, amount);
            }
            Step::PrecombatMain => self.step = Step::BeginningOfCombat,
            Step::BeginningOfCombat => {
                self.step = Step::DeclareAttackers;
                self.attackers_declared = false;
            }
            Step::DeclareAttackers => {
                self.step = Step::DeclareBlockers;
                self.blockers_declared = false;
            }
            Step::DeclareBlockers => {
                self.step = Step::CombatDamage;
                self.begin_combat_damage_assignment();
            }
            Step::CombatDamage => self.step = Step::EndOfCombat,
            Step::EndOfCombat => {
                self.clear_combat();
                self.step = Step::PostcombatMain;
            }
            Step::PostcombatMain => {
                self.step = Step::End;
                self.handle_end_step();
            }
            Step::End => {
                self.step = Step::Cleanup;
                self.cleanup();
            }
            Step::Cleanup => self.start_next_turn(),
        }

        if self.result.is_none() {
            self.priority = self.active_player;
            self.events.push(GameEvent::StepChanged {
                turn: self.turn,
                active_player: self.active_player,
                step: self.step,
            });
        }
    }

    fn start_next_turn(&mut self) {
        self.turn += 1;
        let mut next_player = self
            .extra_turns
            .pop()
            .unwrap_or_else(|| self.active_player.opponent());
        while self.skipped_turns[next_player.index()] > 0 {
            self.skipped_turns[next_player.index()] -= 1;
            let skipped = next_player;
            next_player = self.extra_turns.pop().unwrap_or_else(|| skipped.opponent());
        }
        self.active_player = next_player;
        self.turns_started[self.active_player.index()] += 1;
        self.step = Step::Upkeep;
        self.players[self.active_player.index()].land_played_this_turn = false;
        let winter_orb = self.winter_orb_active();
        let smoke = self.count_behavior(CardBehavior::Smoke) > 0;
        let restricted_lands: Vec<_> = self
            .battlefield
            .iter()
            .filter(|permanent| self.kind(permanent.card.definition) == Some(CardKind::Land))
            .map(|permanent| permanent.card.id)
            .collect();
        let restricted_creatures: Vec<_> = self
            .battlefield
            .iter()
            .filter(|permanent| self.power(permanent).is_some())
            .map(|permanent| permanent.card.id)
            .collect();
        let mana_vaults: Vec<_> = self
            .battlefield
            .iter()
            .filter(|permanent| {
                matches!(
                    self.effective_behavior(permanent),
                    Some(CardBehavior::ManaVault | CardBehavior::TimeVault)
                )
            })
            .map(|permanent| permanent.card.id)
            .collect();
        self.untap_pending = false;
        for permanent in &mut self.battlefield {
            if permanent.controller == self.active_player {
                let restricted = (winter_orb && restricted_lands.contains(&permanent.card.id))
                    || (smoke && restricted_creatures.contains(&permanent.card.id));
                if restricted && permanent.tapped {
                    self.untap_pending = true;
                } else if !mana_vaults.contains(&permanent.card.id) {
                    permanent.tapped = false;
                }
            }
        }
        if !self.untap_pending {
            self.finish_untap_choices();
        }
    }

    #[allow(clippy::too_many_lines)]
    fn handle_upkeep_triggers(&mut self) {
        let player = self.active_player;
        let self_damage = u16::try_from(
            self.battlefield
                .iter()
                .filter(|permanent| {
                    permanent.controller == player
                        && matches!(
                            self.effective_behavior(permanent),
                            Some(CardBehavior::JuzamDjinn | CardBehavior::SerendibEfreet)
                        )
                })
                .count(),
        )
        .unwrap_or(u16::MAX);
        if self_damage > 0 {
            self.deal_damage(player, self_damage);
        }
        let tower_count = self
            .battlefield
            .iter()
            .filter(|permanent| {
                permanent.controller == player
                    && self.effective_behavior(permanent) == Some(CardBehavior::IvoryTower)
            })
            .count();
        let tower_life = self.players[player.index()]
            .hand
            .len()
            .saturating_sub(4)
            .saturating_mul(tower_count);
        self.players[player.index()].life += i16::try_from(tower_life).unwrap_or(i16::MAX);
        let copper_damage = self.count_behavior(CardBehavior::CopperTablet);
        if copper_damage > 0 {
            self.deal_damage(player, copper_damage);
        }
        let vise_damage: u16 = self
            .battlefield
            .iter()
            .filter(|permanent| {
                self.effective_behavior(permanent) == Some(CardBehavior::BlackVise)
                    && permanent.chosen_player == Some(player)
            })
            .map(|_| {
                u16::try_from(self.players[player.index()].hand.len().saturating_sub(4))
                    .unwrap_or(u16::MAX)
            })
            .sum();
        if vise_damage > 0 {
            self.deal_damage(player, vise_damage);
        }
        if self.count_behavior(CardBehavior::TheAbyss) > 0 {
            let target = self
                .battlefield
                .iter()
                .filter(|permanent| {
                    self.power(permanent).is_some() && !self.is_artifact_permanent(permanent)
                })
                .map(|permanent| permanent.card.id)
                .min();
            if let Some(target) = target {
                self.destroy_permanent(target);
            }
        }
        if self.count_behavior(CardBehavior::EnergyFlux) > 0 {
            let artifacts: Vec<_> = self
                .battlefield
                .iter()
                .filter(|permanent| {
                    permanent.controller == player && self.is_artifact_permanent(permanent)
                })
                .map(|permanent| permanent.card.id)
                .collect();
            for artifact in artifacts {
                let cost = ManaCost::new(2, 0);
                if self.can_pay_cost(player, cost, 0) {
                    self.activate_mana_for_cost(player, cost, 0);
                    pay_cost(&mut self.players[player.index()].mana_pool, cost, 0);
                } else {
                    self.destroy_permanent(artifact);
                }
            }
        }
        if self.count_behavior(CardBehavior::CityInABottle) > 0 {
            let doomed: Vec<_> = self
                .battlefield
                .iter()
                .filter(|permanent| {
                    self.behavior(permanent.card.definition) != Some(CardBehavior::CityInABottle)
                        && self
                            .catalog
                            .get(permanent.card.definition)
                            .is_some_and(|card| card.set == CardSet::ArabianNights)
                })
                .map(|permanent| permanent.card.id)
                .collect();
            for permanent in doomed {
                self.destroy_permanent(permanent);
            }
        }
        let tapped_vaults: Vec<_> = self
            .battlefield
            .iter()
            .filter(|permanent| {
                permanent.controller == player
                    && permanent.tapped
                    && self.effective_behavior(permanent) == Some(CardBehavior::ManaVault)
            })
            .map(|permanent| permanent.card.id)
            .collect();
        for permanent in tapped_vaults {
            self.queue_mana_vault_decision(player, permanent);
        }
        self.check_life_totals();
    }

    fn handle_end_step(&mut self) {
        let doomed: Vec<_> = self
            .battlefield
            .iter()
            .filter(|permanent| {
                permanent.destroy_at_end
                    || permanent.berserked && permanent.attacked_this_turn
                    || self.effective_behavior(permanent) == Some(CardBehavior::BallLightning)
            })
            .map(|permanent| permanent.card.id)
            .collect();
        for id in doomed {
            self.destroy_permanent(id);
        }
    }

    fn cleanup(&mut self) {
        if self.players[self.active_player.index()].hand.len() > 7 {
            self.cleanup_pending = true;
        } else {
            self.complete_cleanup();
        }
    }

    fn complete_cleanup(&mut self) {
        self.channel_active[self.active_player.index()] = false;
        self.finish_cleanup();
        self.apply_mana_burn();
        if self.result.is_none() {
            self.start_next_turn();
        }
    }

    fn finish_cleanup(&mut self) {
        for permanent in &mut self.battlefield {
            permanent.damage = 0;
            permanent.power_bonus = 0;
            permanent.toughness_bonus = 0;
            permanent.flying_until_end = false;
            permanent.destroy_at_end = false;
            permanent.factory_animated = false;
            permanent.dragon_whelp_activations = 0;
            permanent.regeneration_shields = 0;
            permanent.trample_until_end = false;
            permanent.berserked = false;
            permanent.attacked_this_turn = false;
        }
    }

    fn clear_combat(&mut self) {
        for permanent in &mut self.battlefield {
            permanent.attacking = false;
            permanent.blocking = None;
            permanent.combat_damage_assignment.clear();
        }
    }

    fn winter_orb_active(&self) -> bool {
        self.battlefield.iter().any(|permanent| {
            !permanent.tapped && self.effective_behavior(permanent) == Some(CardBehavior::WinterOrb)
        })
    }

    fn draw_card(&mut self, player: PlayerId) -> Option<CardInstanceId> {
        let Some(card) = self.players[player.index()].library.pop() else {
            self.finish(GameResult::Winner {
                winner: player.opponent(),
                reason: WinReason::OpponentTriedToDrawFromEmptyLibrary,
            });
            return None;
        };
        let card_id = card.id;
        self.players[player.index()].hand.push(card);
        self.events.push(GameEvent::CardDrawn {
            player,
            card: card_id,
        });
        Some(card_id)
    }

    fn draw_cards(&mut self, player: PlayerId, count: u16) {
        for _ in 0..count {
            if self.result.is_some() {
                break;
            }
            let _ = self.draw_card(player);
        }
    }

    fn apply_mana_burn(&mut self) {
        for player in [PlayerId::One, PlayerId::Two] {
            let amount = self.players[player.index()].mana_pool.total();
            self.players[player.index()].mana_pool = ManaPool::default();
            if amount > 0 {
                let amount_as_i16 = i16::try_from(amount).unwrap_or(i16::MAX);
                self.players[player.index()].life -= amount_as_i16;
                self.events.push(GameEvent::ManaBurn { player, amount });
            }
        }
        self.check_life_totals();
    }

    fn check_life_totals(&mut self) {
        let one_lost = self.players[0].life <= 0;
        let two_lost = self.players[1].life <= 0;
        match (one_lost, two_lost) {
            (true, true) => self.finish(GameResult::Draw),
            (true, false) => self.finish(GameResult::Winner {
                winner: PlayerId::Two,
                reason: WinReason::OpponentLostAllLife,
            }),
            (false, true) => self.finish(GameResult::Winner {
                winner: PlayerId::One,
                reason: WinReason::OpponentLostAllLife,
            }),
            (false, false) => {}
        }
    }

    fn finish(&mut self, result: GameResult) {
        self.result = Some(result);
        self.events.push(GameEvent::GameEnded { result });
    }
}

fn remove_card(cards: &mut Vec<CardInstance>, id: CardInstanceId) -> Option<CardInstance> {
    cards
        .iter()
        .position(|card| card.id == id)
        .map(|index| cards.remove(index))
}

fn public_cards(cards: &[CardInstance]) -> Vec<PublicCard> {
    cards
        .iter()
        .map(|card| (card.id, card.definition))
        .collect()
}

fn draw_opening_hand(library: &mut Vec<CardInstance>) -> Result<Vec<CardInstance>, GameError> {
    if library.len() < rules::OPENING_HAND_SIZE {
        return Err(GameError::NotEnoughCardsForOpeningHand);
    }
    let split_at = library.len() - rules::OPENING_HAND_SIZE;
    Ok(library.split_off(split_at))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameError {
    InvalidDeck { player: PlayerId, error: DeckError },
    TooManyCards,
    NotEnoughCardsForOpeningHand,
}

impl fmt::Display for GameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDeck { player, error } => {
                write!(formatter, "invalid deck for {player}: {error}")
            }
            Self::TooManyCards => formatter.write_str("game contains too many card instances"),
            Self::NotEnoughCardsForOpeningHand => {
                formatter.write_str("deck cannot provide a seven-card opening hand")
            }
        }
    }
}

impl Error for GameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidDeck { error, .. } => Some(error),
            Self::TooManyCards | Self::NotEnoughCardsForOpeningHand => None,
        }
    }
}

fn can_pay(pool: ManaPool, cost: ManaCost, x: u16) -> bool {
    pool.white >= cost.white
        && pool.blue >= cost.blue
        && pool.black >= cost.black
        && pool.red >= cost.red
        && pool.green >= cost.green
        && pool.total()
            >= colored_cost_total(cost)
                .saturating_add(cost.generic)
                .saturating_add(x.saturating_mul(cost.x_multiplier))
}

fn flexible_can_pay(
    sources: &[Vec<ManaPool>],
    index: usize,
    pool: ManaPool,
    cost: ManaCost,
    x: u16,
) -> bool {
    if index == sources.len() {
        return can_pay(pool, cost, x);
    }
    sources[index].iter().any(|output| {
        let mut next = pool;
        next.add(*output);
        flexible_can_pay(sources, index + 1, next, cost, x)
    })
}

fn pay_cost(pool: &mut ManaPool, cost: ManaCost, x: u16) {
    for color in colored_mana() {
        pool.remove_color(color, mana_cost_amount(cost, color));
    }
    pay_generic(
        pool,
        cost.generic
            .saturating_add(x.saturating_mul(cost.x_multiplier)),
    );
}

fn add_generic(mut cost: ManaCost, additional: u16) -> ManaCost {
    cost.generic = cost.generic.saturating_add(additional);
    cost
}

fn fireball_extra_cost(behavior: CardBehavior, target_count: usize) -> u16 {
    if behavior == CardBehavior::Fireball {
        u16::try_from(target_count.saturating_sub(1)).unwrap_or(u16::MAX)
    } else {
        0
    }
}

fn pay_generic(pool: &mut ManaPool, amount: u16) {
    let mut remaining = amount;
    for color in [
        ManaColor::Colorless,
        ManaColor::Green,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::White,
        ManaColor::Blue,
    ] {
        let spent = pool.amount(color).min(remaining);
        pool.remove_color(color, spent);
        remaining -= spent;
        if remaining == 0 {
            break;
        }
    }
    debug_assert_eq!(remaining, 0);
}

fn colored_mana() -> Vec<ManaColor> {
    vec![
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ]
}

const fn mana_cost_amount(cost: ManaCost, color: ManaColor) -> u16 {
    match color {
        ManaColor::White => cost.white,
        ManaColor::Blue => cost.blue,
        ManaColor::Black => cost.black,
        ManaColor::Red => cost.red,
        ManaColor::Green => cost.green,
        ManaColor::Colorless => 0,
    }
}

const fn colored_cost_total(cost: ManaCost) -> u16 {
    cost.white + cost.blue + cost.black + cost.red + cost.green
}

fn one_or_none(values: &[CardInstanceId]) -> Vec<Vec<CardInstanceId>> {
    std::iter::once(Vec::new())
        .chain(values.iter().map(|value| vec![*value]))
        .collect()
}

fn combinations(values: &[CardInstanceId], count: usize) -> Vec<Vec<CardInstanceId>> {
    if count == 0 {
        return vec![Vec::new()];
    }
    if values.len() < count {
        return Vec::new();
    }
    let mut result = Vec::new();
    for (index, value) in values.iter().enumerate() {
        for mut tail in combinations(&values[index + 1..], count - 1) {
            let mut choice = vec![*value];
            choice.append(&mut tail);
            result.push(choice);
        }
    }
    result
}

fn target_combinations(values: &[Target], count: usize) -> Vec<Vec<Target>> {
    if count == 0 {
        return vec![Vec::new()];
    }
    if values.len() < count {
        return Vec::new();
    }
    let mut result = Vec::new();
    for (index, value) in values.iter().enumerate() {
        for mut tail in target_combinations(&values[index + 1..], count - 1) {
            let mut choice = vec![*value];
            choice.append(&mut tail);
            result.push(choice);
        }
    }
    result
}

fn damage_distributions(recipient_count: usize, total: u16) -> Vec<Vec<u16>> {
    if recipient_count == 0 {
        return (total == 0).then_some(Vec::new()).into_iter().collect();
    }
    let mut result = Vec::new();
    for amount in 0..=total {
        for mut tail in damage_distributions(recipient_count - 1, total - amount) {
            let mut distribution = vec![amount];
            distribution.append(&mut tail);
            result.push(distribution);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poc::{self, cards};

    fn ready_game() -> Game {
        let deck = poc::mono_red_atog();
        let mut game = Game::new(poc::catalog().unwrap(), [deck.clone(), deck], 0).unwrap();
        game.pregame = None;
        game.step = Step::PrecombatMain;
        game.active_player = PlayerId::One;
        game.priority = PlayerId::One;
        game.battlefield.clear();
        game.stack.clear();
        game.pending_decisions.clear();
        game.pending_combat_attackers.clear();
        for player in &mut game.players {
            player.hand.clear();
            player.graveyard.clear();
            player.exile.clear();
            player.life = i16::from(rules::STARTING_LIFE);
            player.mana_pool = ManaPool::default();
        }
        game
    }

    fn card(id: u32, definition: CardDefinitionId, owner: PlayerId) -> CardInstance {
        CardInstance {
            id: CardInstanceId(id),
            definition,
            owner,
        }
    }

    fn creature(id: u32, definition: CardDefinitionId, controller: PlayerId) -> Permanent {
        Permanent {
            card: card(id, definition, controller),
            controller,
            tapped: false,
            entered_turn: 0,
            damage: 0,
            power_bonus: 0,
            toughness_bonus: 0,
            attacking: false,
            blocking: None,
            chosen_player: None,
            destroy_at_end: false,
            flying_until_end: false,
            factory_animated: false,
            dragon_whelp_activations: 0,
            plus_one_counters: 0,
            combat_damage_assignment: Vec::new(),
            copied_behavior: None,
            regeneration_shields: 0,
            trample_until_end: false,
            berserked: false,
            attacked_this_turn: false,
        }
    }

    fn spell(id: u32, definition: CardDefinitionId, controller: PlayerId, x: u16) -> StackObject {
        StackObject {
            id: StackObjectId(id),
            kind: StackObjectKind::Spell,
            card: card(id, definition, controller),
            controller,
            targets: Vec::new(),
            chosen_permanents: Vec::new(),
            x,
            is_copy: false,
        }
    }

    fn pass_priority_pair(game: &mut Game) {
        let first = game.priority;
        game.apply(first, Action::PassPriority).unwrap();
        game.apply(first.opponent(), Action::PassPriority).unwrap();
    }

    #[test]
    fn recall_charges_two_generic_mana_for_each_x() {
        let cost = CardBehavior::Recall.mana_cost();
        assert!(can_pay(
            ManaPool {
                blue: 1,
                colorless: 6,
                ..ManaPool::default()
            },
            cost,
            3,
        ));
        assert!(!can_pay(
            ManaPool {
                blue: 1,
                colorless: 5,
                ..ManaPool::default()
            },
            cost,
            3,
        ));
    }

    #[test]
    fn city_of_brass_produces_any_color_and_deals_one_damage() {
        let mut game = ready_game();
        game.battlefield
            .push(creature(10_000, cards::CITY_OF_BRASS, PlayerId::One));

        game.activate_mana_source(PlayerId::One, CardInstanceId(10_000), ManaColor::Blue);

        assert_eq!(game.players[0].mana_pool.blue, 1);
        assert_eq!(game.players[0].life, 19);
    }

    #[test]
    fn crusade_buffs_white_creatures() {
        let mut game = ready_game();
        game.battlefield
            .push(creature(10_000, cards::CRUSADE, PlayerId::One));
        game.battlefield
            .push(creature(10_001, cards::SAVANNAH_LIONS, PlayerId::One));

        assert_eq!(game.power(&game.battlefield[1]), Some(3));
        assert_eq!(game.toughness(&game.battlefield[1]), Some(2));
    }

    #[test]
    fn demonic_tutor_exposes_a_library_choice_then_shuffles() {
        let mut game = ready_game();
        game.players[0]
            .library
            .push(card(10_001, cards::JUZAM_DJINN, PlayerId::One));
        let tutor = spell(10_000, cards::DEMONIC_TUTOR, PlayerId::One, 0);

        game.resolve_spell_effect(&tutor, CardBehavior::DemonicTutor);
        let decision = game.observe(PlayerId::One).decision.unwrap();
        let option = decision
            .options
            .iter()
            .find(|option| option.card == Some((CardInstanceId(10_001), cards::JUZAM_DJINN)))
            .unwrap();
        let choice = Action::ChooseDecision {
            decision: decision.id,
            options: vec![option.id],
        };
        game.apply(PlayerId::One, choice).unwrap();

        assert_eq!(game.players[0].hand[0].definition, cards::JUZAM_DJINN);
        assert!(game.pending_decisions.is_empty());
    }

    #[test]
    fn armageddon_destroys_every_land_but_not_creatures() {
        let mut game = ready_game();
        game.battlefield.extend([
            creature(10_000, cards::CITY_OF_BRASS, PlayerId::One),
            creature(10_001, cards::SWAMP, PlayerId::Two),
            creature(10_002, cards::SAVANNAH_LIONS, PlayerId::One),
        ]);
        let armageddon = spell(10_003, cards::ARMAGEDDON, PlayerId::One, 0);

        game.resolve_spell_effect(&armageddon, CardBehavior::Armageddon);

        assert_eq!(game.battlefield.len(), 1);
        assert_eq!(game.battlefield[0].card.definition, cards::SAVANNAH_LIONS);
    }

    #[test]
    fn recall_uses_cancellable_cost_and_return_decisions() {
        let mut game = ready_game();
        game.players[0].hand.extend([
            card(10_000, cards::RECALL, PlayerId::One),
            card(10_001, cards::LIGHTNING_BOLT, PlayerId::One),
            card(10_002, cards::BALANCE, PlayerId::One),
        ]);
        game.players[0].mana_pool = ManaPool {
            blue: 1,
            colorless: 4,
            ..ManaPool::default()
        };

        game.cast_spell(PlayerId::One, CardInstanceId(10_000), Vec::new(), &[], 2);
        let cost_decision = game.observe(PlayerId::One).decision.unwrap();
        assert!(cost_decision.cancellable);
        assert_eq!(cost_decision.minimum, 2);
        let cost_action = Action::ChooseDecision {
            decision: cost_decision.id,
            options: cost_decision
                .options
                .iter()
                .take(cost_decision.minimum)
                .map(|option| option.id)
                .collect(),
        };
        game.apply(PlayerId::One, cost_action).unwrap();
        assert_eq!(game.players[0].graveyard.len(), 2);

        pass_priority_pair(&mut game);
        let return_decision = game.observe(PlayerId::One).decision.unwrap();
        assert!(!return_decision.cancellable);
        assert_eq!(return_decision.minimum, 2);
        let return_action = Action::ChooseDecision {
            decision: return_decision.id,
            options: return_decision
                .options
                .iter()
                .take(return_decision.minimum)
                .map(|option| option.id)
                .collect(),
        };
        game.apply(PlayerId::One, return_action).unwrap();

        assert_eq!(game.players[0].hand.len(), 2);
        assert_eq!(game.players[0].exile[0].definition, cards::RECALL);
    }

    #[test]
    fn balance_requests_public_sacrifices_and_private_discards() {
        let mut game = ready_game();
        game.battlefield.extend([
            creature(10_000, cards::PLAINS, PlayerId::One),
            creature(10_001, cards::CITY_OF_BRASS, PlayerId::One),
            creature(10_002, cards::SWAMP, PlayerId::Two),
        ]);
        game.players[0].hand.extend([
            card(10_003, cards::LIGHTNING_BOLT, PlayerId::One),
            card(10_004, cards::BALANCE, PlayerId::One),
        ]);
        game.players[1]
            .hand
            .push(card(10_005, cards::TERROR, PlayerId::Two));

        game.resolve_balance();
        assert_eq!(
            game.observe(PlayerId::Two).decision.unwrap().visibility,
            DecisionVisibility::Public
        );
        let decision_player = game.decision_player().unwrap();
        let pending_actions = game.legal_actions(decision_player);
        assert_eq!(pending_actions.len(), 2);
        assert!(matches!(
            &pending_actions[1],
            Action::ChooseDecision {
                decision: _,
                options
            } if options.is_empty()
        ));
        while let Some(player) = game.decision_player() {
            let Some(decision) = game.observe(player).decision else {
                break;
            };
            let action = Action::ChooseDecision {
                decision: decision.id,
                options: decision
                    .options
                    .iter()
                    .take(decision.minimum)
                    .map(|option| option.id)
                    .collect(),
            };
            game.apply(player, action).unwrap();
        }

        let land_counts = [PlayerId::One, PlayerId::Two].map(|player| {
            game.battlefield
                .iter()
                .filter(|permanent| {
                    permanent.controller == player
                        && game.kind(permanent.card.definition) == Some(CardKind::Land)
                })
                .count()
        });
        assert_eq!(land_counts, [1, 1]);
        assert_eq!(game.players[0].hand.len(), game.players[1].hand.len());
    }

    #[test]
    fn time_vault_can_untap_by_skipping_the_controllers_next_turn() {
        let mut game = ready_game();
        let mut vault = creature(10_000, cards::TIME_VAULT, PlayerId::Two);
        vault.tapped = true;
        game.battlefield.push(vault);

        game.start_next_turn();
        let decision = game.observe(PlayerId::Two).decision.unwrap();
        let untap = Action::ChooseDecision {
            decision: decision.id,
            options: vec![1],
        };
        game.apply(PlayerId::Two, untap).unwrap();
        assert!(!game.battlefield[0].tapped);

        game.start_next_turn();
        assert_eq!(game.active_player, PlayerId::One);
        game.start_next_turn();
        assert_eq!(game.active_player, PlayerId::One);
    }

    #[test]
    fn sylvan_library_tracks_drawn_cards_and_resolves_each_choice() {
        let mut game = ready_game();
        game.turn = 2;
        game.step = Step::Upkeep;
        game.battlefield
            .push(creature(10_000, cards::SYLVAN_LIBRARY, PlayerId::One));
        game.players[0].library = vec![
            card(10_001, cards::PLAINS, PlayerId::One),
            card(10_002, cards::SAVANNAH_LIONS, PlayerId::One),
            card(10_003, cards::SWORDS_TO_PLOWSHARES, PlayerId::One),
        ];

        game.advance_step();
        assert_eq!(game.players[0].hand.len(), 3);
        for mode in [1, 0] {
            let selection = game.observe(PlayerId::One).decision.unwrap();
            let select = Action::ChooseDecision {
                decision: selection.id,
                options: vec![selection.options[0].id],
            };
            game.apply(PlayerId::One, select).unwrap();
            let decision = game.observe(PlayerId::One).decision.unwrap();
            game.apply(
                PlayerId::One,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: vec![mode],
                },
            )
            .unwrap();
        }

        assert_eq!(game.players[0].life, 16);
        assert_eq!(game.players[0].hand.len(), 2);
        assert_eq!(game.players[0].library.len(), 1);
    }

    #[test]
    fn mana_vault_stays_tapped_and_can_be_paid_to_untap_at_upkeep() {
        let mut game = ready_game();
        let mut vault = creature(10_000, cards::MANA_VAULT, PlayerId::One);
        vault.tapped = true;
        game.battlefield.push(vault);
        for id in 10_001..10_005 {
            game.battlefield
                .push(creature(id, cards::MOUNTAIN, PlayerId::One));
        }
        game.step = Step::Upkeep;

        game.handle_upkeep_triggers();
        let decision = game.observe(PlayerId::One).decision.unwrap();
        assert_eq!(decision.prompt, "Mana Vault would remain tapped");
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: decision.id,
                options: vec![1],
            },
        )
        .unwrap();

        assert!(
            !game
                .battlefield
                .iter()
                .find(|permanent| permanent.card.definition == cards::MANA_VAULT)
                .unwrap()
                .tapped
        );
    }

    #[test]
    fn multiple_mana_vault_upkeep_choices_do_not_reuse_stale_mana() {
        let mut game = ready_game();
        for id in 10_000..10_002 {
            let mut vault = creature(id, cards::MANA_VAULT, PlayerId::One);
            vault.tapped = true;
            game.battlefield.push(vault);
        }
        for id in 10_002..10_006 {
            game.battlefield
                .push(creature(id, cards::MOUNTAIN, PlayerId::One));
        }
        game.step = Step::Upkeep;

        game.handle_upkeep_triggers();
        let first = game.observe(PlayerId::One).decision.unwrap();
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: first.id,
                options: vec![1],
            },
        )
        .unwrap();

        let second = game.observe(PlayerId::One).decision.unwrap();
        assert_eq!(second.prompt, "Mana Vault would remain tapped");
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: second.id,
                options: vec![1],
            },
        )
        .unwrap();

        let vaults: Vec<_> = game
            .battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == cards::MANA_VAULT)
            .map(|permanent| permanent.tapped)
            .collect();
        assert_eq!(vaults, vec![false, true]);
    }

    #[test]
    fn tapped_mana_vault_deals_one_at_the_draw_step() {
        let mut game = ready_game();
        let mut vault = creature(10_000, cards::MANA_VAULT, PlayerId::One);
        vault.tapped = true;
        game.battlefield.push(vault);
        game.step = Step::Upkeep;

        game.advance_step();

        assert_eq!(game.players[0].life, 19);
        assert_eq!(game.step, Step::Draw);
    }

    #[test]
    fn juggernaut_must_attack_if_able() {
        let mut game = ready_game();
        game.step = Step::DeclareAttackers;
        game.attackers_declared = false;
        let juggernaut = creature(10_000, cards::JUGGERNAUT, PlayerId::One);
        let juggernaut_id = juggernaut.card.id;
        game.battlefield.push(juggernaut);

        let actions = game.legal_actions(PlayerId::One);
        assert!(!actions.contains(&Action::FinishDeclaringAttackers));
        assert!(actions.contains(&Action::DeclareAttacker {
            attacker: juggernaut_id,
        }));

        game.apply(
            PlayerId::One,
            Action::DeclareAttacker {
                attacker: juggernaut_id,
            },
        )
        .unwrap();
        assert!(
            game.legal_actions(PlayerId::One)
                .contains(&Action::FinishDeclaringAttackers)
        );
    }

    #[test]
    fn triskelion_enters_with_counters_and_spends_one_to_deal_damage() {
        let mut game = ready_game();
        let triskelion = card(10_000, cards::TRISKELION, PlayerId::One);
        let triskelion_id = triskelion.id;
        game.players[0].hand.push(triskelion);
        game.players[0].mana_pool.colorless = 6;

        game.apply(
            PlayerId::One,
            Action::CastSpell {
                card: triskelion_id,
                targets: Vec::new(),
                sacrifices: Vec::new(),
                x: 0,
            },
        )
        .unwrap();
        pass_priority_pair(&mut game);
        let permanent = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == triskelion_id)
            .unwrap();
        assert_eq!(game.power(permanent), Some(4));
        assert_eq!(game.toughness(permanent), Some(4));

        game.apply(
            PlayerId::One,
            Action::ActivateAbility {
                source: triskelion_id,
                target: Some(Target::Player(PlayerId::Two)),
                sacrifice: None,
            },
        )
        .unwrap();
        let permanent = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == triskelion_id)
            .unwrap();
        assert_eq!(game.power(permanent), Some(3));
        pass_priority_pair(&mut game);
        assert_eq!(game.players[1].life, 19);
    }

    #[test]
    fn tundras_pay_counterspells_double_blue_cost() {
        let mut game = ready_game();
        let bolt = card(10_000, cards::LIGHTNING_BOLT, PlayerId::Two);
        let counterspell = card(10_001, cards::COUNTERSPELL, PlayerId::One);
        game.players[1].hand.push(bolt.clone());
        game.players[1].mana_pool.red = 1;
        game.players[0].hand.push(counterspell.clone());
        game.battlefield
            .push(creature(10_002, cards::TUNDRA, PlayerId::One));
        game.battlefield
            .push(creature(10_003, cards::TUNDRA, PlayerId::One));
        game.priority = PlayerId::Two;

        game.apply(
            PlayerId::Two,
            Action::CastSpell {
                card: bolt.id,
                targets: vec![Target::Player(PlayerId::One)],
                sacrifices: Vec::new(),
                x: 0,
            },
        )
        .unwrap();
        game.apply(PlayerId::Two, Action::PassPriority).unwrap();
        let bolt_on_stack = game.stack[0].id;
        game.apply(
            PlayerId::One,
            Action::CastSpell {
                card: counterspell.id,
                targets: vec![Target::Spell(bolt_on_stack)],
                sacrifices: Vec::new(),
                x: 0,
            },
        )
        .unwrap();
        pass_priority_pair(&mut game);

        assert!(game.stack.is_empty());
        assert_eq!(game.players[0].life, 20);
        assert_eq!(game.players[0].graveyard[0].definition, cards::COUNTERSPELL);
        assert_eq!(
            game.players[1].graveyard[0].definition,
            cards::LIGHTNING_BOLT
        );
    }

    #[test]
    fn swords_exiles_a_creature_and_grants_life_equal_to_power() {
        let mut game = ready_game();
        let serra = creature(10_000, cards::SERRA_ANGEL, PlayerId::Two);
        let serra_id = serra.card.id;
        game.battlefield.push(serra);
        let swords = card(10_001, cards::SWORDS_TO_PLOWSHARES, PlayerId::One);
        game.players[0].hand.push(swords.clone());
        game.players[0].mana_pool.white = 1;

        game.apply(
            PlayerId::One,
            Action::CastSpell {
                card: swords.id,
                targets: vec![Target::Permanent(serra_id)],
                sacrifices: Vec::new(),
                x: 0,
            },
        )
        .unwrap();
        pass_priority_pair(&mut game);

        assert!(game.battlefield.is_empty());
        assert_eq!(game.players[1].life, 24);
        assert_eq!(game.players[1].exile[0].definition, cards::SERRA_ANGEL);
    }

    #[test]
    fn ancestral_recall_draws_three_and_time_walk_queues_an_extra_turn() {
        let mut game = ready_game();
        let ancestral = card(10_000, cards::ANCESTRAL_RECALL, PlayerId::One);
        game.players[0].hand.push(ancestral.clone());
        game.players[0].mana_pool.blue = 1;
        let hand_before = game.players[0].hand.len();
        game.apply(
            PlayerId::One,
            Action::CastSpell {
                card: ancestral.id,
                targets: vec![Target::Player(PlayerId::One)],
                sacrifices: Vec::new(),
                x: 0,
            },
        )
        .unwrap();
        pass_priority_pair(&mut game);
        assert_eq!(game.players[0].hand.len(), hand_before - 1 + 3);

        let time_walk = card(10_001, cards::TIME_WALK, PlayerId::One);
        game.players[0].hand.push(time_walk.clone());
        game.players[0].mana_pool.blue = 1;
        game.players[0].mana_pool.colorless = 1;
        game.apply(
            PlayerId::One,
            Action::CastSpell {
                card: time_walk.id,
                targets: Vec::new(),
                sacrifices: Vec::new(),
                x: 0,
            },
        )
        .unwrap();
        pass_priority_pair(&mut game);
        game.start_next_turn();
        assert_eq!(game.active_player, PlayerId::One);
        assert_eq!(game.observe(PlayerId::One).active_turn, 2);
    }

    #[test]
    fn serra_angel_attacks_without_tapping() {
        let mut game = ready_game();
        game.step = Step::DeclareAttackers;
        game.attackers_declared = false;
        let serra = creature(10_000, cards::SERRA_ANGEL, PlayerId::One);
        let serra_id = serra.card.id;
        game.battlefield.push(serra);

        game.apply(
            PlayerId::One,
            Action::DeclareAttacker { attacker: serra_id },
        )
        .unwrap();

        let serra = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == serra_id)
            .unwrap();
        assert!(serra.attacking);
        assert!(!serra.tapped);
    }

    #[test]
    fn ivory_tower_and_jayemdae_tome_provide_control_card_advantage() {
        let mut game = ready_game();
        game.players[0].life = 10;
        for id in 10_000..10_006 {
            game.players[0]
                .hand
                .push(card(id, cards::MOUNTAIN, PlayerId::One));
        }
        game.battlefield
            .push(creature(10_010, cards::IVORY_TOWER, PlayerId::One));
        let tome = creature(10_011, cards::JAYEMDAE_TOME, PlayerId::One);
        let tome_id = tome.card.id;
        game.battlefield.push(tome);
        game.players[0].mana_pool.colorless = 4;

        game.handle_upkeep_triggers();
        assert_eq!(game.players[0].life, 12);
        let hand_before = game.players[0].hand.len();
        game.apply(
            PlayerId::One,
            Action::ActivateAbility {
                source: tome_id,
                target: None,
                sacrifice: None,
            },
        )
        .unwrap();
        pass_priority_pair(&mut game);
        assert_eq!(game.players[0].hand.len(), hand_before + 1);
    }

    #[test]
    fn fireball_pays_for_multiple_targets_and_divides_x_evenly() {
        let mut game = ready_game();
        let fireball = card(10_000, cards::FIREBALL, PlayerId::One);
        let creature = creature(10_001, cards::SU_CHI, PlayerId::Two);
        let creature_id = creature.card.id;
        game.players[0].hand.push(fireball.clone());
        game.players[0].mana_pool.red = 6;
        game.battlefield.push(creature);

        let action = Action::CastSpell {
            card: fireball.id,
            targets: vec![
                Target::Player(PlayerId::Two),
                Target::Permanent(creature_id),
            ],
            sacrifices: Vec::new(),
            x: 4,
        };
        assert!(game.legal_actions(PlayerId::One).contains(&action));

        game.apply(PlayerId::One, action).unwrap();
        assert_eq!(game.players[0].mana_pool.total(), 0);
        pass_priority_pair(&mut game);

        assert_eq!(game.players[1].life, 18);
        assert_eq!(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == creature_id)
                .unwrap()
                .damage,
            2
        );
    }

    #[test]
    fn fireball_x_three_can_hit_three_targets_for_six_mana() {
        let mut game = ready_game();
        let fireball = card(10_000, cards::FIREBALL, PlayerId::One);
        let first_creature = creature(10_001, cards::SU_CHI, PlayerId::Two);
        let first_creature_id = first_creature.card.id;
        let second_creature = creature(10_002, cards::JUGGERNAUT, PlayerId::Two);
        let second_creature_id = second_creature.card.id;
        game.players[0].hand.push(fireball.clone());
        game.players[0].mana_pool.red = 6;
        game.battlefield.push(first_creature);
        game.battlefield.push(second_creature);

        let action = Action::CastSpell {
            card: fireball.id,
            targets: vec![
                Target::Player(PlayerId::Two),
                Target::Permanent(first_creature_id),
                Target::Permanent(second_creature_id),
            ],
            sacrifices: Vec::new(),
            x: 3,
        };
        assert!(game.legal_actions(PlayerId::One).contains(&action));

        game.apply(PlayerId::One, action).unwrap();
        assert_eq!(game.players[0].mana_pool.total(), 0);
        pass_priority_pair(&mut game);

        assert_eq!(game.players[1].life, 19);
        for creature_id in [first_creature_id, second_creature_id] {
            assert_eq!(
                game.battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == creature_id)
                    .unwrap()
                    .damage,
                1
            );
        }
    }

    #[test]
    fn fork_controller_can_retarget_the_copied_spell() {
        let mut game = ready_game();
        let fork = card(10_000, cards::FORK, PlayerId::One);
        game.players[0].hand.push(fork.clone());
        game.players[0].mana_pool.red = 2;
        game.stack.push(StackObject {
            id: StackObjectId(77),
            kind: StackObjectKind::Spell,
            card: card(10_001, cards::LIGHTNING_BOLT, PlayerId::Two),
            controller: PlayerId::Two,
            targets: vec![Target::Player(PlayerId::One)],
            chosen_permanents: Vec::new(),
            x: 0,
            is_copy: false,
        });

        game.apply(
            PlayerId::One,
            Action::CastSpell {
                card: fork.id,
                targets: vec![Target::Spell(StackObjectId(77))],
                sacrifices: Vec::new(),
                x: 0,
            },
        )
        .unwrap();
        pass_priority_pair(&mut game);

        let decision = game.observe(PlayerId::One).decision.unwrap();
        let retarget = decision
            .options
            .iter()
            .find(|option| option.label.contains("your opponent"))
            .map(|option| option.id)
            .unwrap();
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: decision.id,
                options: vec![retarget],
            },
        )
        .unwrap();
        pass_priority_pair(&mut game);

        assert_eq!(game.players[0].life, 20);
        assert_eq!(game.players[1].life, 17);
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].targets, vec![Target::Player(PlayerId::One)]);
    }

    #[test]
    fn fork_can_keep_an_original_target_that_has_become_illegal() {
        let mut game = ready_game();
        let stale_target = Target::Permanent(CardInstanceId(99_999));
        game.queue_fork_decision(
            PlayerId::One,
            StackObject {
                id: StackObjectId(77),
                kind: StackObjectKind::Spell,
                card: card(10_001, cards::SHATTER, PlayerId::Two),
                controller: PlayerId::Two,
                targets: vec![stale_target],
                chosen_permanents: Vec::new(),
                x: 0,
                is_copy: false,
            },
        );
        let decision = game.observe(PlayerId::One).decision.unwrap();
        assert!(
            decision
                .options
                .iter()
                .any(|option| option.label == "Keep original targets")
        );
    }

    #[test]
    fn black_lotus_sacrifices_for_three_red_mana() {
        let mut game = ready_game();
        let lotus = creature(10_000, cards::BLACK_LOTUS, PlayerId::One);
        let lotus_id = lotus.card.id;
        game.battlefield.push(lotus);
        let action = Action::ActivateManaAbility {
            source: lotus_id,
            color: ManaColor::Red,
        };
        assert!(game.legal_actions(PlayerId::One).contains(&action));

        game.apply(PlayerId::One, action).unwrap();

        assert_eq!(game.players[0].mana_pool.red, 3);
        assert!(
            game.battlefield
                .iter()
                .all(|permanent| permanent.card.id != lotus_id)
        );
        assert_eq!(game.players[0].graveyard.last().unwrap().id, lotus_id);
    }

    #[test]
    fn mox_ruby_can_pay_black_vises_generic_cost() {
        let mut game = ready_game();
        let mox = creature(10_000, cards::MOX_RUBY, PlayerId::One);
        let vise = card(10_001, cards::BLACK_VISE, PlayerId::One);
        let mox_id = mox.card.id;
        game.battlefield.push(mox);
        game.players[0].hand.push(vise.clone());

        let cast_vise = Action::CastSpell {
            card: vise.id,
            targets: vec![Target::Player(PlayerId::Two)],
            sacrifices: Vec::new(),
            x: 0,
        };
        assert!(game.legal_actions(PlayerId::One).contains(&cast_vise));
        game.apply(PlayerId::One, cast_vise).unwrap();
        assert_eq!(game.players[0].mana_pool, ManaPool::default());
        assert!(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == mox_id)
                .is_some_and(|permanent| permanent.tapped)
        );
    }

    #[test]
    fn mana_preview_uses_existing_pool_before_tapping_sources() {
        let mut game = ready_game();
        let mountain = creature(10_000, cards::MOUNTAIN, PlayerId::One);
        let mox = creature(10_001, cards::MOX_RUBY, PlayerId::One);
        let vise = card(10_002, cards::BLACK_VISE, PlayerId::One);
        let mountain_id = mountain.card.id;
        let mox_id = mox.card.id;
        game.battlefield.extend([mox, mountain]);
        game.players[0].mana_pool.colorless = 1;
        game.players[0].hand.push(vise.clone());

        let action = Action::CastSpell {
            card: vise.id,
            targets: vec![Target::Player(PlayerId::Two)],
            sacrifices: Vec::new(),
            x: 0,
        };
        assert_eq!(
            game.mana_sources_for_action(PlayerId::One, &action),
            Vec::<CardInstanceId>::new(),
            "the floating mana already pays Black Vise's generic cost"
        );

        game.players[0].mana_pool = ManaPool::default();
        assert_eq!(
            game.mana_sources_for_action(PlayerId::One, &action),
            vec![mox_id],
            "the preview chooses a single flexible source without mutating the game"
        );
        assert!(
            !game
                .battlefield
                .iter()
                .find(|permanent| permanent.card.id == mountain_id)
                .expect("mountain remains on the battlefield")
                .tapped
        );
    }

    #[test]
    fn orcish_mechanics_can_sacrifice_an_artifact_to_damage_a_creature() {
        let mut game = ready_game();
        let mechanics = creature(10_000, cards::ORCISH_MECHANICS, PlayerId::One);
        let artifact = creature(10_001, cards::MOX_RUBY, PlayerId::One);
        let target = creature(10_002, cards::SU_CHI, PlayerId::Two);
        let mechanics_id = mechanics.card.id;
        let artifact_id = artifact.card.id;
        let target_id = target.card.id;
        game.battlefield = vec![mechanics, artifact, target];

        let action = Action::ActivateAbility {
            source: mechanics_id,
            target: Some(Target::Permanent(target_id)),
            sacrifice: Some(artifact_id),
        };
        assert!(game.legal_actions(PlayerId::One).contains(&action));

        game.apply(PlayerId::One, action).unwrap();
        assert!(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == mechanics_id)
                .is_some_and(|permanent| permanent.tapped)
        );
        assert!(
            game.battlefield
                .iter()
                .all(|permanent| permanent.card.id != artifact_id)
        );
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].targets, vec![Target::Permanent(target_id)]);
        assert_eq!(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == target_id)
                .unwrap()
                .damage,
            0
        );

        pass_priority_pair(&mut game);
        assert_eq!(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == target_id)
                .unwrap()
                .damage,
            2
        );
    }

    #[test]
    fn iron_star_payment_can_use_untapped_mana_sources() {
        let mut game = ready_game();
        let mountain = creature(10_000, cards::MOUNTAIN, PlayerId::One);
        let mountain_id = mountain.card.id;
        game.battlefield.push(mountain);
        game.queue_iron_star_decision(PlayerId::One);
        let decision = game.observe(PlayerId::One).decision.unwrap();
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: decision.id,
                options: vec![1],
            },
        )
        .unwrap();

        assert_eq!(game.players[0].life, 21);
        assert_eq!(game.players[0].mana_pool, ManaPool::default());
        assert!(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == mountain_id)
                .is_some_and(|permanent| permanent.tapped)
        );
    }

    #[test]
    fn chain_lightning_copy_payment_can_use_untapped_mountains() {
        let mut game = ready_game();
        let first = creature(10_000, cards::MOUNTAIN, PlayerId::Two);
        let second = creature(10_001, cards::MOUNTAIN, PlayerId::Two);
        let first_id = first.card.id;
        let second_id = second.card.id;
        game.battlefield = vec![first, second];
        game.queue_chain_lightning_decision(
            PlayerId::Two,
            StackObject {
                id: StackObjectId(77),
                kind: StackObjectKind::Spell,
                card: card(10_002, cards::CHAIN_LIGHTNING, PlayerId::One),
                controller: PlayerId::One,
                targets: vec![Target::Player(PlayerId::Two)],
                chosen_permanents: Vec::new(),
                x: 0,
                is_copy: false,
            },
        );
        let decision = game.observe(PlayerId::Two).decision.unwrap();
        let copy = decision
            .options
            .iter()
            .find(|option| option.label.contains("your opponent"))
            .map(|option| option.id)
            .unwrap();
        game.apply(
            PlayerId::Two,
            Action::ChooseDecision {
                decision: decision.id,
                options: vec![copy],
            },
        )
        .unwrap();

        assert_eq!(game.players[1].mana_pool, ManaPool::default());
        assert!(
            game.battlefield
                .iter()
                .filter(|permanent| [first_id, second_id].contains(&permanent.card.id))
                .all(|permanent| permanent.tapped)
        );
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].targets, vec![Target::Player(PlayerId::One)]);
        assert!(game.stack[0].is_copy);
    }

    #[test]
    fn goblin_grenade_requires_and_sacrifices_a_goblin() {
        let mut game = ready_game();
        let grenade = card(10_000, cards::GOBLIN_GRENADE, PlayerId::One);
        let goblin = creature(10_001, cards::GOBLIN_BALLOON_BRIGADE, PlayerId::One);
        let goblin_id = goblin.card.id;
        game.players[0].hand.push(grenade.clone());
        game.players[0].mana_pool.red = 1;
        game.battlefield.push(goblin);
        let action = Action::CastSpell {
            card: grenade.id,
            targets: vec![Target::Player(PlayerId::Two)],
            sacrifices: vec![goblin_id],
            x: 0,
        };
        assert!(game.legal_actions(PlayerId::One).contains(&action));

        game.apply(PlayerId::One, action).unwrap();
        assert!(
            game.battlefield
                .iter()
                .all(|permanent| permanent.card.id != goblin_id)
        );
        pass_priority_pair(&mut game);
        assert_eq!(game.players[1].life, 15);
    }

    #[test]
    fn factory_animates_and_strip_mine_destroys_lands() {
        let mut game = ready_game();
        let factory = creature(10_000, cards::MISHRA_S_FACTORY, PlayerId::One);
        let strip = creature(10_001, cards::STRIP_MINE, PlayerId::One);
        let opposing_factory = creature(10_002, cards::MISHRA_S_FACTORY, PlayerId::Two);
        let factory_id = factory.card.id;
        let strip_id = strip.card.id;
        let opposing_id = opposing_factory.card.id;
        game.battlefield = vec![factory, strip, opposing_factory];
        game.players[0].mana_pool.colorless = 1;

        game.apply(
            PlayerId::One,
            Action::ActivateAbility {
                source: factory_id,
                target: None,
                sacrifice: None,
            },
        )
        .unwrap();
        assert_eq!(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == factory_id)
                .and_then(|permanent| game.power(permanent)),
            Some(2)
        );

        game.apply(
            PlayerId::One,
            Action::ActivateAbility {
                source: strip_id,
                target: Some(Target::Permanent(opposing_id)),
                sacrifice: Some(strip_id),
            },
        )
        .unwrap();
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].kind, StackObjectKind::ActivatedAbility);
        assert_eq!(game.stack[0].targets, vec![Target::Permanent(opposing_id)]);
        assert!(
            game.battlefield
                .iter()
                .all(|permanent| permanent.card.id != strip_id)
        );
        assert!(
            game.battlefield
                .iter()
                .any(|permanent| permanent.card.id == opposing_id)
        );

        pass_priority_pair(&mut game);
        assert!(
            game.battlefield
                .iter()
                .all(|permanent| permanent.card.id != opposing_id)
        );
    }

    #[test]
    fn mishras_factory_can_use_its_own_mana_to_animate() {
        let mut game = ready_game();
        let factory = creature(10_000, cards::MISHRA_S_FACTORY, PlayerId::One);
        let factory_id = factory.card.id;
        game.battlefield = vec![factory];
        let animate = Action::ActivateAbility {
            source: factory_id,
            target: None,
            sacrifice: None,
        };

        assert!(game.legal_actions(PlayerId::One).contains(&animate));
        game.apply(PlayerId::One, animate).unwrap();

        let factory = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == factory_id)
            .unwrap();
        assert!(factory.tapped);
        assert_eq!(game.power(factory), Some(2));
        assert_eq!(game.players[0].mana_pool.total(), 0);

        let shatter = card(10_001, cards::SHATTER, PlayerId::Two);
        game.players[1].hand.push(shatter.clone());
        game.players[1].mana_pool.red = 2;
        game.priority = PlayerId::Two;
        assert!(
            game.legal_actions(PlayerId::Two)
                .contains(&Action::CastSpell {
                    card: shatter.id,
                    targets: vec![Target::Permanent(factory_id)],
                    sacrifices: Vec::new(),
                    x: 0,
                })
        );
    }

    #[test]
    fn an_animated_untapped_mishras_factory_can_block() {
        let mut game = ready_game();
        let mut attacker = creature(10_000, cards::GOBLINS_OF_THE_FLARG, PlayerId::One);
        attacker.attacking = true;
        let attacker_id = attacker.card.id;
        let mut factory = creature(10_001, cards::MISHRA_S_FACTORY, PlayerId::Two);
        factory.factory_animated = true;
        let factory_id = factory.card.id;
        game.battlefield = vec![attacker, factory];
        game.active_player = PlayerId::One;
        game.step = Step::DeclareBlockers;
        game.blockers_declared = false;

        assert!(
            game.legal_actions(PlayerId::Two)
                .contains(&Action::DeclareBlocker {
                    blocker: factory_id,
                    attacker: attacker_id,
                })
        );
    }

    #[test]
    fn strip_mine_can_be_activated_in_response_to_strip_mine() {
        let mut game = ready_game();
        let first_strip = creature(10_000, cards::STRIP_MINE, PlayerId::One);
        let second_strip = creature(10_001, cards::STRIP_MINE, PlayerId::Two);
        let other_land = creature(10_002, cards::MOUNTAIN, PlayerId::Two);
        let first_strip_id = first_strip.card.id;
        let second_strip_id = second_strip.card.id;
        let other_land_id = other_land.card.id;
        game.battlefield = vec![first_strip, second_strip, other_land];
        game.priority = PlayerId::Two;

        game.apply(
            PlayerId::Two,
            Action::ActivateAbility {
                source: second_strip_id,
                target: Some(Target::Permanent(first_strip_id)),
                sacrifice: Some(second_strip_id),
            },
        )
        .unwrap();
        game.apply(PlayerId::Two, Action::PassPriority).unwrap();

        let response = Action::ActivateAbility {
            source: first_strip_id,
            target: Some(Target::Permanent(other_land_id)),
            sacrifice: Some(first_strip_id),
        };
        assert!(game.legal_actions(PlayerId::One).contains(&response));
        game.apply(PlayerId::One, response).unwrap();
        assert_eq!(game.stack.len(), 2);

        pass_priority_pair(&mut game);
        assert!(
            game.battlefield
                .iter()
                .all(|permanent| permanent.card.id != other_land_id)
        );
        assert_eq!(game.stack.len(), 1);

        pass_priority_pair(&mut game);
        assert!(game.stack.is_empty());
        assert!(
            game.battlefield
                .iter()
                .all(|permanent| ![first_strip_id, second_strip_id].contains(&permanent.card.id))
        );
    }

    #[test]
    fn chaos_orb_uses_the_documented_deterministic_success_rule() {
        let mut game = ready_game();
        let orb = creature(10_000, cards::CHAOS_ORB, PlayerId::One);
        let target = creature(10_001, cards::BLACK_VISE, PlayerId::Two);
        let orb_id = orb.card.id;
        let target_id = target.card.id;
        game.battlefield = vec![orb, target];
        game.players[0].mana_pool.colorless = 1;
        let action = Action::ActivateAbility {
            source: orb_id,
            target: Some(Target::Permanent(target_id)),
            sacrifice: None,
        };
        assert!(game.legal_actions(PlayerId::One).contains(&action));

        game.apply(PlayerId::One, action).unwrap();

        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].kind, StackObjectKind::ActivatedAbility);
        assert_eq!(game.stack[0].chosen_permanents, vec![target_id]);
        assert!(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == orb_id)
                .is_some_and(|permanent| permanent.tapped)
        );
        assert!(
            game.battlefield
                .iter()
                .any(|permanent| permanent.card.id == target_id)
        );
        pass_priority_pair(&mut game);
        assert!(game.battlefield.is_empty());
        assert_eq!(game.players[0].mana_pool.total(), 0);
    }

    #[test]
    fn chaos_orb_can_be_activated_the_turn_it_enters_using_untapped_mana() {
        let mut game = ready_game();
        let mut orb = creature(10_000, cards::CHAOS_ORB, PlayerId::One);
        let mut mountain = creature(10_001, cards::MOUNTAIN, PlayerId::One);
        let target = creature(10_002, cards::BLACK_VISE, PlayerId::Two);
        orb.entered_turn = game.turn;
        mountain.entered_turn = game.turn;
        let orb_id = orb.card.id;
        let mountain_id = mountain.card.id;
        let target_id = target.card.id;
        game.battlefield = vec![orb, mountain, target];
        let action = Action::ActivateAbility {
            source: orb_id,
            target: Some(Target::Permanent(target_id)),
            sacrifice: None,
        };

        assert!(game.legal_actions(PlayerId::One).contains(&action));
        game.apply(PlayerId::One, action).unwrap();

        assert!(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == orb_id)
                .is_some_and(|permanent| permanent.tapped)
        );
        assert!(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == mountain_id)
                .is_some_and(|permanent| permanent.tapped)
        );
        assert_eq!(game.players[0].mana_pool.total(), 0);
        assert_eq!(game.stack.len(), 1);
    }

    #[test]
    fn removing_chaos_orb_in_response_nullifies_its_flip() {
        let mut game = ready_game();
        let orb = creature(10_000, cards::CHAOS_ORB, PlayerId::One);
        let target = creature(10_001, cards::BLACK_VISE, PlayerId::Two);
        let shatter = card(10_002, cards::SHATTER, PlayerId::Two);
        let orb_id = orb.card.id;
        let target_id = target.card.id;
        game.battlefield = vec![orb, target];
        game.players[0].mana_pool.colorless = 1;
        game.players[1].hand.push(shatter.clone());
        game.players[1].mana_pool.red = 2;

        game.apply(
            PlayerId::One,
            Action::ActivateAbility {
                source: orb_id,
                target: Some(Target::Permanent(target_id)),
                sacrifice: None,
            },
        )
        .unwrap();
        game.apply(PlayerId::One, Action::PassPriority).unwrap();
        game.apply(
            PlayerId::Two,
            Action::CastSpell {
                card: shatter.id,
                targets: vec![Target::Permanent(orb_id)],
                sacrifices: Vec::new(),
                x: 0,
            },
        )
        .unwrap();
        pass_priority_pair(&mut game);
        assert_eq!(game.stack.len(), 1);
        assert!(
            game.battlefield
                .iter()
                .all(|permanent| permanent.card.id != orb_id)
        );

        pass_priority_pair(&mut game);

        assert!(game.stack.is_empty());
        assert!(
            game.battlefield
                .iter()
                .any(|permanent| permanent.card.id == target_id)
        );
    }

    #[test]
    fn goblin_king_buffs_other_goblins_and_grants_mountainwalk() {
        let mut game = ready_game();
        let king = creature(10_000, cards::GOBLIN_KING, PlayerId::One);
        let mut flarg = creature(10_001, cards::GOBLINS_OF_THE_FLARG, PlayerId::One);
        flarg.attacking = true;
        let mountain = creature(10_002, cards::MOUNTAIN, PlayerId::Two);
        let blocker = creature(10_003, cards::IRONCLAW_ORCS, PlayerId::Two);
        let flarg_id = flarg.card.id;
        game.battlefield = vec![king, flarg, mountain, blocker];
        game.step = Step::DeclareBlockers;
        game.blockers_declared = false;

        let flarg = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == flarg_id)
            .unwrap();
        assert_eq!(game.power(flarg), Some(2));
        assert!(
            game.legal_actions(PlayerId::Two)
                .iter()
                .all(|action| !matches!(
                    action,
                    Action::DeclareBlocker { attacker, .. } if *attacker == flarg_id
                ))
        );
    }

    #[test]
    fn wheel_discards_both_hands_and_draws_seven() {
        let mut game = ready_game();
        let wheel = card(10_000, cards::WHEEL_OF_FORTUNE, PlayerId::One);
        game.players[0].hand.push(wheel.clone());
        game.players[0]
            .hand
            .push(card(10_001, cards::MOUNTAIN, PlayerId::One));
        game.players[1]
            .hand
            .push(card(10_002, cards::MOUNTAIN, PlayerId::Two));
        game.players[0].mana_pool.red = 3;

        game.apply(
            PlayerId::One,
            Action::CastSpell {
                card: wheel.id,
                targets: Vec::new(),
                sacrifices: Vec::new(),
                x: 0,
            },
        )
        .unwrap();
        pass_priority_pair(&mut game);

        assert_eq!(game.players[0].hand.len(), 7);
        assert_eq!(game.players[1].hand.len(), 7);
        assert!(
            game.players[0]
                .graveyard
                .iter()
                .any(|card| card.id == CardInstanceId(10_001))
        );
    }

    #[test]
    fn cleanup_without_a_discard_advances_without_priority() {
        let mut game = ready_game();
        game.step = Step::End;
        let first_turn = game.turn;

        pass_priority_pair(&mut game);

        assert_eq!(game.turn, first_turn + 1);
        assert_eq!(game.step, Step::Upkeep);
        assert_eq!(game.active_player, PlayerId::Two);
        assert_eq!(game.observe(PlayerId::One).active_turn, 1);
        assert_eq!(game.decision_player(), Some(PlayerId::Two));
    }

    #[test]
    fn cleanup_discard_advances_directly_to_the_next_upkeep() {
        let mut game = ready_game();
        game.step = Step::End;
        for id in 10_000..10_008 {
            game.players[0]
                .hand
                .push(card(id, cards::MOUNTAIN, PlayerId::One));
        }

        pass_priority_pair(&mut game);
        assert_eq!(game.step, Step::Cleanup);
        let discard = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| matches!(action, Action::DiscardCards { .. }))
            .unwrap();
        game.apply(PlayerId::One, discard).unwrap();

        assert_eq!(game.turn, 2);
        assert_eq!(game.step, Step::Upkeep);
        assert_eq!(game.active_player, PlayerId::Two);
        assert_eq!(game.decision_player(), Some(PlayerId::Two));
    }

    #[test]
    fn attacker_controller_assigns_damage_freely_across_multiple_blockers() {
        let mut game = ready_game();
        let mut attacker = creature(10_000, cards::SU_CHI, PlayerId::One);
        attacker.attacking = true;
        let mut first_blocker = creature(10_001, cards::ATOG, PlayerId::Two);
        first_blocker.blocking = Some(attacker.card.id);
        let mut second_blocker = creature(10_002, cards::ATOG, PlayerId::Two);
        second_blocker.blocking = Some(attacker.card.id);
        let attacker_id = attacker.card.id;
        let first_id = first_blocker.card.id;
        let second_id = second_blocker.card.id;
        game.battlefield = vec![attacker, first_blocker, second_blocker];
        game.begin_combat_damage_assignment();

        let assignment = Action::AssignCombatDamage {
            attacker: attacker_id,
            assignments: vec![
                CombatDamageAssignment {
                    recipient: Target::Permanent(first_id),
                    amount: 1,
                },
                CombatDamageAssignment {
                    recipient: Target::Permanent(second_id),
                    amount: 3,
                },
            ],
        };
        assert!(game.legal_actions(PlayerId::One).contains(&assignment));
        game.apply(PlayerId::One, assignment).unwrap();

        let first = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == first_id)
            .unwrap();
        assert_eq!(first.damage, 1);
        assert!(
            game.battlefield
                .iter()
                .all(|permanent| permanent.card.id != second_id)
        );
        let attacker = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == attacker_id)
            .unwrap();
        assert_eq!(attacker.damage, 2);
    }

    #[test]
    fn trample_requires_lethal_assignment_before_player_damage() {
        let mut game = ready_game();
        let mut attacker = creature(10_000, cards::BALL_LIGHTNING, PlayerId::One);
        attacker.attacking = true;
        let mut blocker = creature(10_001, cards::ATOG, PlayerId::Two);
        blocker.blocking = Some(attacker.card.id);
        let attacker_id = attacker.card.id;
        let blocker_id = blocker.card.id;
        game.battlefield = vec![attacker, blocker];
        game.begin_combat_damage_assignment();

        let assignment = |to_blocker, to_player| Action::AssignCombatDamage {
            attacker: attacker_id,
            assignments: vec![
                CombatDamageAssignment {
                    recipient: Target::Permanent(blocker_id),
                    amount: to_blocker,
                },
                CombatDamageAssignment {
                    recipient: Target::Player(PlayerId::Two),
                    amount: to_player,
                },
            ],
        };
        let actions = game.legal_actions(PlayerId::One);
        assert!(!actions.contains(&assignment(1, 5)));
        assert!(actions.contains(&assignment(2, 4)));
    }

    #[test]
    fn green_creatures_get_their_land_bonuses_and_llanowar_elves_make_green() {
        let mut game = ready_game();
        game.battlefield.extend([
            creature(10_000, cards::FOREST, PlayerId::One),
            creature(10_001, cards::KIRD_APE, PlayerId::One),
            creature(10_002, cards::LLANOWAR_ELVES, PlayerId::One),
        ]);
        assert_eq!(game.power(&game.battlefield[1]), Some(2));
        assert_eq!(game.toughness(&game.battlefield[1]), Some(3));
        assert_eq!(
            game.mana_colors(&game.battlefield[2]),
            vec![ManaColor::Green]
        );
    }

    #[test]
    fn copy_artifact_copies_an_artifact_creature() {
        let mut game = ready_game();
        let source = creature(10_000, cards::TETRAVUS, PlayerId::Two);
        game.battlefield.push(source);
        let copy = card(10_001, cards::COPY_ARTIFACT, PlayerId::One);
        game.players[0].hand.push(copy.clone());
        game.players[0].mana_pool.blue = 1;
        game.players[0].mana_pool.colorless = 1;
        let action = Action::CastSpell {
            card: copy.id,
            targets: vec![Target::Permanent(CardInstanceId(10_000))],
            sacrifices: Vec::new(),
            x: 0,
        };
        assert!(game.legal_actions(PlayerId::One).contains(&action));
        game.apply(PlayerId::One, action).unwrap();
        pass_priority_pair(&mut game);
        let copied = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == copy.id)
            .unwrap();
        assert_eq!(
            game.effective_behavior(copied),
            Some(CardBehavior::Tetravus)
        );
        assert_eq!(game.power(copied), Some(4));
        assert!(game.has_flying(copied));
    }

    #[test]
    fn dust_to_dust_exiles_two_artifacts_and_hurkyls_recall_returns_them() {
        let mut game = ready_game();
        game.battlefield.extend([
            creature(10_000, cards::SOL_RING, PlayerId::Two),
            creature(10_001, cards::BLACK_VISE, PlayerId::Two),
        ]);
        let dust = spell(10_002, cards::DUST_TO_DUST, PlayerId::One, 0);
        dust_to_dust_targets(&mut game, dust);
        assert_eq!(game.players[0].exile.len(), 0);
        assert_eq!(game.players[1].exile.len(), 2);

        let mut game = ready_game();
        game.battlefield.extend([
            creature(10_000, cards::SOL_RING, PlayerId::Two),
            creature(10_001, cards::BLACK_VISE, PlayerId::Two),
        ]);
        let mut recall = spell(10_002, cards::HURKYLS_RECALL, PlayerId::One, 0);
        recall.targets = vec![Target::Player(PlayerId::Two)];
        game.resolve_spell_effect(&recall, CardBehavior::HurkylsRecall);
        assert_eq!(game.players[1].hand.len(), 2);
        assert!(game.battlefield.is_empty());
    }

    fn dust_to_dust_targets(game: &mut Game, mut spell: StackObject) {
        spell.targets = vec![
            Target::Permanent(CardInstanceId(10_000)),
            Target::Permanent(CardInstanceId(10_001)),
        ];
        game.resolve_spell_effect(&spell, CardBehavior::DustToDust);
    }

    #[test]
    fn regeneration_shields_stop_destroy_but_not_wrath() {
        let mut game = ready_game();
        let mut troll = creature(10_000, cards::SEDGE_TROLL, PlayerId::One);
        troll.regeneration_shields = 1;
        game.battlefield.push(troll);
        game.destroy_permanent(CardInstanceId(10_000));
        assert_eq!(game.battlefield.len(), 1);
        assert!(game.battlefield[0].tapped);
        assert_eq!(game.battlefield[0].regeneration_shields, 0);

        let wrath = spell(10_001, cards::WRATH_OF_GOD, PlayerId::Two, 0);
        game.resolve_spell_effect(&wrath, CardBehavior::WrathOfGod);
        assert!(game.battlefield.is_empty());
    }

    #[test]
    fn moat_prevents_nonfliers_and_argothian_pixies_dodge_artifact_blockers() {
        let mut game = ready_game();
        game.step = Step::DeclareAttackers;
        game.attackers_declared = false;
        game.battlefield
            .push(creature(10_000, cards::MOAT, PlayerId::Two));
        game.battlefield
            .push(creature(10_001, cards::SAVANNAH_LIONS, PlayerId::One));
        game.battlefield
            .push(creature(10_002, cards::SERENDIB_EFREET, PlayerId::One));
        let actions = game.legal_actions(PlayerId::One);
        assert!(!actions.contains(&Action::DeclareAttacker {
            attacker: CardInstanceId(10_001)
        }));
        assert!(actions.contains(&Action::DeclareAttacker {
            attacker: CardInstanceId(10_002)
        }));

        let mut game = ready_game();
        game.step = Step::DeclareBlockers;
        game.blockers_declared = false;
        let mut pixies = creature(10_003, cards::ARGOTHIAN_PIXIES, PlayerId::One);
        pixies.attacking = true;
        game.battlefield.push(pixies);
        game.battlefield
            .push(creature(10_004, cards::SU_CHI, PlayerId::Two));
        assert!(
            !game
                .legal_actions(PlayerId::Two)
                .contains(&Action::DeclareBlocker {
                    blocker: CardInstanceId(10_004),
                    attacker: CardInstanceId(10_003),
                })
        );
    }
}
