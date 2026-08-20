//! Effects that only set a field on a permanent.
//!
//! Split out of the parent module for the source-size budget. Each of these
//! finds the permanents a recipient names and records something on them.

#![allow(clippy::wildcard_imports)]

use super::*;

impl Game {
    pub(super) fn resolve_permanent_state_effect(
        &mut self,
        scoped: ScopedEffect,
        object: &StackObject,
        context: &EffectResolutionContext,
    ) {
        match scoped.effect {
            EffectDef::AddCounters { .. }
            | EffectDef::DoubleCounters { .. }
            | EffectDef::RemoveCounters { .. } => {
                self.resolve_counter_effect(scoped, object, context);
            }
            EffectDef::SubstituteBasicLandTypeUntilEndOfTurn { chooser } => {
                self.queue_basic_land_type_substitution(object, context, scoped, chooser);
            }
            EffectDef::PhaseOut { object: recipient } => {
                self.phase_out_recipients(recipient, object, context, scoped);
            }
            EffectDef::DestroyAtEndOfCombat { object: recipient } => {
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    let Target::Permanent(id) = target else {
                        continue;
                    };
                    if let Some(permanent) = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == id)
                    {
                        permanent.destroy_at_end_of_combat = true;
                    }
                }
            }
            EffectDef::RemoveAllCounters {
                object: recipient,
                kind,
            } => {
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    let Target::Permanent(id) = target else {
                        continue;
                    };
                    if let Some(permanent) = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == id)
                    {
                        let held = permanent.counters(kind);
                        permanent.remove_counters(kind, held);
                    }
                }
            }
            EffectDef::SkipNextUntapSteps {
                object: recipient,
                count,
            } => {
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    let Target::Permanent(id) = target else {
                        continue;
                    };
                    if let Some(permanent) = self
                        .battlefield
                        .iter_mut()
                        .find(|permanent| permanent.card.id == id)
                    {
                        // Two of these stack rather than overwrite: a creature
                        // told twice to sit out sits out twice.
                        permanent.skipped_untap_steps =
                            permanent.skipped_untap_steps.saturating_add(count);
                    }
                }
            }
            _ => unreachable!("only permanent-state effects are dispatched here"),
        }
    }

    /// Counters put on or taken off the permanents a recipient names.
    fn resolve_counter_effect(
        &mut self,
        scoped: ScopedEffect,
        object: &StackObject,
        context: &EffectResolutionContext,
    ) {
        match scoped.effect {
            EffectDef::AddCounters {
                object: recipient,
                kind,
                amount,
            } => {
                let amount = self
                    .effect_value(amount, object, context, scoped)
                    .max(0)
                    .try_into()
                    .unwrap_or(u16::MAX);
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Permanent(permanent) = target
                        && let Some(permanent) = self
                            .battlefield
                            .iter_mut()
                            .find(|candidate| candidate.card.id == permanent)
                    {
                        permanent.add_counters(kind, amount);
                    }
                }
            }
            EffectDef::DoubleCounters {
                object: recipient,
                kind,
            } => {
                // Each permanent's own count, read as that permanent is
                // reached: doubling is not one amount handed to everybody.
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Permanent(permanent) = target
                        && let Some(permanent) = self
                            .battlefield
                            .iter_mut()
                            .find(|candidate| candidate.card.id == permanent)
                    {
                        let existing = permanent.counters(kind);
                        permanent.add_counters(kind, existing);
                    }
                }
            }
            EffectDef::RemoveCounters {
                object: recipient,
                kind,
                amount,
            } => {
                let amount = self
                    .effect_value(amount, object, context, scoped)
                    .max(0)
                    .try_into()
                    .unwrap_or(u16::MAX);
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Permanent(permanent) = target
                        && let Some(permanent) = self
                            .battlefield
                            .iter_mut()
                            .find(|candidate| candidate.card.id == permanent)
                    {
                        permanent.remove_counters(kind, amount);
                    }
                }
            }
            _ => unreachable!("only counter effects reach the counter resolver"),
        }
    }
}
