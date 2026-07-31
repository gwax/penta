//! Bot policies and deterministic game-running utilities.

use std::error::Error;
use std::fmt;

use crate::card::{CardBehavior, CardCatalog};
use crate::game::{Game, GameResult, PlayerObservation, Step};
use crate::{Action, ActionError, CardDefinitionId, CardInstanceId, PlayerId, Target};

/// Chooses one of the actions in a player's current observation.
pub trait Policy {
    fn choose_action(&mut self, observation: &PlayerObservation) -> Option<Action>;
}

/// Selects uniformly from the non-concession legal actions using a seeded PRNG.
#[derive(Clone, Debug)]
pub struct RandomPolicy {
    state: u64,
}

impl RandomPolicy {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

impl Policy for RandomPolicy {
    fn choose_action(&mut self, observation: &PlayerObservation) -> Option<Action> {
        let choices: Vec<_> = observation
            .legal_actions
            .iter()
            .filter(|action| !matches!(action, Action::Concede))
            .collect();
        if choices.is_empty() {
            return observation.legal_actions.first().cloned();
        }
        let choice_count = u64::try_from(choices.len()).unwrap_or(u64::MAX);
        let unbiased_range = u64::MAX - u64::MAX % choice_count;
        loop {
            let value = self.next_u64();
            if value < unbiased_range {
                let index = usize::try_from(value % choice_count).unwrap_or(0);
                return Some(choices[index].clone());
            }
        }
    }
}

/// A deterministic baseline that applies simple card- and combat-aware rules.
#[derive(Clone, Debug)]
pub struct HandcraftedPolicy {
    catalog: CardCatalog,
    mulligans_taken: u8,
}

impl HandcraftedPolicy {
    #[must_use]
    pub fn new(catalog: CardCatalog) -> Self {
        Self {
            catalog,
            mulligans_taken: 0,
        }
    }

    fn behavior(&self, definition: CardDefinitionId) -> Option<CardBehavior> {
        self.catalog.get(definition).map(|card| card.behavior)
    }

    fn hand_definition(
        observation: &PlayerObservation,
        id: CardInstanceId,
    ) -> Option<CardDefinitionId> {
        observation
            .hand
            .iter()
            .find_map(|(candidate, definition)| (*candidate == id).then_some(*definition))
    }

    fn permanent_definition(
        observation: &PlayerObservation,
        id: CardInstanceId,
    ) -> Option<CardDefinitionId> {
        observation
            .battlefield
            .iter()
            .find_map(|permanent| (permanent.id == id).then_some(permanent.definition))
    }

    fn target_score(observation: &PlayerObservation, target: Target) -> i32 {
        match target {
            Target::Player(player) if player == observation.viewer.opponent() => 500,
            Target::Player(_) => -10_000,
            Target::Permanent(id) => observation
                .battlefield
                .iter()
                .find(|permanent| permanent.id == id)
                .map_or(-500, |permanent| {
                    if permanent.controller == observation.viewer {
                        -500
                    } else {
                        250
                    }
                }),
            Target::Spell(_) => 100,
        }
    }

    fn damage_target_score(observation: &PlayerObservation, target: Target, amount: u16) -> i32 {
        match target {
            Target::Player(player) if player == observation.viewer.opponent() => {
                if observation.life_totals[player.index()]
                    <= i16::try_from(amount).unwrap_or(i16::MAX)
                {
                    10_000
                } else {
                    -2_000
                }
            }
            Target::Player(_) => -10_000,
            Target::Permanent(id) => observation
                .battlefield
                .iter()
                .find(|permanent| permanent.id == id)
                .map_or(-500, |permanent| {
                    if permanent.controller == observation.viewer {
                        return -10_000;
                    }
                    let remaining = permanent
                        .toughness
                        .unwrap_or(0)
                        .saturating_sub(i16::try_from(permanent.damage).unwrap_or(i16::MAX));
                    if i16::try_from(amount).unwrap_or(i16::MAX) >= remaining {
                        700 + i32::from(permanent.power.unwrap_or(0).max(0)) * 25
                    } else {
                        100
                    }
                }),
            Target::Spell(_) => -500,
        }
    }

