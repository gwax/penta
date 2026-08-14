//! Tapping and untapping permanents.

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
            _ => unreachable!("resolve_tap_effect called for another effect"),
        }
    }
}
