//! Kitsa, Otterball Elite: a looter that copies the spell which made her
//! big enough to copy it.

use super::*;

/// Kitsa on the battlefield under Player One since last turn, with a
/// stocked library and `hand` in hand.
fn staged(hand: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..6 {
        game.players[0]
            .library
            .push(card(113_000 + index, cards::ISLAND, PlayerId::One));
    }
    let kitsa = game
        .put_onto_battlefield(PlayerId::One, cards::KITSA_OTTERBALL_ELITE)
        .expect("cataloged");
    drain_pending(&mut game);
    let mut held = Vec::new();
    for definition in hand {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        held.push(card.id);
        game.players[0].hand.push(card);
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    for color in [ManaColor::Red, ManaColor::Blue, ManaColor::Colorless] {
        game.add_unrestricted_mana(PlayerId::One, color, 4);
    }
    game.players[1].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, kitsa, held)
}

/// Answers decisions, keeping a copy's original targets where that is one
/// of the options.
fn settle(game: &mut Game) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .find(|option| option.label.contains("Keep"))
                .map_or_else(
                    || {
                        decision
                            .options
                            .iter()
                            .map(|option| option.id)
                            .take(decision.minimum.max(1).min(decision.maximum))
                            .collect()
                    },
                    |option| vec![option.id],
                );
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

fn power(game: &Game, id: GameObjectId) -> Option<i16> {
    game.power(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == id)
            .expect("it is on the battlefield"),
    )
}

/// Every way Kitsa's copy ability can be activated right now.
fn copy_offers(game: &Game, kitsa: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| {
            matches!(
                action,
                Action::ActivateAbility {
                    source,
                    ability: AbilityOrigin::Printed { ability, .. },
                    ..
                } if *source == kitsa && *ability == AbilityId(3)
            )
        })
        .collect()
}

/// She has vigilance and starts as a 1/3. Prowess is a triggered ability
/// rather than a flag, so the tests below are what show it working.
#[test]
fn she_is_a_vigilant_prowess_body() {
    let (game, kitsa, _) = staged(&[]);
    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == kitsa)
        .expect("she is there");

    assert_eq!(game.power(body), Some(1));
    assert_eq!(game.toughness(body), Some(3));
    assert!(game.permanent_has_executable_keyword(body, KeywordAbility::Vigilance));
}

/// Tapping her draws and discards.
#[test]
fn she_loots_for_free() {
    let (mut game, kitsa, _) = staged(&[cards::MOUNTAIN]);
    let library = game.players[0].library.len();

    let loot = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                Action::ActivateAbility {
                    source,
                    ability: AbilityOrigin::Printed { ability, .. },
                    ..
                } if *source == kitsa && *ability == AbilityId(2)
            )
        })
        .expect("the tap ability is offered");
    game.apply(PlayerId::One, loot).expect("it activates");
    settle(&mut game);

    assert_eq!(game.players[0].library.len(), library - 1, "one drawn");
    assert_eq!(game.players[0].hand.len(), 1, "and one discarded");
    assert_eq!(game.players[0].graveyard.len(), 1);
}

/// The copy ability is off while she is a 1/3.
#[test]
fn a_one_power_kitsa_cannot_copy() {
    let (mut game, kitsa, held) = staged(&[cards::LIGHTNING_BOLT]);
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == held[0]
                    && choices.targets().iter().any(|selection| {
                        selection.targets().contains(&Target::Player(PlayerId::Two))
                    })
            }
            _ => false,
        })
        .expect("the Bolt can point at them");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(power(&game, kitsa), Some(2), "one prowess trigger so far");
    assert!(
        copy_offers(&game, kitsa).is_empty(),
        "two power is not three",
    );
}

/// Passes priority until only `keep` objects are left on the stack, so a
/// trigger above a spell resolves while the spell itself waits.
fn resolve_above(game: &mut Game, keep: usize) {
    for _ in 0..24 {
        if game.stack.len() <= keep && game.pending_triggers.is_empty() {
            break;
        }
        if game.observe(PlayerId::One).decision.is_some()
            || game.observe(PlayerId::Two).decision.is_some()
        {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// The second noncreature spell of the turn is what turns her on, and she
/// copies it while it is still on the stack.
#[test]
fn three_power_copies_the_spell() {
    let (mut game, kitsa, held) = staged(&[cards::LIGHTNING_BOLT, cards::LIGHTNING_BOLT]);
    let cast = |game: &mut Game, spell: GameObjectId| {
        let action = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == spell
                        && choices.targets().iter().any(|selection| {
                            selection.targets().contains(&Target::Player(PlayerId::Two))
                        })
                }
                _ => false,
            })
            .expect("the Bolt can point at them");
        game.apply(PlayerId::One, action).expect("it is cast");
    };

    cast(&mut game, held[0]);
    settle(&mut game);
    assert_eq!(game.players[1].life, 17, "the first one landed");
    assert_eq!(power(&game, kitsa), Some(2), "one prowess trigger");

    // The second Bolt waits on the stack while its own prowess trigger
    // resolves above it, which is what makes her big enough to copy it.
    cast(&mut game, held[1]);
    resolve_above(&mut game, 1);
    assert_eq!(power(&game, kitsa), Some(3), "two prowess triggers");

    let offers = copy_offers(&game, kitsa);
    assert!(!offers.is_empty(), "and now the copy is on offer");
    game.apply(PlayerId::One, offers[0].clone())
        .expect("it activates");
    settle(&mut game);

    assert_eq!(
        game.players[1].life, 11,
        "two Bolts and a copy of the second",
    );
}