    fn card_value(&self, definition: CardDefinitionId) -> i32 {
        match self.behavior(definition) {
            Some(CardBehavior::BlackLotus) => 100,
            Some(
                CardBehavior::MoxRuby
                | CardBehavior::MoxEmerald
                | CardBehavior::MoxJet
                | CardBehavior::MoxPearl
                | CardBehavior::MoxSapphire
                | CardBehavior::SolRing,
            ) => 90,
            Some(
                CardBehavior::Mountain | CardBehavior::MishrasFactory | CardBehavior::StripMine,
            ) => 80,
            Some(CardBehavior::LightningBolt | CardBehavior::GoblinGrenade) => 75,
            Some(behavior) if behavior.kind().is_creature() => 65,
            Some(_) => 55,
            None => 0,
        }
    }

    fn score_cast(
        &self,
        observation: &PlayerObservation,
        card: CardInstanceId,
        targets: &[Target],
        x: u16,
    ) -> i32 {
        let behavior = Self::hand_definition(observation, card).and_then(|id| self.behavior(id));
        let damage = match behavior {
            Some(CardBehavior::LightningBolt | CardBehavior::ChainLightning) => Some(3),
            Some(CardBehavior::GoblinGrenade) => Some(5),
            Some(CardBehavior::Fireball) => Some(
                x.checked_div(u16::try_from(targets.len()).unwrap_or(u16::MAX))
                    .unwrap_or(0),
            ),
            _ => None,
        };
        let target_score: i32 = targets
            .iter()
            .map(|target| {
                damage.map_or_else(
                    || Self::target_score(observation, *target),
                    |amount| Self::damage_target_score(observation, *target, amount),
                )
            })
            .sum();
        let base = match behavior {
            Some(CardBehavior::GoblinGrenade) => 8_500,
            Some(CardBehavior::LightningBolt | CardBehavior::ChainLightning) => 8_000,
            Some(CardBehavior::Fireball) => 7_900 + i32::from(x) * 20,
            Some(CardBehavior::Shatter | CardBehavior::Detonate | CardBehavior::ChaosOrb) => 7_400,
            Some(CardBehavior::Fork) => 7_300,
            Some(CardBehavior::WheelOfFortune) => 6_600,
            Some(behavior) if behavior.kind().is_permanent() => 6_800,
            Some(_) => 6_200,
            None => -10_000,
        };
        base + target_score
    }

    fn score_ability(
        &self,
        observation: &PlayerObservation,
        source: CardInstanceId,
        target: Option<Target>,
        sacrifice: Option<CardInstanceId>,
    ) -> i32 {
        let behavior =
            Self::permanent_definition(observation, source).and_then(|id| self.behavior(id));
        let target_score = target.map_or(0, |value| {
            if behavior == Some(CardBehavior::OrcishMechanics) {
                Self::damage_target_score(observation, value, 2)
            } else {
                Self::target_score(observation, value)
            }
        });
        let sacrifice_cost = sacrifice
            .filter(|card| *card != source)
            .and_then(|card| Self::permanent_definition(observation, card))
            .map_or(0, |definition| self.card_value(definition));
        let score = match behavior {
            Some(
                CardBehavior::ChaosOrb | CardBehavior::StripMine | CardBehavior::OrcishMechanics,
            ) => 7_200 + target_score,
            Some(CardBehavior::MishrasFactory) => 5_800 + target_score,
            Some(
                CardBehavior::GoblinBalloonBrigade
                | CardBehavior::GraniteGargoyle
                | CardBehavior::DragonWhelp,
            ) => 5_200,
            Some(CardBehavior::Atog) if self.atog_can_attack_for_lethal(observation, source) => {
                10_000
            }
            Some(CardBehavior::Atog) => -100,
            Some(_) => 4_500 + target_score,
            None => -10_000,
        };
        if behavior == Some(CardBehavior::OrcishMechanics)
            && matches!(target, Some(Target::Player(player)) if player == observation.viewer.opponent())
            && observation.life_totals[observation.viewer.opponent().index()] > 2
        {
            return -1_000;
        }
        score - sacrifice_cost
    }

