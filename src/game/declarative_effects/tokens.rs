//! Making tokens: the printed clauses that put a permanent onto the
//! battlefield out of nothing.
//!
//! Split out of the parent module for the source-size budget. What belongs
//! here is the three clauses that mint one: a token the card names, a token
//! the resolving permanent then wears, and a copy of something already on
//! the battlefield.

use super::super::{EffectResolutionContext, Game, ScopedEffect, StackObject, Target};
use crate::card::EffectDef;

impl Game {
    pub(super) fn resolve_token_effect(
        &mut self,
        scoped: ScopedEffect,
        object: &StackObject,
        context: &EffectResolutionContext,
    ) {
        match scoped.effect {
            EffectDef::CreateToken {
                token,
                count,
                tapped,
                attacking,
                counters,
            } => {
                // Two players, so the one opponent is the only thing an
                // arriving attacker could be attacking (CR 506.3d).
                let defender =
                    attacking.then(|| crate::AttackDefender::Player(object.controller.opponent()));
                // Worked out once, before any token is made: the number is
                // what the effect found, not what the board looks like part
                // way through creating them.
                let counters = counters.map(|counters| {
                    (
                        counters.kind,
                        u16::try_from(
                            self.effect_value(counters.amount, object, context, scoped)
                                .max(0),
                        )
                        .unwrap_or(u16::MAX),
                    )
                });
                for _ in 0..self.effect_value(count, object, context, scoped).max(0) {
                    self.create_token_arriving(
                        object.controller,
                        token,
                        None,
                        tapped,
                        defender,
                        counters,
                    );
                }
            }
            EffectDef::CreateAttachedToken { token } => {
                if let Some(source) = object.source {
                    self.create_attached_token(object.controller, token, source);
                }
            }
            EffectDef::CreateTokenCopyOf { object: recipient } => {
                let copies = self
                    .effect_recipients(recipient, object, context, scoped)
                    .into_iter()
                    .filter_map(|target| match target {
                        Target::Permanent(id) => self
                            .battlefield
                            .iter()
                            .find(|permanent| permanent.card.id == id)
                            // A token that is itself a copy of something else
                            // copies what it became, not what it was made as.
                            .map(|permanent| {
                                permanent
                                    .copied_from
                                    .map_or(permanent.card.definition, |(definition, _)| definition)
                            }),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                for definition in copies {
                    self.create_token(object.controller, definition);
                }
            }
            _ => unreachable!("the caller admits only token-making clauses"),
        }
    }
}
