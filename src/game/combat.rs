use super::{
    Action, AppliedRuleDef, AttackDefender, CardBehavior, CardType, CombatDamageAssignment,
    CombatDamageStage, CommittedTriggerEvent, ControlFlow, CounterKind, DeclarativeAbilityDef,
    EffectDef, Game, GameEvent, GameObjectId, KeywordAbility, Permanent, PlayerId, Target,
};

mod damage_delivery;

impl Game {
    pub(super) fn attacker_actions(&self, player: PlayerId, moat_active: bool) -> Vec<Action> {
        let mut defenders = vec![AttackDefender::Player(player.opponent())];
        defenders.extend(
            self.battlefield
                .iter()
                .filter(|permanent| {
                    permanent.controller == player.opponent()
                        && self
                            .permanent_types(permanent)
                            .is_some_and(|types| types.contains(CardType::Planeswalker))
                })
                .map(|permanent| AttackDefender::Planeswalker(permanent.card.id)),
        );
        self.battlefield
            .iter()
            .filter(|permanent| {
                permanent.controller == player
                    && !permanent.tapped
                    && !permanent.attacking
                    && permanent.detained_until_turn_of.is_none()
                    && self.can_attack_with_moat(permanent, moat_active)
            })
            .flat_map(|permanent| {
                defenders
                    .iter()
                    .copied()
                    .map(|defender| Action::DeclareAttacker {
                        attacker: permanent.card.id,
                        defender,
                    })
            })
            .collect()
    }

    #[cfg(test)]
    pub(super) fn can_attack(&self, permanent: &Permanent) -> bool {
        let moat_active = self.count_behavior(CardBehavior::Moat) > 0;
        self.can_attack_with_moat(permanent, moat_active)
    }

    pub(super) fn can_attack_with_moat(&self, permanent: &Permanent, moat_active: bool) -> bool {
        if self.base_stats(permanent).is_none() {
            return false;
        }
        let flying = moat_active && self.has_flying(permanent);
        self.can_attack_creature(permanent, moat_active, flying)
    }

    pub(super) fn can_attack_creature(
        &self,
        permanent: &Permanent,
        moat_active: bool,
        flying: bool,
    ) -> bool {
        if self.permanent_has_executable_keyword(permanent, KeywordAbility::Defender) {
            return false;
        }
        if moat_active && !flying {
            return false;
        }
        if !self.attack_restrictions_met(permanent) || self.cannot_attack(permanent) {
            return false;
        }
        self.permanent_has_executable_keyword(permanent, KeywordAbility::Haste)
            || self.turns_started[permanent.controller.index()] > permanent.entered_controller_turn
    }

    /// Whether every "can't attack unless ..." clause this creature prints is
    /// currently satisfied. The query carries its own controller relation, so
    /// "defending player" is read as the attacker's opponent -- which is the
    /// only defending player there is in a two-player game.
    fn attack_restrictions_met(&self, permanent: &Permanent) -> bool {
        let mut allowed = true;
        let _ = self.visit_effective_abilities(permanent, |effective| {
            if effective.ability.is_executable()
                && matches!(
                    effective.ability.definition,
                    DeclarativeAbilityDef::Static(_)
                )
                && let Some(effect) = effective.ability.declarative_effect()
                && match effect {
                    EffectDef::CannotAttackUnless(query) => !self.any_battlefield_object_matches(
                        query,
                        permanent.card.id,
                        permanent.controller,
                    ),
                    EffectDef::CannotAttackIf(query) => self.any_battlefield_object_matches(
                        query,
                        permanent.card.id,
                        permanent.controller,
                    ),
                    _ => false,
                }
            {
                allowed = false;
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(())
        });
        allowed
    }

    pub(super) fn declare_attacker(&mut self, attacker: GameObjectId, defender: AttackDefender) {
        let vigilance = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == attacker)
            .is_some_and(|permanent| {
                self.permanent_has_executable_keyword(permanent, KeywordAbility::Vigilance)
            });
        if let Some(permanent) = self
            .battlefield
            .iter_mut()
            .find(|permanent| permanent.card.id == attacker)
        {
            permanent.attacking = true;
            permanent.attack_defender = Some(defender);
            permanent.attacked_this_turn = true;
            permanent.attacks_this_turn = permanent.attacks_this_turn.saturating_add(1);
            if !vigilance {
                // Tapping is part of the single CR 508.1 declaration. Commit
                // the state now for later attacker legality, but defer its
                // trigger event until every attacker has been declared.
                permanent.tapped = true;
            }
        }
    }

