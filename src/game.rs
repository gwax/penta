use std::error::Error;
use std::fmt;

use crate::action::{Action, ActionError, CombatDamageAssignment, Target};
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
    factory_animated: bool,
    dragon_whelp_activations: u8,
    combat_damage_assignment: Vec<CombatDamageAssignment>,
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
    Fork {
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
    pending_combat_attackers: Vec<CardInstanceId>,
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
            pending_combat_attackers: Vec::new(),
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
        if let Some(choice) = self.pending_choices.first() {
            return Some(match choice {
                PendingChoice::IronStar { player }
                | PendingChoice::ChainLightning { player, .. }
                | PendingChoice::Fork { player, .. } => *player,
            });
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
        if let Some(choice) = self.pending_choices.first() {
            match choice {
                PendingChoice::IronStar { player: deciding } if *deciding == player => {
                    actions.push(Action::ChooseTriggeredAbility {
                        pay: false,
                        new_targets: Vec::new(),
                    });
                    if self.players[player.index()].mana_pool.total() > 0 {
                        actions.push(Action::ChooseTriggeredAbility {
                            pay: true,
                            new_targets: Vec::new(),
                        });
                    }
                }
                PendingChoice::ChainLightning {
                    player: deciding,
                    spell,
                } if *deciding == player => {
                    actions.push(Action::ChooseTriggeredAbility {
                        pay: false,
                        new_targets: Vec::new(),
                    });
                    if self.players[player.index()].mana_pool.red >= 2 {
                        let mut targets = self.damage_targets();
                        if let Some(target) = spell.targets.first()
                            && !targets.contains(target)
                        {
                            targets.push(*target);
                        }
                        for target in targets {
                            actions.push(Action::ChooseTriggeredAbility {
                                pay: true,
                                new_targets: vec![target],
                            });
                        }
                    }
                }
                PendingChoice::Fork {
                    player: deciding,
                    spell,
                } if *deciding == player => {
                    let mut target_lists =
                        self.behavior(spell.card.definition)
                            .map_or_else(Vec::new, |behavior| {
                                self.legal_target_lists(
                                    behavior,
                                    spell.x,
                                    player,
                                    Some(spell.targets.len()),
                                )
                            });
                    target_lists.push(spell.targets.clone());
                    target_lists.sort_unstable();
                    target_lists.dedup();
                    actions.extend(
                        target_lists
                            .into_iter()
                            .map(|targets| Action::ChooseCopyTargets { targets }),
                    );
                }
                PendingChoice::IronStar { .. }
                | PendingChoice::ChainLightning { .. }
                | PendingChoice::Fork { .. } => {}
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
            Action::ChooseTriggeredAbility { pay, new_targets } => {
                self.choose_triggered_ability(player, pay, &new_targets);
            }
            Action::ChooseCopyTargets { targets } => self.choose_copy_targets(player, targets),
            Action::ChooseUntap { permanents } => self.choose_untap(player, &permanents),
            Action::PassPriority => self.pass_priority(player),
            Action::PlayLand { card } => self.play_land(player, card),
            Action::ActivateManaAbility { source } => self.activate_mana_source(player, source),
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
            result: self.result,
            legal_actions: self.legal_actions(viewer),
        }
    }

    fn add_mana_actions(&self, player: PlayerId, actions: &mut Vec<Action>) {
        actions.extend(
            self.battlefield
                .iter()
                .filter(|permanent| permanent.controller == player && !permanent.tapped)
                .filter(|permanent| self.mana_production(permanent).is_some())
                .filter(|permanent| self.can_use_tap_ability(permanent))
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

    fn choose_triggered_ability(&mut self, player: PlayerId, pay: bool, new_targets: &[Target]) {
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
                spell.targets = new_targets.to_vec();
                spell.is_copy = true;
                self.stack.push(spell);
            }
            PendingChoice::IronStar { .. }
            | PendingChoice::ChainLightning { .. }
            | PendingChoice::Fork { .. } => {}
        }
    }

    fn choose_copy_targets(&mut self, player: PlayerId, targets: Vec<Target>) {
        let PendingChoice::Fork { mut spell, .. } = self.pending_choices.remove(0) else {
            return;
        };
        spell.id = StackObjectId(self.next_stack_id);
        self.next_stack_id += 1;
        spell.controller = player;
        spell.targets = targets;
        spell.is_copy = true;
        self.stack.push(spell);
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
                state.mana_pool.total()
            } else {
                0
            };
            for x in 0..=max_x {
                let target_counts: Vec<_> = if behavior == CardBehavior::Fireball {
                    (1..=self.damage_targets().len())
                        .filter(|count| {
                            can_pay(
                                state.mana_pool,
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
                        if !can_pay(state.mana_pool, add_generic(cost, extra), x) {
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

    fn legal_target_lists(
        &self,
        behavior: CardBehavior,
        x: u16,
        player: PlayerId,
        exact_count: Option<usize>,
    ) -> Vec<Vec<Target>> {
        match behavior {
            CardBehavior::LightningBolt
            | CardBehavior::ChainLightning
            | CardBehavior::GoblinGrenade => self
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
            CardBehavior::Shatter => self
                .battlefield
                .iter()
                .filter(|permanent| {
                    self.kind(permanent.card.definition)
                        .is_some_and(CardKind::is_artifact)
                })
                .map(|permanent| vec![Target::Permanent(permanent.card.id)])
                .collect(),
            CardBehavior::Detonate => self
                .battlefield
                .iter()
                .filter(|permanent| {
                    self.kind(permanent.card.definition)
                        .is_some_and(CardKind::is_artifact)
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
            CardBehavior::RedElementalBlast => Vec::new(),
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
                    if self.players[player.index()].mana_pool.total() > 0 =>
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
                    if !permanent.tapped && self.players[player.index()].mana_pool.total() > 0 =>
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
                            && self
                                .kind(candidate.card.definition)
                                .is_some_and(CardKind::is_artifact)
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
            factory_animated: false,
            dragon_whelp_activations: 0,
            combat_damage_assignment: Vec::new(),
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

    fn activate_mana_source(&mut self, player: PlayerId, source: CardInstanceId) {
        let production = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == source)
            .and_then(|permanent| self.mana_production(permanent))
            .expect("legal mana action references a mana source");
        let is_lotus = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == source)
            .is_some_and(|permanent| {
                self.behavior(permanent.card.definition) == Some(CardBehavior::BlackLotus)
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
        self.players[player.index()].mana_pool.red += production.red;
        self.players[player.index()].mana_pool.colorless += production.colorless;
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
        let card = remove_card(&mut self.players[player.index()].hand, card_id)
            .expect("legal cast action references a card in hand");
        let behavior = self.behavior(card.definition).expect("cataloged card");
        pay_cost(
            &mut self.players[player.index()].mana_pool,
            add_generic(
                behavior.mana_cost(),
                fireball_extra_cost(behavior, targets.len()),
            ),
            x,
        );
        for sacrificed in sacrifices {
            self.destroy_permanent(*sacrificed);
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
                factory_animated: false,
                dragon_whelp_activations: 0,
                combat_damage_assignment: Vec::new(),
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

    fn resolve_activated_ability(&mut self, object: &StackObject) {
        if self.behavior(object.card.definition) != Some(CardBehavior::ChaosOrb)
            || !self
                .battlefield
                .iter()
                .any(|permanent| permanent.card.id == object.card.id)
        {
            return;
        }
        if let Some(chosen) = object.chosen_permanents.first().copied() {
            self.destroy_permanent(chosen);
        }
        self.destroy_permanent(object.card.id);
    }

    fn resolve_spell_effect(&mut self, object: &StackObject, behavior: CardBehavior) {
        match behavior {
            CardBehavior::LightningBolt => {
                self.damage_target(object.targets.first().copied(), 3);
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
                    self.pending_choices.push(PendingChoice::ChainLightning {
                        player,
                        spell: object.clone(),
                    });
                }
            }
            CardBehavior::Fireball => {
                let divisor = u16::try_from(object.targets.len()).unwrap_or(u16::MAX);
                let amount = object.x.checked_div(divisor).unwrap_or(0);
                for target in &object.targets {
                    self.damage_target(Some(*target), amount);
                }
            }
            CardBehavior::Shatter => {
                if let Some(Target::Permanent(target)) = object.targets.first() {
                    self.destroy_permanent(*target);
                }
            }
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
                    self.pending_choices.push(PendingChoice::Fork {
                        player: object.controller,
                        spell: original,
                    });
                }
            }
            CardBehavior::WheelOfFortune => self.resolve_wheel_of_fortune(),
            _ => {}
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
                .filter(|permanent| self.behavior(permanent.card.definition) == Some(behavior))
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

    fn effective_behavior(&self, permanent: &Permanent) -> Option<CardBehavior> {
        if self.blood_moon_active() && self.is_nonbasic_land(permanent) {
            Some(CardBehavior::Mountain)
        } else {
            self.behavior(permanent.card.definition)
        }
    }

    fn mana_production(&self, permanent: &Permanent) -> Option<ManaPool> {
        match self.effective_behavior(permanent)? {
            CardBehavior::Mountain | CardBehavior::MoxRuby => Some(ManaPool {
                red: 1,
                colorless: 0,
            }),
            CardBehavior::MishrasFactory
            | CardBehavior::MoxEmerald
            | CardBehavior::MoxJet
            | CardBehavior::MoxPearl
            | CardBehavior::MoxSapphire
            | CardBehavior::StripMine => Some(ManaPool {
                red: 0,
                colorless: 1,
            }),
            CardBehavior::SolRing => Some(ManaPool {
                red: 0,
                colorless: 2,
            }),
            CardBehavior::BlackLotus => Some(ManaPool {
                red: 3,
                colorless: 0,
            }),
            _ => None,
        }
    }

    fn base_stats(&self, permanent: &Permanent) -> Option<crate::CreatureStats> {
        if self.effective_behavior(permanent) == Some(CardBehavior::MishrasFactory)
            && permanent.factory_animated
        {
            Some(crate::CreatureStats {
                power: 2,
                toughness: 2,
                haste: false,
                trample: false,
            })
        } else {
            self.effective_behavior(permanent)
                .and_then(CardBehavior::creature_stats)
        }
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

    fn power(&self, permanent: &Permanent) -> Option<i16> {
        self.base_stats(permanent)
            .map(|stats| stats.power + permanent.power_bonus + self.goblin_bonus(permanent))
    }

    fn toughness(&self, permanent: &Permanent) -> Option<i16> {
        self.base_stats(permanent)
            .map(|stats| stats.toughness + permanent.toughness_bonus + self.goblin_bonus(permanent))
    }

    fn has_flying(&self, permanent: &Permanent) -> bool {
        permanent.flying_until_end
            || self
                .effective_behavior(permanent)
                .is_some_and(CardBehavior::has_flying)
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
                    pay_generic(&mut self.players[player.index()].mana_pool, 1);
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
                    self.destroy_permanent(source);
                    self.destroy_permanent(target);
                }
            }
            Some(CardBehavior::ChaosOrb) => {
                pay_generic(&mut self.players[player.index()].mana_pool, 1);
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
                if let Some(permanent) = self
                    .battlefield
                    .iter_mut()
                    .find(|permanent| permanent.card.id == source)
                {
                    permanent.tapped = true;
                }
                if let Some(sacrificed) = sacrifice {
                    self.destroy_permanent(sacrificed);
                }
                self.damage_target(target, 2);
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
        self.base_stats(permanent)
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
                        let can_block = !(*unblockable
                            || *flying && !blocker_flying
                            || ironclaw && *power >= 2);
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
                let trample = self.base_stats(attacker).is_some_and(|stats| stats.trample);
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
        let trample = self.base_stats(attacker).is_some_and(|stats| stats.trample);
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
        self.active_player = self.active_player.opponent();
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
        self.untap_pending = false;
        for permanent in &mut self.battlefield {
            if permanent.controller == self.active_player {
                let restricted = (winter_orb && restricted_lands.contains(&permanent.card.id))
                    || (smoke && restricted_creatures.contains(&permanent.card.id));
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
            permanent.factory_animated = false;
            permanent.dragon_whelp_activations = 0;
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
        game.pending_choices.clear();
        game.pending_combat_attackers.clear();
        for player in &mut game.players {
            player.hand.clear();
            player.graveyard.clear();
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
            combat_damage_assignment: Vec::new(),
        }
    }

    fn pass_priority_pair(game: &mut Game) {
        let first = game.priority;
        game.apply(first, Action::PassPriority).unwrap();
        game.apply(first.opponent(), Action::PassPriority).unwrap();
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

        let retarget = Action::ChooseCopyTargets {
            targets: vec![Target::Player(PlayerId::Two)],
        };
        assert!(game.legal_actions(PlayerId::One).contains(&retarget));
        game.apply(PlayerId::One, retarget).unwrap();
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
        game.pending_choices.push(PendingChoice::Fork {
            player: PlayerId::One,
            spell: StackObject {
                id: StackObjectId(77),
                kind: StackObjectKind::Spell,
                card: card(10_001, cards::SHATTER, PlayerId::Two),
                controller: PlayerId::Two,
                targets: vec![stale_target],
                chosen_permanents: Vec::new(),
                x: 0,
                is_copy: false,
            },
        });

        assert!(
            game.legal_actions(PlayerId::One)
                .contains(&Action::ChooseCopyTargets {
                    targets: vec![stale_target],
                })
        );
    }

    #[test]
    fn black_lotus_sacrifices_for_three_red_mana() {
        let mut game = ready_game();
        let lotus = creature(10_000, cards::BLACK_LOTUS, PlayerId::One);
        let lotus_id = lotus.card.id;
        game.battlefield.push(lotus);
        let action = Action::ActivateManaAbility { source: lotus_id };
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
        assert!(
            game.battlefield
                .iter()
                .all(|permanent| ![strip_id, opposing_id].contains(&permanent.card.id))
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
}
