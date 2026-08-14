//! Tapping, untapping, and holding a permanent down.
//!
//! These three travel together: each reaches the same battlefield permanents
//! and the third is the one that lasts, so keeping them in one place makes
//! the difference between them easy to see.

use super::super::{EffectDef, EffectResolutionContext, Game, ScopedEffect, StackObject, Target};

impl Game {
    pub(super) fn resolve_tap_effect(
        &mut self,
        scoped: ScopedEffect,
        object: &StackObject,
        context: &EffectResolutionContext,
    ) {
        match scoped.effect {
            EffectDef::Tap { object: recipient } => {
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Permanent(permanent) = target {
                        let _ = self.tap_permanent(permanent);
                    }
                }
            }
            EffectDef::Untap { object: recipient } => {
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Permanent(id) = target
                        && let Some(permanent) = self
                            .battlefield
                            .iter_mut()
                            .find(|candidate| candidate.card.id == id)
                    {
                        permanent.tapped = false;
                    }
                }
            }
            EffectDef::DoesNotUntapWhileSourceTapped { object: recipient } => {
                // The source is recorded rather than a deadline: it decides
                // when the hold ends by untapping.
                let holder = object.source.unwrap_or(object.id);
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Permanent(permanent) = target
                        && let Some(permanent) = self
                            .battlefield
                            .iter_mut()
                            .find(|candidate| candidate.card.id == permanent)
                        && !permanent.held_tapped_by.contains(&holder)
                    {
                        permanent.held_tapped_by.push(holder);
                    }
                }
            }
            _ => unreachable!("resolve_tap_effect called for another effect"),
        }
    }
}