/// "A spell you control": theirs is not a legal target.
#[test]
fn their_spell_is_not_a_target() {
    let (mut game, kitsa, _) = staged(&[]);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == kitsa)
        .expect("she is there")
        .set_counters(CounterKind::PlusOnePlusOne, 2);
    let theirs = game
        .build_zone(PlayerId::Two, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let their_bolt = theirs.id;
    game.players[1].hand.push(theirs);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.priority = PlayerId::Two;
    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == their_bolt))
        .expect("they can cast it");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    game.priority = PlayerId::One;

    assert_eq!(power(&game, kitsa), Some(3), "big enough to copy");
    assert!(
        copy_offers(&game, kitsa).is_empty(),
        "but their spell is not one she may point at",
    );
}

/// "The copy is created on the stack, so it's not cast. Abilities that
/// trigger when a player casts a spell won't trigger." Her own prowess is
/// such an ability, and it counts the Bolt but not the copy of it -- and the
/// copy resolves before the spell it was made from.
#[test]
fn the_copy_is_not_cast_and_resolves_first() {
    let (mut game, kitsa, held) = staged(&[cards::LIGHTNING_BOLT, cards::LIGHTNING_BOLT]);
    let cast = |game: &mut Game, spell: GameObjectId| {
        let action = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == spell
                        && choices.targets().iter().any(|selection| {
                            selection.targets().contains(&Target::Player(PlayerId::Two))
                        })
                }
                _ => false,
            })
            .expect("the Bolt can point at them");
        game.apply(PlayerId::One, action).expect("it is cast");
    };

    cast(&mut game, held[0]);
    settle(&mut game);
    cast(&mut game, held[1]);
    resolve_above(&mut game, 1);
    assert_eq!(power(&game, kitsa), Some(3), "two casts, two prowess");

    let offers = copy_offers(&game, kitsa);
    game.apply(
        PlayerId::One,
        offers.into_iter().next().expect("she may copy it"),
    )
    .expect("it activates");
    // The ability resolves, asks about the copy's targets, and puts the copy
    // on the stack above the Bolt.
    pass_priority_pair(&mut game);
    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the copy's targets are asked about");
    let keep = decision
        .options
        .iter()
        .find(|option| option.label.contains("Keep"))
        .expect("keeping them is one of the answers")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![keep],
        },
    )
    .expect("keeping the original targets is legal");

    assert_eq!(
        game.stack.len(),
        2,
        "the copy sits on the stack above the spell it came from",
    );
    assert_eq!(
        power(&game, kitsa),
        Some(3),
        "and it was put there rather than cast, so prowess did not see it",
    );

    // One object resolves: the copy, which is on top.
    pass_priority_pair(&mut game);
    game.check_state_based_actions();

    assert_eq!(game.stack.len(), 1, "one of the two resolved");
    assert_eq!(
        game.players[1].life, 14,
        "the copy went first: three off the second Bolt's copy, \
         with the Bolt itself still waiting",
    );
}

