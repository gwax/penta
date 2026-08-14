//! Dealing and publishing combat damage.
//!
//! Split out of the parent module for the source-size budget.

#![allow(clippy::wildcard_imports)]

use super::*;

impl Game {
    /// How much life a drain can take from a recipient: what it had before
    /// the damage, which is all it can give however much is dealt.
    pub(in crate::game) fn drainable_from(&self, target: Target) -> u16 {
        match target {
            Target::Player(player) => self.players[player.index()].life.max(0).cast_unsigned(),
            Target::Permanent(id) => self
                .battlefield
                .iter()
                .find(|permanent| permanent.card.id == id)
                .and_then(|permanent| {
                    if self
                        .permanent_types(permanent)
                        .is_some_and(|types| types.contains(CardType::Planeswalker))
                    {
                        return Some(permanent.counters(CounterKind::Loyalty));
                    }
                    self.toughness(permanent)
                        .map(|value| value.max(0).cast_unsigned())
                })
                .unwrap_or(0),
            Target::Card(_) | Target::Spell(_) => 0,
        }
    }

    /// Combat damage from an attacker to whatever it is attacking. A player
    /// and a planeswalker both use the canonical damage event; its `combat`
    /// flag is what a trigger matcher narrows.
    pub(in crate::game) fn deal_combat_damage_to(
        &mut self,
        attacker: GameObjectId,
        defender: Target,
        amount: u16,
    ) {
        match defender {
            Target::Player(player) => self.deal_combat_damage_to_player(attacker, player, amount),
            Target::Permanent(_) | Target::Card(_) | Target::Spell(_) => {
                // Flagged as combat damage so a trigger that listens for it
                // arriving here, as Vraska's does, can tell it apart from an
                // ability's damage.
                self.damage_target_from_kind(Some(attacker), Some(defender), amount, true);
            }
        }
    }

    pub(in crate::game) fn deal_combat_damage_to_player(
        &mut self,
        attacker: GameObjectId,
        player: PlayerId,
        amount: u16,
    ) {
        self.damage_target_from_kind(Some(attacker), Some(Target::Player(player)), amount, true);
    }

    /// Combat damage between one blocked attacker and everything blocking it,
    /// in both directions.
    pub(in crate::game) fn exchange_blocked_combat_damage(
        &mut self,
        attacker_id: GameObjectId,
        attacker_index: usize,
        blockers: &[GameObjectId],
        attacker_deals_damage: bool,
    ) {
        let assignments = self.battlefield[attacker_index]
            .combat_damage_assignment
            .clone();
        if attacker_deals_damage {
            let split = if assignments.is_empty() {
                self.default_damage_split(attacker_id, blockers)
            } else {
                assignments
                    .into_iter()
                    .map(|assignment| (assignment.recipient, assignment.amount))
                    .collect()
            };
            for (recipient, amount) in split {
                // Trample past a blocker is still combat damage to a player,
                // so it goes through the same path as an unblocked hit.
                if let Target::Player(player) = recipient {
                    self.deal_combat_damage_to_player(attacker_id, player, amount);
                } else {
                    self.damage_target_from_kind(Some(attacker_id), Some(recipient), amount, true);
                }
            }
        }
        let return_damage = blockers
            .iter()
            .filter_map(|id| {
                self.battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == *id)
                    .filter(|permanent| self.deals_damage_in_current_combat_step(permanent))
                    .and_then(|permanent| self.power(permanent))
                    .map(|power| (*id, power.max(0).cast_unsigned()))
            })
            .collect::<Vec<_>>();
        for (blocker, amount) in return_damage {
            self.damage_target_from_kind(
                Some(blocker),
                Some(Target::Permanent(attacker_id)),
                amount,
                true,
            );
        }
    }

    pub(in crate::game) fn combat_defender(attacker: &Permanent) -> AttackDefender {
        attacker
            .attack_defender
            .unwrap_or(AttackDefender::Player(attacker.controller.opponent()))
    }

    pub(in crate::game) fn combat_defender_target(&self, attacker: &Permanent) -> Option<Target> {
        match Self::combat_defender(attacker) {
            AttackDefender::Player(player) => Some(Target::Player(player)),
            AttackDefender::Planeswalker(id) => self
                .battlefield
                .iter()
                .find(|permanent| {
                    permanent.card.id == id
                        && permanent.controller != attacker.controller
                        && self
                            .permanent_types(permanent)
                            .is_some_and(|types| types.contains(CardType::Planeswalker))
                })
                .map(|permanent| Target::Permanent(permanent.card.id)),
        }
    }
}