    fn atog_can_attack_for_lethal(
        &self,
        observation: &PlayerObservation,
        source: CardInstanceId,
    ) -> bool {
        let Some(atog) = observation
            .battlefield
            .iter()
            .find(|permanent| permanent.id == source)
        else {
            return false;
        };
        if !atog.attacking
            || observation
                .battlefield
                .iter()
                .any(|permanent| permanent.blocking == Some(source))
        {
            return false;
        }
        let artifacts = observation
            .battlefield
            .iter()
            .filter(|permanent| {
                permanent.controller == observation.viewer
                    && self
                        .behavior(permanent.definition)
                        .is_some_and(|behavior| behavior.kind().is_artifact())
            })
            .count();
        let potential_power = atog
            .power
            .unwrap_or(0)
            .saturating_add(i16::try_from(artifacts.saturating_mul(2)).unwrap_or(i16::MAX));
        potential_power >= observation.life_totals[observation.viewer.opponent().index()]
    }

    fn score_attack(&self, observation: &PlayerObservation, attacker: CardInstanceId) -> i32 {
        let Some(attacker) = observation
            .battlefield
            .iter()
            .find(|permanent| permanent.id == attacker)
        else {
            return -1_000;
        };
        let attacker_power = attacker.power.unwrap_or(0).max(0);
        let attacker_toughness = attacker
            .toughness
            .unwrap_or(0)
            .saturating_sub(i16::try_from(attacker.damage).unwrap_or(i16::MAX));
        let already_attacking = observation
            .battlefield
            .iter()
            .filter(|permanent| permanent.controller == observation.viewer && permanent.attacking)
            .count();
        let blockers: Vec<_> = observation
            .battlefield
            .iter()
            .filter(|permanent| {
                permanent.controller == observation.viewer.opponent()
                    && !permanent.tapped
                    && permanent.power.is_some()
                    && (!attacker.flying || permanent.flying)
                    && !(self.behavior(permanent.definition) == Some(CardBehavior::IronclawOrcs)
                        && attacker_power >= 2)
            })
            .collect();
        if already_attacking >= blockers.len() {
            return 7_000;
        }
        let gets_eaten = blockers.iter().any(|blocker| {
            let blocker_power = blocker.power.unwrap_or(0).max(0);
            let blocker_toughness = blocker
                .toughness
                .unwrap_or(0)
                .saturating_sub(i16::try_from(blocker.damage).unwrap_or(i16::MAX));
            blocker_power >= attacker_toughness && blocker_toughness > attacker_power
        });
        if gets_eaten { 500 } else { 7_000 }
    }

    fn score_block(
        observation: &PlayerObservation,
        blocker: CardInstanceId,
        attacker: CardInstanceId,
    ) -> i32 {
        let blocker = observation
            .battlefield
            .iter()
            .find(|permanent| permanent.id == blocker);
        let attacker = observation
            .battlefield
            .iter()
            .find(|permanent| permanent.id == attacker);
        let (Some(blocker), Some(attacker)) = (blocker, attacker) else {
            return 0;
        };
        let blocker_power = blocker.power.unwrap_or(0);
        let blocker_toughness =
            blocker.toughness.unwrap_or(0) - i16::try_from(blocker.damage).unwrap_or(i16::MAX);
        let attacker_power = attacker.power.unwrap_or(0);
        let attacker_toughness =
            attacker.toughness.unwrap_or(0) - i16::try_from(attacker.damage).unwrap_or(i16::MAX);
        let existing_power: i16 = observation
            .battlefield
            .iter()
            .filter(|permanent| permanent.blocking == Some(attacker.id))
            .filter_map(|permanent| permanent.power)
            .fold(0, i16::saturating_add);
        if existing_power >= attacker_toughness {
            return 0;
        }
        let kills = existing_power.saturating_add(blocker_power) >= attacker_toughness;
        let survives = blocker_toughness > attacker_power;
        match (kills, survives) {
            (true, true) => 7_000,
            (true, false) => 6_000,
            (false, true) => 4_000,
            (false, false) if attacker_power >= 4 => 2_000,
            (false, false) => 500,
        }
    }

