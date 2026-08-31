//! Leovold, Emissary of Trest: their draw spells become one card, and their
//! removal spells become a replacement.

use super::*;

/// Leovold on the battlefield under Player One since last turn.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    game.players[0].library.clear();
    game.players[1].library.clear();
    for index in 0..8 {
        game.players[0]
            .library
            .push(card(110_000 + index, cards::ISLAND, PlayerId::One));
        game.players[1]
            .library
            .push(card(110_100 + index, cards::ISLAND, PlayerId::Two));
    }
    let leovold = game
        .put_onto_battlefield(PlayerId::One, cards::LEOVOLD_EMISSARY_OF_TREST)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, leovold)
}

fn settle(game: &mut Game, draw: bool) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let wanted = |take: bool| {
                decision
                    .options
                    .iter()
                    .find(|option| (option.label != "Decline") == take)
                    .map(|option| vec![option.id])
                    .unwrap_or_default()
            };
            let options = wanted(draw);
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the offered choice is legal");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(game);
}

/// Their first draw of the turn lands and the second does not.
#[test]
fn an_opponent_draws_only_one_card_a_turn() {
    let (mut game, _) = staged();
    let before = game.players[1].library.len();

    game.draw_cards(PlayerId::Two, 1);
    assert_eq!(game.players[1].hand.len(), 1, "the first one lands");

    game.draw_cards(PlayerId::Two, 3);

    assert_eq!(game.players[1].hand.len(), 1, "and no more do");
    assert_eq!(
        game.players[1].library.len(),
        before - 1,
        "the cards stay in the library rather than being lost",
    );
}

/// The bound is per turn: a new turn is a new card.
#[test]
fn the_bound_resets_with_the_turn() {
    let (mut game, _) = staged();
    game.draw_cards(PlayerId::Two, 2);
    assert_eq!(game.players[1].hand.len(), 1);

    game.cards_drawn_this_turn = [0; 2];

    game.draw_cards(PlayerId::Two, 2);
    assert_eq!(game.players[1].hand.len(), 2, "one more, and only one");
}

/// It binds them and not you.
#[test]
fn you_draw_as_many_as_you_like() {
    let (mut game, _) = staged();

    game.draw_cards(PlayerId::One, 3);

    assert_eq!(
        game.players[0].hand.len(),
        3,
        "your own draws are untouched"
    );
}

/// A spell of theirs pointed at you offers a card.
#[test]
fn their_spell_pointed_at_you_replaces_itself() {
    let (mut game, _) = staged();
    let bolt = game
        .build_zone(PlayerId::Two, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = bolt.id;
    game.players[1].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.priority = PlayerId::Two;
    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == held
                    && choices.targets().iter().any(|selection| {
                        selection.targets().contains(&Target::Player(PlayerId::One))
                    })
            }
            _ => false,
        })
        .expect("they can point it at you");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    settle(&mut game, true);

    assert_eq!(game.players[0].hand.len(), 1, "you drew a card for it");
    assert_eq!(game.players[0].life, 17, "and took the three all the same");
}

/// A spell of theirs pointed at a permanent you control does the same.
#[test]
fn their_spell_pointed_at_your_creature_replaces_itself() {
    let (mut game, leovold) = staged();
    let bolt = game
        .build_zone(PlayerId::Two, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = bolt.id;
    game.players[1].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.priority = PlayerId::Two;
    let cast =
        game.legal_actions(PlayerId::Two)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == held
                        && choices.targets().iter().any(|selection| {
                            selection.targets().contains(&Target::Permanent(leovold))
                        })
                }
                _ => false,
            })
            .expect("they can point it at Leovold");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    settle(&mut game, true);

    assert_eq!(game.players[0].hand.len(), 1, "the card arrives anyway");
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == leovold),
        "even though the Elf is dead",
    );
}

/// "A spell an opponent controls": your own spell pointed at your own
/// creature is not one.
#[test]
fn your_own_spell_offers_nothing() {
    let (mut game, leovold) = staged();
    let bolt = game
        .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = bolt.id;
    game.players[0].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    let cast =
        game.legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == held
                        && choices.targets().iter().any(|selection| {
                            selection.targets().contains(&Target::Permanent(leovold))
                        })
                }
                _ => false,
            })
            .expect("you can point it at your own creature");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game, true);

    assert!(game.players[0].hand.is_empty(), "nothing was drawn");
}

/// "You may": declining leaves the card in the library.
#[test]
fn the_draw_may_be_declined() {
    let (mut game, _) = staged();
    let bolt = game
        .build_zone(PlayerId::Two, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = bolt.id;
    game.players[1].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.priority = PlayerId::Two;
    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held))
        .expect("they can cast it");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    settle(&mut game, false);

    assert!(game.players[0].hand.is_empty(), "you declined the card");
}

/// "If you and a permanent you control each become the target of the same
/// spell or ability an opponent controls, Leovold's ability will trigger
/// twice." A Kolaghan's Command that names you and something of yours pays
/// you two cards for the privilege.
#[test]
fn one_spell_naming_two_of_yours_triggers_twice() {
    let (mut game, leovold) = staged();
    let command = game
        .build_zone(PlayerId::Two, &[cards::KOLAGHAN_S_COMMAND])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = command.id;
    game.players[1].hand.push(command);
    game.players[0]
        .hand
        .push(card(110_500, cards::ISLAND, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Colorless, 1);
    game.priority = PlayerId::Two;

    // The discard mode names you; the damage mode names your Elf.
    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == held
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Player(PlayerId::One))
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(leovold))
            }
            _ => false,
        })
        .expect("two modes can name you and your Elf at once");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    // The Command asks Player One for a discard as well as offering the two
    // draws, so every decision is answered by taking what it offers.
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .filter(|option| option.label != "Decline")
                .map(|option| option.id)
                .take(decision.minimum.max(1))
                .collect();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the offered choice is legal");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    // One card is discarded to the Command's own mode, so what is left in
    // hand is what the two triggers drew.
    assert_eq!(
        game.players[0].hand.len(),
        2,
        "two triggers, two cards, less the one the Command took",
    );
}

/// "Your opponents can draw a maximum of one card each on each player's
/// turn." The bound is not only about their own turn: it is one on yours
/// too, and the turn rolling over is what gives them another.
#[test]
fn the_bound_holds_on_every_players_turn() {
    let (mut game, _) = staged();
    game.active_player = PlayerId::One;

    game.draw_cards(PlayerId::Two, 2);
    assert_eq!(game.players[1].hand.len(), 1, "one on your turn, not two");

    game.start_next_turn();
    drain_pending(&mut game);
    let held = game.players[1].hand.len();

    game.draw_cards(PlayerId::Two, 2);
    assert_eq!(
        game.players[1].hand.len(),
        held + 1,
        "and one more once the turn has turned over",
    );
}
