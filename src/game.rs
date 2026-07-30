use std::error::Error;
use std::fmt;

use crate::action::{Action, ActionError, Target};
use crate::card::{CardBehavior, CardCatalog, CardKind, ManaCost};
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StackObject {
    id: StackObjectId,
    card: CardInstance,
    controller: PlayerId,
    target: Option<Target>,
    x: u16,
    is_copy: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlayerState {
    life: i16,
    library: Vec<CardInstance>,
    hand: Vec<CardInstance>,
    graveyard: Vec<CardInstance>,
    mana_pool: ManaPool,
    land_played_this_turn: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Pregame {
    Mulligan(PlayerId),
    Bottom(PlayerId),
}

#[derive(Clone, Debug)]
enum PendingChoice {
    IronStar {
        player: PlayerId,
    },
    ChainLightning {
        player: PlayerId,
        spell: StackObject,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ManaPool {
    pub red: u16,
    pub colorless: u16,
}

impl ManaPool {
    #[must_use]
    pub const fn total(self) -> u16 {
        self.red + self.colorless
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
        target: Option<Target>,
    },
    SpellResolved {
        card: CardInstanceId,
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
    pub card: CardInstanceId,
    pub definition: CardDefinitionId,
    pub controller: PlayerId,
    pub target: Option<Target>,
    pub x: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerObservation {
    pub viewer: PlayerId,
    pub turn: u32,
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
    pub battlefield: Vec<PermanentObservation>,
    pub stack: Vec<StackObservation>,
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
    pending_choices: Vec<PendingChoice>,
    last_seen_hands: [LastSeenHand; 2],
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
            pending_choices: Vec::new(),
            last_seen_hands: [None, None],
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
        if let Some(choice) = self.pending_choices.first() {
            match choice {
                PendingChoice::IronStar { player: deciding } if *deciding == player => {
                    actions.push(Action::ChooseTriggeredAbility {
                        pay: false,
                        new_target: None,
                    });
                    if self.players[player.index()].mana_pool.total() > 0 {
                        actions.push(Action::ChooseTriggeredAbility {
                            pay: true,
                            new_target: None,
                        });
                    }
                }
                PendingChoice::ChainLightning {
                    player: deciding, ..
                } if *deciding == player => {
                    actions.push(Action::ChooseTriggeredAbility {
                        pay: false,
                        new_target: None,
                    });
                    if self.players[player.index()].mana_pool.red >= 2 {
                        for target in self.damage_targets() {
                            actions.push(Action::ChooseTriggeredAbility {
                                pay: true,
                                new_target: Some(target),
                            });
                        }
                    }
                }
                PendingChoice::IronStar { .. } | PendingChoice::ChainLightning { .. } => {}
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
                actions.push(Action::FinishDeclaringAttackers);
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
        if !self.legal_actions(player).contains(&action) {
            return Err(ActionError::NotLegal { player, action });
        }

        match action {
            Action::KeepHand => self.keep_hand(player),
            Action::TakeMulligan => self.take_mulligan(player),
            Action::BottomCards { cards } => self.bottom_cards(player, &cards),
            Action::DiscardCards { cards } => self.discard_cards(player, &cards),
            Action::ChooseTriggeredAbility { pay, new_target } => {
                self.choose_triggered_ability(player, pay, new_target);
            }
            Action::ChooseUntap { permanents } => self.choose_untap(player, &permanents),
            Action::PassPriority => self.pass_priority(player),
            Action::PlayLand { card } => self.play_land(player, card),
            Action::ActivateManaAbility { source } => self.activate_mountain(player, source),
            Action::CastSpell { card, target, x } => self.cast_spell(player, card, target, x),
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
            Action::Concede => self.finish(GameResult::Winner {
                winner: player.opponent(),
                reason: WinReason::OpponentConceded,
            }),
        }
        Ok(())
    }

    #[must_use]
    pub fn observe(&self, viewer: PlayerId) -> PlayerObservation {
        let player = &self.players[viewer.index()];
        let opponent = &self.players[viewer.opponent().index()];
        PlayerObservation {
            viewer,
            turn: self.turn,
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
                    flying: permanent.flying_until_end,
                })
                .collect(),
            stack: self
                .stack
                .iter()
                .map(|object| StackObservation {
                    id: object.id,
                    card: object.card.id,
                    definition: object.card.definition,
                    controller: object.controller,
                    target: object.target,
                    x: object.x,
                })
                .collect(),
            result: self.result,
            legal_actions: self.legal_actions(viewer),
        }
    }

    fn add_mana_actions(&self, player: PlayerId, actions: &mut Vec<Action>) {
        actions.extend(
            self.battlefield
                .iter()
                .filter(|permanent| permanent.controller == player && !permanent.tapped)
                .filter(|permanent| {
                    self.behavior(permanent.card.definition) == Some(CardBehavior::Mountain)
                })
                .map(|permanent| Action::ActivateManaAbility {
                    source: permanent.card.id,
                }),
        );
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
        self.finish_cleanup();
    }

    fn choose_triggered_ability(
        &mut self,
        player: PlayerId,
        pay: bool,
        new_target: Option<Target>,
    ) {
        let choice = self.pending_choices.remove(0);
        match choice {
            PendingChoice::IronStar { .. } if pay => {
                pay_generic(&mut self.players[player.index()].mana_pool, 1);
                self.players[player.index()].life += 1;
            }
            PendingChoice::ChainLightning { mut spell, .. } if pay => {
                self.players[player.index()].mana_pool.red -= 2;
                spell.id = StackObjectId(self.next_stack_id);
                self.next_stack_id += 1;
                spell.controller = player;
                spell.target = new_target;
                spell.is_copy = true;
                self.stack.push(spell);
            }
            PendingChoice::IronStar { .. } | PendingChoice::ChainLightning { .. } => {}
        }
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
                .filter(|card| self.behavior(card.definition) == Some(CardBehavior::Mountain))
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
            if matches!(behavior, CardBehavior::Unsupported | CardBehavior::Mountain) {
                continue;
            }
            if !matches!(kind, CardKind::Instant)
                && (player != self.active_player || !self.step.is_main() || !self.stack.is_empty())
            {
                continue;
            }
            let cost = behavior.mana_cost();
            let max_x = if cost.variable_x {
                state
                    .mana_pool
                    .total()
                    .saturating_sub(cost.red + cost.generic)
            } else {
                0
            };
            for x in 0..=max_x {
                if !can_pay(state.mana_pool, cost, x) {
                    continue;
                }
                for target in self.spell_targets(behavior, x, player) {
                    actions.push(Action::CastSpell {
                        card: card.id,
                        target,
                        x,
                    });
                }
            }
        }
    }

    fn spell_targets(
        &self,
        behavior: CardBehavior,
        x: u16,
        player: PlayerId,
    ) -> Vec<Option<Target>> {
        match behavior {
            CardBehavior::LightningBolt | CardBehavior::ChainLightning | CardBehavior::Fireball => {
                let mut targets = vec![
                    Some(Target::Player(PlayerId::One)),
                    Some(Target::Player(PlayerId::Two)),
                ];
                targets.extend(
                    self.battlefield
                        .iter()
                        .filter(|permanent| self.power(permanent).is_some())
                        .map(|permanent| Some(Target::Permanent(permanent.card.id))),
                );
                targets
            }
            CardBehavior::Shatter => self
                .battlefield
                .iter()
                .filter(|permanent| {
                    self.kind(permanent.card.definition)
                        .is_some_and(CardKind::is_artifact)
                })
                .map(|permanent| Some(Target::Permanent(permanent.card.id)))
                .collect(),
            CardBehavior::Detonate => self
                .battlefield
                .iter()
                .filter(|permanent| {
                    self.kind(permanent.card.definition)
                        .is_some_and(CardKind::is_artifact)
                        && self.mana_value(permanent.card.definition) == x
                })
                .map(|permanent| Some(Target::Permanent(permanent.card.id)))
                .collect(),
            CardBehavior::Fork => self
                .stack
                .iter()
                .filter(|object| {
                    matches!(
                        self.kind(object.card.definition),
                        Some(CardKind::Instant | CardKind::Sorcery)
                    )
                })
                .map(|object| Some(Target::Spell(object.id)))
                .collect(),
            CardBehavior::RedElementalBlast => Vec::new(),
            CardBehavior::BlackVise => vec![Some(Target::Player(player.opponent()))],
            _ => vec![None],
        }
    }

    fn add_ability_actions(&self, player: PlayerId, actions: &mut Vec<Action>) {
        for permanent in self
            .battlefield
            .iter()
            .filter(|permanent| permanent.controller == player)
        {
            match self.behavior(permanent.card.definition) {
                Some(CardBehavior::Atog) => {
                    actions.extend(
                        self.battlefield
                            .iter()
                            .filter(|candidate| {
                                candidate.controller == player
                                    && self
                                        .kind(candidate.card.definition)
                                        .is_some_and(CardKind::is_artifact)
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
            .map_or(0, |cost| cost.generic + cost.red)
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

    fn activate_mountain(&mut self, player: PlayerId, source: CardInstanceId) {
        let permanent = self
            .battlefield
            .iter_mut()
            .find(|permanent| permanent.card.id == source)
            .expect("legal mana action references a permanent");
        permanent.tapped = true;
        self.players[player.index()].mana_pool.red += 1;
        self.consecutive_passes = 0;
        self.events.push(GameEvent::ManaAdded { player, source });
    }

    fn cast_spell(
        &mut self,
        player: PlayerId,
        card_id: CardInstanceId,
        target: Option<Target>,
        x: u16,
    ) {
        let card = remove_card(&mut self.players[player.index()].hand, card_id)
            .expect("legal cast action references a card in hand");
        let behavior = self.behavior(card.definition).expect("cataloged card");
        pay_cost(
            &mut self.players[player.index()].mana_pool,
            behavior.mana_cost(),
            x,
        );
        let stack_id = StackObjectId(self.next_stack_id);
        self.next_stack_id += 1;
        self.stack.push(StackObject {
            id: stack_id,
            card,
            controller: player,
            target,
            x,
            is_copy: false,
        });
        self.consecutive_passes = 0;
        self.events.push(GameEvent::SpellCast {
            player,
            card: card_id,
            target,
        });
        if behavior.is_red() {
            for permanent in &self.battlefield {
                if self.behavior(permanent.card.definition) == Some(CardBehavior::IronStar) {
                    self.pending_choices.push(PendingChoice::IronStar {
                        player: permanent.controller,
                    });
                }
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
        let behavior = self
            .behavior(object.card.definition)
            .expect("stack cards are cataloged");
        if behavior.kind().is_permanent() {
            let chosen_player = match object.target {
                Some(Target::Player(player)) => Some(player),
                _ => None,
            };
            self.battlefield.push(Permanent {
                card: object.card.clone(),
                controller: object.controller,
                tapped: false,
                entered_turn: self.turn,
                damage: 0,
                power_bonus: 0,
                toughness_bonus: 0,
                attacking: false,
                blocking: None,
                chosen_player,
                destroy_at_end: false,
                flying_until_end: false,
            });
        } else {
            self.resolve_spell_effect(&object, behavior);
        }
        let card_id = object.card.id;
        if !behavior.kind().is_permanent() && !object.is_copy {
            self.players[object.card.owner.index()]
                .graveyard
                .push(object.card);
        }
        self.events.push(GameEvent::SpellResolved { card: card_id });
        self.check_state_based_actions();
    }

    fn resolve_spell_effect(&mut self, object: &StackObject, behavior: CardBehavior) {
        match behavior {
            CardBehavior::LightningBolt => {
                self.damage_target(object.target, 3);
            }
            CardBehavior::ChainLightning => {
                let deciding = match object.target {
                    Some(Target::Player(player)) => Some(player),
                    Some(Target::Permanent(id)) => self.permanent_controller(id),
                    Some(Target::Spell(_)) | None => None,
                };
                self.damage_target(object.target, 3);
                if let Some(player) = deciding {
                    self.pending_choices.push(PendingChoice::ChainLightning {
                        player,
                        spell: object.clone(),
                    });
                }
            }
            CardBehavior::Fireball => self.damage_target(object.target, object.x),
            CardBehavior::Shatter => {
                if let Some(Target::Permanent(target)) = object.target {
                    self.destroy_permanent(target);
                }
            }
            CardBehavior::Detonate => {
                if let Some(Target::Permanent(target)) = object.target
                    && let Some(controller) = self.permanent_controller(target)
                {
                    self.destroy_permanent(target);
                    self.deal_damage(controller, object.x);
                }
            }
            CardBehavior::Fork => {
                if let Some(Target::Spell(target)) = object.target
                    && let Some(original) =
                        self.stack.iter().find(|item| item.id == target).cloned()
                {
                    let mut copy = original;
                    copy.id = StackObjectId(self.next_stack_id);
                    self.next_stack_id += 1;
                    copy.controller = object.controller;
                    copy.is_copy = true;
                    self.stack.push(copy);
                }
            }
            _ => {}
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
                .filter(|permanent| self.behavior(permanent.card.definition) == Some(behavior))
                .count(),
        )
        .unwrap_or(u16::MAX)
    }

    fn power(&self, permanent: &Permanent) -> Option<i16> {
        self.behavior(permanent.card.definition)
            .and_then(CardBehavior::creature_stats)
            .map(|stats| stats.power + permanent.power_bonus)
    }

    fn toughness(&self, permanent: &Permanent) -> Option<i16> {
        self.behavior(permanent.card.definition)
            .and_then(CardBehavior::creature_stats)
            .map(|stats| stats.toughness + permanent.toughness_bonus)
    }

    fn can_use_tap_ability(&self, permanent: &Permanent) -> bool {
        self.behavior(permanent.card.definition)
            .and_then(CardBehavior::creature_stats)
            .is_none_or(|stats| stats.haste || permanent.entered_turn < self.turn)
    }

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
            .and_then(|permanent| self.behavior(permanent.card.definition));
        match behavior {
            Some(CardBehavior::Atog) => {
                if let Some(sacrificed) = sacrifice {
                    self.destroy_permanent(sacrificed);
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
        self.behavior(permanent.card.definition)
            .and_then(CardBehavior::creature_stats)
            .is_some_and(|stats| stats.haste || permanent.entered_turn < self.turn)
    }

    fn declare_attacker(&mut self, attacker: CardInstanceId) {
        if let Some(permanent) = self
            .battlefield
            .iter_mut()
            .find(|permanent| permanent.card.id == attacker)
        {
            permanent.attacking = true;
            permanent.tapped = true;
        }
    }

    fn finish_declaring_attackers(&mut self) {
        self.attackers_declared = true;
        self.priority = self.active_player;
        self.consecutive_passes = 0;
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
            .map(|permanent| (permanent.card.id, permanent.flying_until_end))
            .collect();
        blockers
            .into_iter()
            .flat_map(|blocker| {
                attackers.iter().filter_map(move |(attacker, flying)| {
                    (!flying).then_some(Action::DeclareBlocker {
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
            let trample = self
                .behavior(self.battlefield[attacker_index].card.definition)
                .and_then(CardBehavior::creature_stats)
                .is_some_and(|stats| stats.trample);
            let mut blockers: Vec<_> = self
                .battlefield
                .iter()
                .filter(|permanent| permanent.blocking == Some(attacker_id))
                .map(|permanent| permanent.card.id)
                .collect();
            blockers.sort_unstable();
            if blockers.is_empty() {
                self.deal_damage(self.active_player.opponent(), power);
            } else {
                let mut remaining = power;
                for (index, blocker) in blockers.iter().enumerate() {
                    let existing_damage = self
                        .battlefield
                        .iter()
                        .find(|permanent| permanent.card.id == *blocker)
                        .map_or(0, |permanent| permanent.damage);
                    let lethal = self
                        .battlefield
                        .iter()
                        .find(|permanent| permanent.card.id == *blocker)
                        .and_then(|permanent| self.toughness(permanent))
                        .unwrap_or(0)
                        .max(0)
                        .cast_unsigned()
                        .saturating_sub(existing_damage);
                    let is_last = index + 1 == blockers.len();
                    let assigned = if is_last && !trample {
                        remaining
                    } else {
                        remaining.min(lethal)
                    };
                    self.damage_target(Some(Target::Permanent(*blocker)), assigned);
                    remaining -= assigned;
                }
                if trample && remaining > 0 {
                    self.deal_damage(self.active_player.opponent(), remaining);
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
        let permanent = self.battlefield.remove(index);
        if self.behavior(permanent.card.definition) == Some(CardBehavior::SuChi) {
            self.players[permanent.controller.index()]
                .mana_pool
                .colorless += 4;
        }
        self.players[permanent.card.owner.index()]
            .graveyard
            .push(permanent.card);
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
        self.handle_upkeep_triggers();
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
                if !(self.turn == 1 && self.active_player == PlayerId::One) {
                    self.draw_card(self.active_player);
                }
            }
            Step::Draw => self.step = Step::PrecombatMain,
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
                self.deal_combat_damage();
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
        self.active_player = self.active_player.opponent();
        self.step = Step::Upkeep;
        self.players[self.active_player.index()].land_played_this_turn = false;
        let winter_orb = self.winter_orb_active();
        let smoke = self.count_behavior(CardBehavior::Smoke) > 0;
        self.untap_pending = false;
        for permanent in &mut self.battlefield {
            if permanent.controller == self.active_player {
                let behavior = self
                    .catalog
                    .get(permanent.card.definition)
                    .map(|card| card.behavior);
                let restricted = (winter_orb && behavior == Some(CardBehavior::Mountain))
                    || (smoke && behavior.and_then(CardBehavior::creature_stats).is_some());
                if restricted && permanent.tapped {
                    self.untap_pending = true;
                } else {
                    permanent.tapped = false;
                }
            }
        }
        if !self.untap_pending {
            self.handle_upkeep_triggers();
        }
    }

    fn handle_upkeep_triggers(&mut self) {
        let player = self.active_player;
        let copper_damage = self.count_behavior(CardBehavior::CopperTablet);
        if copper_damage > 0 {
            self.deal_damage(player, copper_damage);
        }
        let vise_damage: u16 = self
            .battlefield
            .iter()
            .filter(|permanent| {
                self.behavior(permanent.card.definition) == Some(CardBehavior::BlackVise)
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
        self.check_life_totals();
    }

    fn handle_end_step(&mut self) {
        let doomed: Vec<_> = self
            .battlefield
            .iter()
            .filter(|permanent| {
                permanent.destroy_at_end
                    || self.behavior(permanent.card.definition) == Some(CardBehavior::BallLightning)
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
            self.finish_cleanup();
        }
    }

    fn finish_cleanup(&mut self) {
        for permanent in &mut self.battlefield {
            permanent.damage = 0;
            permanent.power_bonus = 0;
            permanent.toughness_bonus = 0;
            permanent.flying_until_end = false;
            permanent.destroy_at_end = false;
        }
    }

    fn clear_combat(&mut self) {
        for permanent in &mut self.battlefield {
            permanent.attacking = false;
            permanent.blocking = None;
        }
    }

    fn winter_orb_active(&self) -> bool {
        self.battlefield.iter().any(|permanent| {
            !permanent.tapped
                && self.behavior(permanent.card.definition) == Some(CardBehavior::WinterOrb)
        })
    }

    fn draw_card(&mut self, player: PlayerId) {
        let Some(card) = self.players[player.index()].library.pop() else {
            self.finish(GameResult::Winner {
                winner: player.opponent(),
                reason: WinReason::OpponentTriedToDrawFromEmptyLibrary,
            });
            return;
        };
        let card_id = card.id;
        self.players[player.index()].hand.push(card);
        self.events.push(GameEvent::CardDrawn {
            player,
            card: card_id,
        });
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
    pool.red >= cost.red && pool.total() >= cost.red.saturating_add(cost.generic).saturating_add(x)
}

fn pay_cost(pool: &mut ManaPool, cost: ManaCost, x: u16) {
    pool.red -= cost.red;
    let generic = cost.generic.saturating_add(x);
    let colorless = pool.colorless.min(generic);
    pool.colorless -= colorless;
    pool.red -= generic - colorless;
}

fn pay_generic(pool: &mut ManaPool, amount: u16) {
    let colorless = pool.colorless.min(amount);
    pool.colorless -= colorless;
    pool.red -= amount - colorless;
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