    fn score_assignment(
        observation: &PlayerObservation,
        assignments: &[crate::CombatDamageAssignment],
    ) -> i32 {
        assignments
            .iter()
            .map(|assignment| match assignment.recipient {
                Target::Player(player) if player == observation.viewer.opponent() => {
                    i32::from(assignment.amount) * 200
                }
                Target::Permanent(id) => observation
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.id == id)
                    .map_or(0, |permanent| {
                        let remaining = permanent
                            .toughness
                            .unwrap_or(0)
                            .saturating_sub(i16::try_from(permanent.damage).unwrap_or(i16::MAX));
                        if i16::try_from(assignment.amount).unwrap_or(i16::MAX) >= remaining {
                            500
                        } else {
                            i32::from(assignment.amount)
                        }
                    }),
                Target::Player(_) | Target::Spell(_) => 0,
            })
            .sum()
    }

    fn should_mulligan(&self, observation: &PlayerObservation) -> bool {
        if self.mulligans_taken >= 2 {
            return false;
        }
        let mana_sources = observation
            .hand
            .iter()
            .filter(|(_, definition)| {
                matches!(
                    self.behavior(*definition),
                    Some(
                        CardBehavior::Mountain
                            | CardBehavior::MishrasFactory
                            | CardBehavior::MoxEmerald
                            | CardBehavior::MoxJet
                            | CardBehavior::MoxPearl
                            | CardBehavior::MoxRuby
                            | CardBehavior::MoxSapphire
                            | CardBehavior::BlackLotus
                    )
                )
            })
            .count();
        !(2..=5).contains(&mana_sources)
    }

    fn mana_action_score(&self, observation: &PlayerObservation, source: CardInstanceId) -> i32 {
        let needs_factory_mana = observation.active_player == observation.viewer
            && matches!(
                observation.step,
                Step::BeginningOfCombat | Step::DeclareAttackers
            )
            && observation.mana_pools[observation.viewer.index()].total() == 0
            && observation.battlefield.iter().any(|permanent| {
                permanent.controller == observation.viewer
                    && !permanent.tapped
                    && permanent.power.is_none()
                    && self.behavior(permanent.definition) == Some(CardBehavior::MishrasFactory)
            });
        if needs_factory_mana
            && Self::permanent_definition(observation, source)
                .and_then(|definition| self.behavior(definition))
                != Some(CardBehavior::MishrasFactory)
        {
            8_800
        } else {
            -100
        }
    }

    fn score_land(&self, observation: &PlayerObservation, card: CardInstanceId) -> i32 {
        match Self::hand_definition(observation, card).and_then(|id| self.behavior(id)) {
            Some(CardBehavior::Mountain) => 9_300,
            Some(CardBehavior::MishrasFactory) => 9_200,
            Some(CardBehavior::StripMine) => 9_100,
            Some(_) | None => 9_000,
        }
    }

    fn score_untap(&self, observation: &PlayerObservation, permanents: &[CardInstanceId]) -> i32 {
        8_000
            + permanents
                .iter()
                .filter_map(|id| {
                    observation
                        .battlefield
                        .iter()
                        .find(|permanent| permanent.id == *id)
                })
                .map(|permanent| {
                    let card = self.card_value(permanent.definition);
                    let power = i32::from(permanent.power.unwrap_or(0).max(0));
                    card + power * 10
                })
                .sum::<i32>()
    }