    pub(super) fn finish_declaring_attackers(&mut self) {
        self.attackers_declared = true;
        self.priority = self.active_player;
        self.consecutive_passes = 0;
        let attackers = self
            .battlefield
            .iter()
            .filter(|permanent| permanent.controller == self.active_player && permanent.attacking)
            .map(|permanent| permanent.card.id)
            .collect::<Vec<_>>();
        if attackers.is_empty() {
            return;
        }
        self.events.push(GameEvent::AttackDeclared {
            player: self.active_player,
            attackers: attackers.clone(),
        });
        // CR 508.2: the whole declaration happens at once, so every attacker
        // is already attacking by the time any of these triggers is captured.
        // Declaration size and attack number are facts of this event, not
        // mutable conditions to recheck while placing the trigger.
        let declaration_size = u8::try_from(attackers.len()).unwrap_or(u8::MAX);
        let listeners = self.battlefield_trigger_listeners();
        let mut events = attackers
            .iter()
            .filter_map(|attacker| {
                self.battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == *attacker && permanent.tapped)
                    .map(|permanent| CommittedTriggerEvent::Tapped {
                        object: self.trigger_event_object(permanent),
                        for_mana: false,
                    })
            })
            .collect::<Vec<_>>();
        events.extend(attackers.iter().filter_map(|attacker| {
            self.battlefield
                .iter()
                .find(|permanent| permanent.card.id == *attacker)
                .map(|permanent| CommittedTriggerEvent::Attacks {
                    object: self.trigger_event_object(permanent),
                    declaration_size,
                    attack_number: permanent.attacks_this_turn,
                })
        }));
        self.capture_battlefield_trigger_batch_from_snapshot(&listeners, &events);
    }

    /// Whether a static effect on `blocker` narrows what it may block, and
    /// this attacker is outside that. The other direction -- an attacker
    /// forbidding a blocker -- is `blocking_is_prevented`.
    pub(super) fn blocker_may_only_block(&self, blocker: &Permanent, attacker: &Permanent) -> bool {
        let characteristics = self.trigger_event_object(attacker);
        let mut restricted = false;
        let _ = self.visit_applied_rules(blocker, |applied| {
            if let AppliedRuleDef::CanBlockOnly(predicate) = applied.rule
                && !self.trigger_object_matches(predicate, &characteristics, applied.source, false)
            {
                restricted = true;
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(())
        });
        restricted
    }

    /// Whether a static effect on `attacker` forbids `blocker` from blocking
    /// it, as Juggernaut forbids Walls.
    pub(super) fn blocking_is_prevented(&self, attacker: &Permanent, blocker: &Permanent) -> bool {
        let characteristics = self.trigger_event_object(blocker);
        let mut prevented = false;
        let result = self.visit_applied_rules(attacker, |applied| {
            if let AppliedRuleDef::CannotBeBlockedBy(predicate) = applied.rule
                && self.trigger_object_matches(predicate, &characteristics, applied.source, false)
            {
                prevented = true;
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(())
        });
        debug_assert!(result.is_continue() || prevented);
        prevented
    }

    /// Whether a static or resolved rule on `attacker` requires `blocker` to
    /// block it, as Lure requires every creature that can. Read from the
    /// attacker for the same reason the prohibition above is: the printed
    /// text sits on the creature being blocked, not on the ones doing it.
    fn must_be_blocked_by(&self, attacker: &Permanent, blocker: &Permanent) -> bool {
        let characteristics = self.trigger_event_object(blocker);
        let mut required = false;
        let _ = self.visit_applied_rules(attacker, |applied| {
            if let AppliedRuleDef::MustBeBlockedBy(predicate) = applied.rule
                && self.trigger_object_matches(predicate, &characteristics, applied.source, false)
            {
                required = true;
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(())
        });
        required
    }

    /// Whether a continuous effect currently stops anything blocking this
    /// permanent. Asked afresh, so a resolved rule stops when its duration
    /// expires and a static one when its source leaves.
    fn cannot_be_blocked(&self, permanent: &Permanent) -> bool {
        self.has_applied_rule(permanent, AppliedRuleDef::CannotBeBlocked)
    }

    /// Whether a continuous effect from anywhere forbids this creature from
    /// attacking. The printed "can't attack unless ..." clause is a separate
    /// question a creature asks about itself.
    pub(super) fn cannot_attack(&self, permanent: &Permanent) -> bool {
        self.has_applied_rule(permanent, AppliedRuleDef::CannotAttack)
    }

    pub(super) fn cannot_block(&self, permanent: &Permanent) -> bool {
        if permanent.detained_until_turn_of.is_some() {
            return true;
        }
        // Unleash: the counter is what stops it blocking, so a creature that
        // declined the counter blocks as normal and one that took it never
        // does again.
        if permanent.counters(CounterKind::PlusOnePlusOne) > 0
            && self.permanent_has_executable_keyword(permanent, KeywordAbility::Unleash)
        {
            return true;
        }
        self.has_applied_rule(permanent, AppliedRuleDef::CannotBlock)
    }

    /// The blocks this player may declare, after combat requirements have
    /// taken the alternatives away.
    ///
    /// CR 509.1c asks for the maximum possible number of requirements to be
    /// obeyed without violating a restriction. A creature that is able to
    /// block a must-be-blocked attacker therefore has no other legal
    /// assignment: every block it makes elsewhere obeys one requirement
    /// fewer. Two such attackers leave it a choice between them, because it
    /// can only block one either way.
    pub(super) fn blocker_actions(&self, player: PlayerId) -> Vec<Action> {
        let available = self.available_blocker_actions(player);
        let required: Vec<GameObjectId> = available
            .iter()
            .filter_map(|action| self.required_block(action).map(|(blocker, _)| blocker))
            .collect();
        if required.is_empty() {
            return available;
        }
        available
            .into_iter()
            .filter(|action| match action {
                Action::DeclareBlocker { blocker, .. } => {
                    !required.contains(blocker) || self.required_block(action).is_some()
                }
                _ => true,
            })
            .collect()
    }

    /// The blocker and attacker of a declaration a requirement compels.
    fn required_block(&self, action: &Action) -> Option<(GameObjectId, GameObjectId)> {
        let Action::DeclareBlocker { blocker, attacker } = action else {
            return None;
        };
        let find = |id: GameObjectId| {
            self.battlefield
                .iter()
                .find(|permanent| permanent.card.id == id)
        };
        let (blocker_permanent, attacker_permanent) = (find(*blocker)?, find(*attacker)?);
        self.must_be_blocked_by(attacker_permanent, blocker_permanent)
            .then_some((*blocker, *attacker))
    }

    /// Whether a requirement is still unmet, which is what stops the
    /// defending player from finishing the declaration.
    pub(super) fn block_requirement_outstanding(&self, available: &[Action]) -> bool {
        available
            .iter()
            .any(|action| self.required_block(action).is_some())
    }

    fn available_blocker_actions(&self, player: PlayerId) -> Vec<Action> {
        let blockers: Vec<_> = self
            .battlefield
            .iter()
            .filter(|permanent| {
                permanent.controller == player
                    && !permanent.tapped
                    && permanent.blocking.is_none()
                    && self.power(permanent).is_some()
                    && !self.cannot_block(permanent)
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
                    self.landwalk_beats(permanent, permanent.controller.opponent()),
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
                let blocker_can_block_flying = self.has_flying(blocker_permanent)
                    || self
                        .permanent_has_executable_keyword(blocker_permanent, KeywordAbility::Reach);
                let ironclaw =
                    self.effective_behavior(blocker_permanent) == Some(CardBehavior::IronclawOrcs);
                attackers
                    .iter()
                    .filter_map(move |(attacker, flying, unblockable, power)| {
                        let attacker_permanent = self
                            .battlefield
                            .iter()
                            .find(|permanent| permanent.card.id == *attacker)
                            .expect("attacker is on the battlefield");
                        let intimidate = self.permanent_has_executable_keyword(
                            attacker_permanent,
                            KeywordAbility::Intimidate,
                        );
                        let shares_color = self
                            .permanent_colors(attacker_permanent)
                            .into_iter()
                            .zip(self.permanent_colors(blocker_permanent))
                            .any(|(attacker, blocker)| attacker && blocker);
                        let can_block = !(*unblockable
                            || self.cannot_be_blocked(attacker_permanent)
                            || self.blocking_is_prevented(attacker_permanent, blocker_permanent)
                            || self.blocker_may_only_block(blocker_permanent, attacker_permanent)
                            || *flying && !blocker_can_block_flying
                            || intimidate
                                && !self.is_artifact_permanent(blocker_permanent)
                                && !shares_color
                            || ironclaw && *power >= 2
                            || self.combat_is_protected(blocker_permanent, attacker_permanent));
                        can_block.then_some(Action::DeclareBlocker {
                            blocker,
                            attacker: *attacker,
                        })
                    })
            })
            .collect()
    }

    pub(super) fn declare_blocker(&mut self, blocker: GameObjectId, attacker: GameObjectId) {
        if let Some(permanent) = self
            .battlefield
            .iter_mut()
            .find(|permanent| permanent.card.id == blocker)
        {
            permanent.blocking = Some(attacker);
        }
        if !self.combat_blocked_attackers.contains(&attacker) {
            self.combat_blocked_attackers.push(attacker);
        }
    }

    pub(super) fn finish_declaring_blockers(&mut self) {
        self.blockers_declared = true;
        self.priority = self.active_player;
        self.consecutive_passes = 0;
        let blocked = self
            .battlefield
            .iter()
            .filter_map(|permanent| permanent.blocking)
            .collect::<Vec<_>>();
        for permanent in &mut self.battlefield {
            permanent.blocked = blocked.contains(&permanent.card.id);
        }
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
        // Blocking is one declaration. Freeze its listeners and every
        // object-local event before a triggered-mana ability can mutate the
        // battlefield while the declaration is being published.
        let listeners = self.battlefield_trigger_listeners();
        let mut trigger_events = self.becomes_blocked_trigger_events(&blocked);
        trigger_events.extend(self.blocking_relationship_trigger_events());
        trigger_events.extend(self.unblocked_attacker_trigger_events(&blocked));
        self.capture_battlefield_trigger_batch_from_snapshot(&listeners, &trigger_events);
    }

    /// CR 509.1h leaves an attacker nobody blocked unblocked, which is what
    /// these clauses read. It can only be answered once blocking is done.
    fn unblocked_attacker_trigger_events(
        &self,
        blocked: &[GameObjectId],
    ) -> Vec<CommittedTriggerEvent> {
        self.battlefield
            .iter()
            .filter(|permanent| permanent.attacking && !blocked.contains(&permanent.card.id))
            .map(|permanent| CommittedTriggerEvent::AttacksAndIsNotBlocked {
                object: self.trigger_event_object(permanent),
            })
            .collect()
    }

    /// One event per ordered pair of a blocker and what it blocks, so a
    /// clause printed on either creature reads the other as the triggering
    /// object. "Blocks or becomes blocked by" is one clause, not two.
    fn blocking_relationship_trigger_events(&self) -> Vec<CommittedTriggerEvent> {
        let pairs = self
            .battlefield
            .iter()
            .filter_map(|permanent| {
                permanent
                    .blocking
                    .map(|attacker| (permanent.card.id, attacker))
            })
            .collect::<Vec<_>>();
        let mut events = Vec::with_capacity(pairs.len().saturating_mul(2));
        for (blocker, attacker) in pairs {
            let Some((blocker, attacker)) = self
                .battlefield
                .iter()
                .find(|permanent| permanent.card.id == blocker)
                .map(|permanent| self.trigger_event_object(permanent))
                .zip(
                    self.battlefield
                        .iter()
                        .find(|permanent| permanent.card.id == attacker)
                        .map(|permanent| self.trigger_event_object(permanent)),
                )
            else {
                continue;
            };
            for (creature, other) in [(blocker.clone(), attacker.clone()), (attacker, blocker)] {
                events.push(CommittedTriggerEvent::BlocksOrBecomesBlocked { creature, other });
            }
        }
        events
    }

    /// CR 509.1h. Each attacker becomes blocked once, however many creatures
    /// blocked it, so the event fires per attacker and carries the count the
    /// rampage-style clauses are written against.
    fn becomes_blocked_trigger_events(
        &self,
        blocked: &[GameObjectId],
    ) -> Vec<CommittedTriggerEvent> {
        let mut attackers = blocked.to_vec();
        attackers.sort_unstable();
        attackers.dedup();
        attackers
            .into_iter()
            .filter_map(|attacker| {
                let blockers = blocked.iter().filter(|id| **id == attacker).count();
                self.battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == attacker)
                    .map(|permanent| CommittedTriggerEvent::BecomesBlocked {
                        object: self.trigger_event_object(permanent),
                        blockers_beyond_first: u16::try_from(blockers.saturating_sub(1))
                            .unwrap_or(u16::MAX),
                    })
            })
            .collect()
    }

    pub(super) fn start_combat_damage(&mut self) {
        // Tests and a few internal procedures can construct combat directly,
        // so also capture live blocking relationships here. During an ordinary
        // game, `declare_blocker` recorded them before either player received
        // priority and they therefore survive a blocker leaving the field.
        let newly_blocked = self
            .battlefield
            .iter()
            .filter_map(|permanent| permanent.blocking)
            .collect::<Vec<_>>();
        for attacker in newly_blocked {
            if !self.combat_blocked_attackers.contains(&attacker) {
                self.combat_blocked_attackers.push(attacker);
            }
        }

        let strike_wave_combatants = self
            .battlefield
            .iter()
            .filter(|permanent| permanent.attacking || permanent.blocking.is_some())
            .filter(|permanent| {
                self.permanent_has_executable_keyword(permanent, KeywordAbility::FirstStrike)
                    || self
                        .permanent_has_executable_keyword(permanent, KeywordAbility::DoubleStrike)
            })
            .map(|permanent| permanent.card.id)
            .collect::<Vec<_>>();
        self.combat_damage_stage = if strike_wave_combatants.is_empty() {
            CombatDamageStage::Single
        } else {
            CombatDamageStage::FirstStrike {
                strike_wave_combatants,
            }
        };
        self.begin_combat_damage_assignment();
    }

    pub(super) fn begin_regular_combat_damage_after_first_strike(&mut self) {
        let CombatDamageStage::FirstStrike {
            strike_wave_combatants,
        } = &self.combat_damage_stage
        else {
            return;
        };
        self.combat_damage_stage = CombatDamageStage::RegularAfterFirstStrike {
            strike_wave_combatants: strike_wave_combatants.clone(),
        };
        self.begin_combat_damage_assignment();
    }

    pub(super) fn deals_damage_in_current_combat_step(&self, permanent: &Permanent) -> bool {
        match &self.combat_damage_stage {
            CombatDamageStage::NotStarted | CombatDamageStage::Single => true,
            CombatDamageStage::FirstStrike {
                strike_wave_combatants,
            } => strike_wave_combatants.contains(&permanent.card.id),
            CombatDamageStage::RegularAfterFirstStrike {
                strike_wave_combatants,
            } => {
                !strike_wave_combatants.contains(&permanent.card.id)
                    || self
                        .permanent_has_executable_keyword(permanent, KeywordAbility::DoubleStrike)
            }
        }
    }

    pub(super) fn begin_combat_damage_assignment(&mut self) {
        for permanent in &mut self.battlefield {
            permanent.combat_damage_assignment.clear();
        }
        self.pending_combat_attackers = self
            .battlefield
            .iter()
            .filter(|attacker| {
                attacker.attacking && self.deals_damage_in_current_combat_step(attacker)
            })
            // Ask exactly when there is a real choice. One blocker and no
            // trample leaves a single legal distribution and no question; one
            // blocker plus trample is a genuine decision about how much to
            // spill past it.
            .filter(|attacker| self.combat_assignment_actions(attacker.card.id).len() > 1)
            .map(|attacker| attacker.card.id)
            .collect();
        if self.pending_combat_attackers.is_empty() {
            self.deal_combat_damage();
        }
    }

    /// Who assigns this attacker's combat damage. CR 702.22: banding moves
    /// that choice to the defending player, so a creature with banding
    /// blocking an attacker takes the decision away from the attacker's
    /// controller. Any one banding blocker is enough.
    pub(super) fn combat_damage_assigner(&self, attacker: GameObjectId) -> PlayerId {
        self.battlefield
            .iter()
            .find(|permanent| {
                permanent.blocking == Some(attacker)
                    && self.permanent_has_executable_keyword(permanent, KeywordAbility::Banding)
            })
            .map_or(self.active_player, |blocker| blocker.controller)
    }

    pub(super) fn combat_assignment_actions(&self, attacker_id: GameObjectId) -> Vec<Action> {
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
        let blocker_count = recipients.len();
        let defender_index = trample
            .then(|| self.combat_defender_target(attacker))
            .flatten()
            .map(|defender| {
                let index = recipients.len();
                recipients.push(defender);
                index
            });

        damage_distributions(recipients.len(), power)
            .into_iter()
            .filter(|amounts| {
                let blockers = || {
                    recipients
                        .iter()
                        .take(blocker_count)
                        .zip(amounts)
                        .filter_map(|(target, amount)| match target {
                            Target::Permanent(id) => Some((*id, *amount)),
                            Target::Player(_) | Target::Card(_) | Target::Spell(_) => None,
                        })
                };
                // CR 702.19b: trample only spills once every blocker has
                // lethal damage assigned. Without defender damage, current
                // CR 510.1c permits any division among the blockers.
                let defender_damage = defender_index
                    .and_then(|index| amounts.get(index))
                    .copied()
                    .unwrap_or(0);
                if defender_damage == 0 {
                    return true;
                }
                blockers().all(|(id, amount)| amount >= self.lethal_damage_from(id, attacker_id))
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

    /// How an unassigned attacker spreads its damage: enough to kill each
    /// blocker in turn, then the remainder over the top when it can trample
    /// onto its defender, or onto the last blocker otherwise.
    pub(super) fn default_damage_split(
        &self,
        attacker_id: GameObjectId,
        blockers: &[GameObjectId],
    ) -> Vec<(Target, u16)> {
        let Some(attacker) = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == attacker_id)
        else {
            return Vec::new();
        };
        let mut remaining = self.power(attacker).unwrap_or(0).max(0).cast_unsigned();
        let trample = self.has_trample(attacker);
        let mut split = Vec::with_capacity(blockers.len() + 1);
        for blocker in blockers {
            let amount = self
                .lethal_damage_from(*blocker, attacker_id)
                .min(remaining);
            remaining -= amount;
            split.push((Target::Permanent(*blocker), amount));
        }
        if remaining > 0 {
            if trample && let Some(defender) = self.combat_defender_target(attacker) {
                split.push((defender, remaining));
            } else if let Some(last) = split.last_mut() {
                last.1 += remaining;
            }
        }
        split
    }

    pub(super) fn lethal_damage(&self, permanent_id: GameObjectId) -> u16 {
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

    pub(super) fn lethal_damage_from(
        &self,
        permanent_id: GameObjectId,
        source: GameObjectId,
    ) -> u16 {
        let ordinary = self.lethal_damage(permanent_id);
        if ordinary > 0
            && self
                .source_controller_with_keyword(source, KeywordAbility::Deathtouch)
                .is_some()
        {
            1
        } else {
            ordinary
        }
    }

    pub(super) fn assign_combat_damage(
        &mut self,
        attacker: GameObjectId,
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

    pub(super) fn deal_combat_damage(&mut self) {
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
            let attacker_deals_damage =
                self.deals_damage_in_current_combat_step(&self.battlefield[attacker_index]);
            let blockers: Vec<_> = self
                .battlefield
                .iter()
                .filter(|permanent| permanent.blocking == Some(attacker_id))
                .map(|permanent| permanent.card.id)
                .collect();
            if attacker_deals_damage && blockers.is_empty() {
                let was_blocked = self.combat_blocked_attackers.contains(&attacker_id);
                if was_blocked && !self.has_trample(&self.battlefield[attacker_index]) {
                    continue;
                }
                let Some(defender) = self.combat_defender_target(&self.battlefield[attacker_index])
                else {
                    continue;
                };
                self.deal_combat_damage_to(attacker_id, defender, power);
            } else if !blockers.is_empty() {
                self.exchange_blocked_combat_damage(
                    attacker_id,
                    attacker_index,
                    &blockers,
                    attacker_deals_damage,
                );
            }
        }
        self.check_state_based_actions();
    }
}

pub(super) fn damage_distributions(recipient_count: usize, total: u16) -> Vec<Vec<u16>> {
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
