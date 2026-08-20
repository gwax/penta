//! Drawing, discarding, milling, and searching: the effects that move cards
//! through a player's hand and library.

use super::super::{
    DiscardSelectionDef, DrawReplacement, EffectDef, EffectResolutionContext, Game, GameEvent,
    ScopedEffect, StackObject, Target, ZoneMoveCause, public_cards,
};

impl Game {
    #[allow(clippy::too_many_lines)]
    pub(super) fn resolve_hand_and_library_effect(
        &mut self,
        scoped: ScopedEffect,
        object: &StackObject,
        context: &EffectResolutionContext,
    ) {
        match scoped.effect {
            EffectDef::DrawCards { recipient, amount } => {
                let amount = self
                    .effect_value(amount, object, context, scoped)
                    .max(0)
                    .try_into()
                    .unwrap_or(u16::MAX);
                let mut players = self
                    .effect_recipients(recipient, object, context, scoped)
                    .into_iter()
                    .filter_map(|target| match target {
                        Target::Player(player) => Some(player),
                        Target::Card(_) | Target::Permanent(_) | Target::Spell(_) => None,
                    })
                    .collect::<Vec<_>>();
                // CR 121.2c: when multiple players draw, the active player
                // performs every individual draw first, followed by the
                // nonactive player. This order belongs to drawing rather than
                // to the general `EachPlayer` recipient.
                players.sort_by_key(|player| (*player != self.active_player, player.index()));
                for player in players {
                    self.draw_cards(player, amount);
                }
            }
            EffectDef::ShuffleLibrary { player: recipient } => {
                let mut players = self
                    .effect_recipients(recipient, object, context, scoped)
                    .into_iter()
                    .filter_map(|target| match target {
                        Target::Player(player) => Some(player),
                        Target::Card(_) | Target::Permanent(_) | Target::Spell(_) => None,
                    })
                    .collect::<Vec<_>>();
                players.sort_by_key(|player| (*player != self.active_player, player.index()));
                for player in players {
                    self.rng.shuffle(&mut self.players[player.index()].library);
                }
            }
            EffectDef::Discard {
                recipient,
                amount,
                selection: DiscardSelectionDef::RecipientChooses,
                then,
            } => {
                let amount = self.effect_value(amount, object, context, scoped).max(0);
                let cause = ZoneMoveCause::Effect {
                    controller: object.controller,
                };
                let players = self
                    .effect_recipients(recipient, object, context, scoped)
                    .into_iter()
                    .filter_map(|target| match target {
                        Target::Player(player) => Some(player),
                        Target::Card(_) | Target::Permanent(_) | Target::Spell(_) => None,
                    })
                    // A player nobody can force to discard is simply not
                    // among the ones asked to.
                    .filter(|player| self.can_be_forced_to_discard(*player, object.controller))
                    .collect();
                let follow_up = then.map(|follow_up| crate::game::DiscardFollowUp {
                    counted: follow_up.counted,
                    effect: scoped.with_effect(*follow_up.effect),
                    object: Box::new(object.clone()),
                    context: context.clone(),
                });
                self.queue_effect_discards_then(players, amount, cause, follow_up);
            }
            EffectDef::Discard {
                recipient,
                amount,
                selection: DiscardSelectionDef::Random,
                then: None,
            } => {
                let amount = self
                    .effect_value(amount, object, context, scoped)
                    .max(0)
                    .try_into()
                    .unwrap_or(u16::MAX);
                let cause = ZoneMoveCause::Effect {
                    controller: object.controller,
                };
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Player(player) = target
                        && self.can_be_forced_to_discard(player, object.controller)
                    {
                        self.discard_random(player, amount, cause);
                    }
                }
            }
            EffectDef::Discard {
                recipient,
                amount,
                selection: DiscardSelectionDef::RandomMatching(predicate),
                then: None,
            } => {
                let amount = self
                    .effect_value(amount, object, context, scoped)
                    .max(0)
                    .try_into()
                    .unwrap_or(u16::MAX);
                let cause = ZoneMoveCause::Effect {
                    controller: object.controller,
                };
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Player(player) = target {
                        self.discard_random_matching(
                            player,
                            amount,
                            *predicate,
                            object.source.unwrap_or(object.id),
                            cause,
                        );
                    }
                }
            }
            EffectDef::DiscardCards { object: recipient } => {
                let recipients = self.effect_recipients(recipient, object, context, scoped);
                let cause = ZoneMoveCause::Effect {
                    controller: object.controller,
                };
                for player in [self.active_player, self.active_player.opponent()] {
                    let cards = recipients
                        .iter()
                        .filter_map(|target| match target {
                            Target::Card(card) => Some(*card),
                            Target::Player(_) | Target::Permanent(_) | Target::Spell(_) => None,
                        })
                        .filter(|card| {
                            self.players[player.index()]
                                .hand
                                .iter()
                                .any(|candidate| candidate.id == *card)
                        })
                        .collect::<Vec<_>>();
                    self.discard_cards_with_cause(player, &cards, cause);
                }
            }
            EffectDef::MillUntil {
                player: recipient,
                object: predicate,
                matched_zone,
            } => {
                let source = object.source.unwrap_or(object.id);
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Player(player) = target {
                        self.mill_until_matching(player, predicate, matched_zone, source);
                    }
                }
            }
            EffectDef::Mill {
                player: recipient,
                amount,
                binding,
                then,
            } => {
                let count = self.effect_value(amount, object, context, scoped).max(0);
                let Ok(count) = usize::try_from(count) else {
                    return;
                };
                let mut buried = Vec::new();
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Player(player) = target {
                        let milled = self.take_top_of_library(player, count);
                        // Bound by the identity the cards have in the
                        // graveyard: burying them mints new objects, and
                        // "from among them" means the ones lying there now.
                        for card in milled {
                            let (card, _zone_change) = self.zone_change_card(card);
                            buried.push(Target::Card(card.id));
                            self.put_card_into_graveyard(player, card);
                        }
                    }
                }
                let Some(then) = then else {
                    return;
                };
                // A mill never stops to ask, so the follow-up runs here
                // rather than out of a continuation.
                let mut context = context.clone();
                if let Some(binding) = binding {
                    context.bind_object_group(binding, buried);
                }
                self.resolve_effect_def(scoped.with_effect(*then), object, context);
            }
            EffectDef::ExileTopOfLibraryToPlay {
                player: recipient,
                amount,
            } => {
                let count = self.effect_value(amount, object, context, scoped).max(0);
                let Ok(count) = usize::try_from(count) else {
                    return;
                };
                let controller = object.controller;
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Player(player) = target {
                        for card in self.take_top_of_library(player, count) {
                            let (card, _zone_change) = self.zone_change_card(card);
                            let exiled = card.id;
                            self.players[player.index()].exile.push(card);
                            self.permit_free_play_this_turn(exiled, controller);
                        }
                    }
                }
            }
            EffectDef::LookAtHand { player: recipient } => {
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Player(seen) = target {
                        self.last_seen_hands[object.controller.index()] =
                            Some((seen, public_cards(&self.players[seen.index()].hand)));
                    }
                }
            }
            EffectDef::RevealHand { player: recipient } => {
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Player(revealer) = target {
                        let hand = &self.players[revealer.index()].hand;
                        let events = hand
                            .iter()
                            .map(|card| GameEvent::CardRevealed {
                                player: revealer,
                                card: card.id,
                                definition: card.definition,
                            })
                            .collect::<Vec<_>>();
                        let seen = public_cards(hand);
                        self.events.extend(events);
                        // Everyone saw it, so everyone remembers it.
                        for viewer in &mut self.last_seen_hands {
                            *viewer = Some((revealer, seen.clone()));
                        }
                    }
                }
            }
            EffectDef::RevealAtRandomFromHand {
                player: recipient,
                binding,
                then,
            } => {
                let mut context = context.clone();
                for target in self.effect_recipients(recipient, object, &context, scoped) {
                    if let Target::Player(revealer) = target {
                        // Drawn through the game's seeded RNG so a replay
                        // reveals the same card, and read before anything
                        // moves so the reveal is of the hand as it stands.
                        let hand = &self.players[revealer.index()].hand;
                        let revealed = (!hand.is_empty()).then(|| {
                            let index = self.rng.index_below(hand.len());
                            let card = &self.players[revealer.index()].hand[index];
                            (card.id, card.definition)
                        });
                        if let Some((card, definition)) = revealed {
                            self.events.push(GameEvent::CardRevealed {
                                player: revealer,
                                card,
                                definition,
                            });
                            context.bind_single_object(binding, Some(Target::Card(card)));
                        }
                    }
                }
                self.resolve_effect_def(scoped.with_effect(*then), object, context);
            }
            EffectDef::LookAtTopAndSelect {
                player: recipient,
                looker,
                selection,
            } => {
                // The looker is resolved first and once: a spy that has left
                // the table still finishes looking, but nobody else does it
                // for them.
                let Some(Target::Player(looker)) = self
                    .effect_recipients(looker, object, context, scoped)
                    .into_iter()
                    .next()
                else {
                    return;
                };
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Player(player) = target {
                        self.queue_top_card_selection(
                            player,
                            looker,
                            selection,
                            object,
                            context.clone(),
                            scoped,
                        );
                    }
                }
            }
            EffectDef::SearchZone {
                player: recipient,
                source: source_zone,
                object: predicate,
                minimum,
                maximum,
                reveal,
                destination,
                placement,
                shuffle,
                enters_tapped,
                binding,
                then,
            } => {
                let source = object.source.unwrap_or(object.id);
                // Sized once, before the search is offered: "up to X, where X
                // is the number of lands you control" is answered by the
                // board as the spell resolves.
                let maximum =
                    usize::try_from(self.effect_value(maximum, object, context, scoped).max(0))
                        .unwrap_or(usize::MAX);
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Player(player) = target {
                        self.queue_zone_search(
                            player,
                            source_zone,
                            predicate,
                            minimum,
                            maximum,
                            reveal,
                            destination,
                            placement,
                            shuffle,
                            binding,
                            then.map(|effect| {
                                (object.clone(), context.clone(), scoped.with_effect(*effect))
                            }),
                            enters_tapped,
                            source,
                            object.controller,
                        );
                    }
                }
            }
            EffectDef::ChooseCards {
                player: recipient,
                sources,
                object: predicate,
                minimum,
                maximum,
                reveal,
                destination,
                placement,
                arrival_effect,
            } => {
                let source = object.source.unwrap_or(object.id);
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Player(player) = target {
                        self.queue_owned_card_choice(
                            player,
                            sources,
                            predicate,
                            minimum,
                            maximum,
                            reveal,
                            destination,
                            placement,
                            // Only a battlefield arrival can carry anything,
                            // and only the clauses that print one do.
                            arrival_effect.map(|_| (object.clone(), context.clone(), scoped)),
                            source,
                            object.controller,
                        );
                    }
                }
            }
            EffectDef::ReplaceNextDrawThisTurn {
                player: recipient,
                effect,
            } => {
                for target in self.effect_recipients(recipient, object, context, scoped) {
                    if let Target::Player(player) = target {
                        self.draw_replacements[player.index()].push_back(DrawReplacement {
                            object: Box::new(object.clone()),
                            context: context.clone(),
                            effect: scoped.with_effect(*effect),
                        });
                    }
                }
            }
            _ => unreachable!("resolve_hand_and_library_effect called for another effect"),
        }
    }
}
