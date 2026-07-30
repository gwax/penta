use std::error::Error;
use std::fmt;

use crate::action::{Action, ActionError, Target};
use crate::card::{CardBehavior, CardCatalog};
use crate::deck::{Deck, DeckError, ValidatedDeck};
use crate::ids::{CardDefinitionId, CardInstanceId, PlayerId};
use crate::rng::ReplayRng;
use crate::rules;

#[derive(Clone, Debug, Eq, PartialEq)]
struct CardInstance {
    id: CardInstanceId,
    definition: CardDefinitionId,
    owner: PlayerId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Permanent {
    card: CardInstance,
    controller: PlayerId,
    tapped: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StackObject {
    card: CardInstance,
    controller: PlayerId,
    target: Target,
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ManaPool {
    pub red: u16,
}

impl ManaPool {
    #[must_use]
    pub const fn total(self) -> u16 {
        self.red
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
}

impl Step {
    const fn is_main(self) -> bool {
        matches!(self, Self::PrecombatMain | Self::PostcombatMain)
    }

    const fn ends_phase(self) -> bool {
        matches!(
            self,
            Self::Draw | Self::PrecombatMain | Self::EndOfCombat | Self::PostcombatMain | Self::End
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
        target: Target,
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StackObservation {
    pub id: CardInstanceId,
    pub definition: CardDefinitionId,
    pub controller: PlayerId,
    pub target: Target,
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
    pub library_sizes: [usize; 2],
    pub graveyards: [Vec<(CardInstanceId, CardDefinitionId)>; 2],
    pub battlefield: Vec<PermanentObservation>,
    pub stack: Vec<StackObservation>,
    pub result: Option<GameResult>,
    pub legal_actions: Vec<Action>,
}

#[derive(Clone, Debug)]
pub struct Game {
    seed: u64,
    catalog: CardCatalog,
    players: [PlayerState; 2],
    battlefield: Vec<Permanent>,
    stack: Vec<StackObject>,
    turn: u32,
    active_player: PlayerId,
    priority: PlayerId,
    consecutive_passes: u8,
    step: Step,
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
            catalog,
            players,
            battlefield: Vec::new(),
            stack: Vec::new(),
            turn: 1,
            active_player: PlayerId::One,
            priority: PlayerId::One,
            consecutive_passes: 0,
            step: Step::Upkeep,
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
    pub fn legal_actions(&self, player: PlayerId) -> Vec<Action> {
        if self.result.is_some() {
            return Vec::new();
        }

        let mut actions = vec![Action::Concede];
        if player != self.priority {
            return actions;
        }

        actions.push(Action::PassPriority);
        self.add_mana_actions(player, &mut actions);
        self.add_land_actions(player, &mut actions);
        self.add_spell_actions(player, &mut actions);
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
            Action::PassPriority => self.pass_priority(player),
            Action::PlayLand { card } => self.play_land(player, card),
            Action::ActivateManaAbility { source } => self.activate_mountain(player, source),
            Action::CastSpell { card, target } => self.cast_spell(player, card, target),
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
                })
                .collect(),
            stack: self
                .stack
                .iter()
                .map(|object| StackObservation {
                    id: object.card.id,
                    definition: object.card.definition,
                    controller: object.controller,
                    target: object.target,
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
        if state.mana_pool.red == 0 {
            return;
        }
        for card in &state.hand {
            if self.behavior(card.definition) == Some(CardBehavior::LightningBolt) {
                actions.push(Action::CastSpell {
                    card: card.id,
                    target: Target::Player(PlayerId::One),
                });
                actions.push(Action::CastSpell {
                    card: card.id,
                    target: Target::Player(PlayerId::Two),
                });
            }
        }
    }

    fn behavior(&self, definition: CardDefinitionId) -> Option<CardBehavior> {
        self.catalog.get(definition).map(|card| card.behavior)
    }

    fn play_land(&mut self, player: PlayerId, card_id: CardInstanceId) {
        let card = remove_card(&mut self.players[player.index()].hand, card_id)
            .expect("legal land action references a card in hand");
        self.players[player.index()].land_played_this_turn = true;
        self.battlefield.push(Permanent {
            card,
            controller: player,
            tapped: false,
        });
        self.consecutive_passes = 0;
        self.events.push(GameEvent::LandPlayed {
            player,
            card: card_id,
        });
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

    fn cast_spell(&mut self, player: PlayerId, card_id: CardInstanceId, target: Target) {
        let card = remove_card(&mut self.players[player.index()].hand, card_id)
            .expect("legal cast action references a card in hand");
        self.players[player.index()].mana_pool.red -= 1;
        self.stack.push(StackObject {
            card,
            controller: player,
            target,
        });
        self.consecutive_passes = 0;
        self.events.push(GameEvent::SpellCast {
            player,
            card: card_id,
            target,
        });
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
        match self.behavior(object.card.definition) {
            Some(CardBehavior::LightningBolt) => {
                let Target::Player(target) = object.target;
                self.deal_damage(target, 3);
            }
            Some(CardBehavior::Mountain | CardBehavior::Unsupported) | None => {
                unreachable!("only supported spells can be cast")
            }
        }
        let card_id = object.card.id;
        self.players[object.card.owner.index()]
            .graveyard
            .push(object.card);
        self.events.push(GameEvent::SpellResolved { card: card_id });
        self.check_life_totals();
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
            Step::BeginningOfCombat => self.step = Step::DeclareAttackers,
            Step::DeclareAttackers => self.step = Step::DeclareBlockers,
            Step::DeclareBlockers => self.step = Step::CombatDamage,
            Step::CombatDamage => self.step = Step::EndOfCombat,
            Step::EndOfCombat => self.step = Step::PostcombatMain,
            Step::PostcombatMain => self.step = Step::End,
            Step::End => self.start_next_turn(),
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
        for permanent in &mut self.battlefield {
            if permanent.controller == self.active_player {
                permanent.tapped = false;
            }
        }
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

fn public_cards(cards: &[CardInstance]) -> Vec<(CardInstanceId, CardDefinitionId)> {
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
