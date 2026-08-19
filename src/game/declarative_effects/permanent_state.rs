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
}