/// A copy of a spell has no card behind it, so countering one is the end of
/// it: a Remand that says "put it into its owner's hand instead" has nothing
/// to put anywhere (CR 707.10a), and still draws its card.
#[test]
fn remanding_a_copy_leaves_nothing_to_return() {
    let (mut game, kitsa, held) =
        staged(&[cards::LIGHTNING_BOLT, cards::LIGHTNING_BOLT, cards::REMAND]);
    let cast_bolt = |game: &mut Game, spell: GameObjectId| {
        let action = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == spell
                        && choices.targets().iter().any(|selection| {
                            selection.targets().contains(&Target::Player(PlayerId::Two))
                        })
                }
                _ => false,
            })
            .expect("the Bolt can point at them");
        game.apply(PlayerId::One, action).expect("it is cast");
    };

    cast_bolt(&mut game, held[0]);
    settle(&mut game);
    cast_bolt(&mut game, held[1]);
    resolve_above(&mut game, 1);
    assert_eq!(power(&game, kitsa), Some(3), "two casts, two prowess");

    game.apply(
        PlayerId::One,
        copy_offers(&game, kitsa)
            .into_iter()
            .next()
            .expect("she may copy it"),
    )
    .expect("it activates");
    pass_priority_pair(&mut game);
    let targets = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the copy's targets are asked about");
    let keep = targets
        .options
        .iter()
        .find(|option| option.label.contains("Keep"))
        .expect("keeping them is one of the answers")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: targets.id,
            options: vec![keep],
        },
    )
    .expect("keeping the original targets is legal");

    let copy = game
        .stack
        .objects
        .last()
        .expect("the copy is on top of the stack")
        .id;
    let hand = game.players[0].hand.len();
    let library = game.players[0].library.len();
    let cast_remand = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == held[2]
                    && choices
                        .targets()
                        .iter()
                        .any(|selection| selection.targets() == [Target::Spell(copy)])
            }
            _ => false,
        })
        .expect("the Remand can name the copy");
    game.apply(PlayerId::One, cast_remand).expect("it is cast");
    settle(&mut game);

    assert!(
        !game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "the copy had no card to go to hand",
    );
    assert_eq!(
        game.players[0].hand.len(),
        hand,
        "the Remand left the hand and the drawn card took its place",
    );
    assert_eq!(
        game.players[0].library.len(),
        library - 1,
        "and the draw happened all the same",
    );
    assert_eq!(
        game.players[1].life, 14,
        "the Bolt that was copied still resolved, and the copy never did",
    );
}

/// "Once you've activated Kitsa's last ability, any changes to Kitsa's power
/// won't stop the ability from resolving." Killing her is the largest change
/// there is, and the copy still happens.
#[test]
fn killing_her_after_the_activation_does_not_stop_the_copy() {
    let (mut game, kitsa, held) = staged(&[cards::LIGHTNING_BOLT]);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == kitsa)
        .expect("she is there")
        .set_counters(CounterKind::PlusOnePlusOne, 2);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == held[0]
                    && choices.targets().iter().any(|selection| {
                        selection.targets().contains(&Target::Player(PlayerId::Two))
                    })
            }
            _ => false,
        })
        .expect("the Bolt can point at them");
    game.apply(PlayerId::One, cast).expect("it is cast");
    resolve_above(&mut game, 1);

    let offers = copy_offers(&game, kitsa);
    assert!(!offers.is_empty(), "she is big enough to copy");
    game.apply(PlayerId::One, offers[0].clone())
        .expect("it activates");
    game.move_permanents_to_graveyard(&[kitsa]);
    game.check_state_based_actions();
    settle(&mut game);

    assert_eq!(
        game.players[1].life, 14,
        "the Bolt and its copy both landed, with nobody left to have made it",
    );
}

/// "The copy will have the same targets as the spell it's copying unless you
/// choose new ones." The Bolt goes at the player and its copy is pointed at
/// a creature instead.
#[test]
fn the_copy_may_be_pointed_somewhere_else() {
    let (mut game, kitsa, held) = staged(&[cards::LIGHTNING_BOLT]);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == kitsa)
        .expect("she is there")
        .set_counters(CounterKind::PlusOnePlusOne, 2);
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == held[0]
                    && choices.targets().iter().any(|selection| {
                        selection.targets().contains(&Target::Player(PlayerId::Two))
                    })
            }
            _ => false,
        })
        .expect("the Bolt can point at them");
    game.apply(PlayerId::One, cast).expect("it is cast");
    resolve_above(&mut game, 1);
    let offers = copy_offers(&game, kitsa);
    game.apply(PlayerId::One, offers[0].clone())
        .expect("it activates");

    // The copy asks where it is pointed, and the Bears are one of the answers.
    for _ in 0..12 {
        let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        else {
            if game.stack.is_empty() && game.pending_triggers.is_empty() {
                break;
            }
            let priority = game.priority;
            if game.apply(priority, Action::PassPriority).is_err() {
                break;
            }
            continue;
        };
        // The copy's target options name what they point at in their labels
        // rather than carrying the object.
        let wanted = decision
            .options
            .iter()
            .find(|option| option.label.contains("Grizzly Bears"))
            .map_or_else(
                || {
                    decision
                        .options
                        .iter()
                        .map(|option| option.id)
                        .take(decision.minimum.max(1).min(decision.maximum))
                        .collect()
                },
                |option| vec![option.id],
            );
        game.apply(
            decision.player,
            Action::ChooseDecision {
                decision: decision.id,
                options: wanted,
            },
        )
        .expect("the answer is legal");
    }
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears),
        "the copy went at the Bears and killed them",
    );
    assert_eq!(
        game.players[1].life, 17,
        "while the Bolt itself still went at the player",
    );
}