    fn score_action(&self, observation: &PlayerObservation, action: &Action) -> i32 {
        match action {
            Action::KeepHand => 10_000,
            Action::TakeMulligan if self.should_mulligan(observation) => 11_000,
            Action::TakeMulligan => -5_000,
            Action::BottomCards { cards } | Action::DiscardCards { cards } => {
                9_000
                    - cards
                        .iter()
                        .filter_map(|card| Self::hand_definition(observation, *card))
                        .map(|definition| self.card_value(definition))
                        .sum::<i32>()
            }
            Action::ChooseTriggeredAbility { pay, new_targets } => {
                if *pay {
                    7_000
                        + new_targets
                            .iter()
                            .map(|target| Self::target_score(observation, *target))
                            .sum::<i32>()
                } else {
                    6_000
                }
            }
            Action::ChooseCopyTargets { targets } => {
                7_500
                    + targets
                        .iter()
                        .map(|target| Self::target_score(observation, *target))
                        .sum::<i32>()
            }
            Action::ChooseUntap { permanents } => self.score_untap(observation, permanents),
            Action::PlayLand { card } => self.score_land(observation, *card),
            Action::ActivateManaAbility { source } => self.mana_action_score(observation, *source),
            Action::CastSpell {
                card, targets, x, ..
            } => self.score_cast(observation, *card, targets, *x),
            Action::ActivateAbility {
                source,
                target,
                sacrifice,
            } => self.score_ability(observation, *source, *target, *sacrifice),
            Action::DeclareAttacker { attacker } => self.score_attack(observation, *attacker),
            Action::DeclareBlocker { blocker, attacker } => {
                Self::score_block(observation, *blocker, *attacker)
            }
            Action::FinishDeclaringAttackers | Action::FinishDeclaringBlockers => 1_000,
            Action::AssignCombatDamage { assignments, .. } => {
                6_000 + Self::score_assignment(observation, assignments)
            }
            Action::PassPriority => 0,
            Action::Concede => i32::MIN,
        }
    }
}

impl Policy for HandcraftedPolicy {
    fn choose_action(&mut self, observation: &PlayerObservation) -> Option<Action> {
        let action = observation
            .legal_actions
            .iter()
            .max_by_key(|action| self.score_action(observation, action))
            .cloned();
        if matches!(action, Some(Action::TakeMulligan)) {
            self.mulligans_taken += 1;
        } else if matches!(action, Some(Action::KeepHand)) {
            self.mulligans_taken = 0;
        }
        action
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlayError {
    PolicyReturnedNoAction(PlayerId),
    IllegalAction(ActionError),
    ActionLimitExceeded(usize),
}

impl fmt::Display for PlayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PolicyReturnedNoAction(player) => {
                write!(formatter, "policy for {player} returned no action")
            }
            Self::IllegalAction(error) => {
                write!(formatter, "policy returned an illegal action: {error}")
            }
            Self::ActionLimitExceeded(limit) => {
                write!(formatter, "game exceeded its action limit of {limit}")
            }
        }
    }
}

impl Error for PlayError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::IllegalAction(error) => Some(error),
            Self::PolicyReturnedNoAction(_) | Self::ActionLimitExceeded(_) => None,
        }
    }
}

/// Plays a game to completion using one policy for each player.
///
/// # Errors
///
/// Returns [`PlayError`] if a policy fails to choose an action, chooses an
/// illegal action, or the game exceeds `action_limit`.
pub fn play_game(
    game: &mut Game,
    player_one: &mut dyn Policy,
    player_two: &mut dyn Policy,
    action_limit: usize,
) -> Result<GameResult, PlayError> {
    for _ in 0..action_limit {
        if let Some(result) = game.result() {
            return Ok(result);
        }
        let Some(player) = game.decision_player() else {
            return game
                .result()
                .ok_or(PlayError::ActionLimitExceeded(action_limit));
        };
        let observation = game.observe(player);
        let action = match player {
            PlayerId::One => player_one.choose_action(&observation),
            PlayerId::Two => player_two.choose_action(&observation),
        }
        .ok_or(PlayError::PolicyReturnedNoAction(player))?;
        game.apply(player, action)
            .map_err(PlayError::IllegalAction)?;
    }
    game.result()
        .ok_or(PlayError::ActionLimitExceeded(action_limit))
}
