//! Nadu, Winged Wisdom: every targeting spell you own is a card, twice per
//! creature per turn.

use super::*;

/// Nadu on the battlefield under Player One with `library` on top of their
/// library -- the last is the top card -- and a bear beside him.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for definition in library {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].library.push(card);
    }
    let nadu = game
        .put_onto_battlefield(PlayerId::One, cards::NADU_WINGED_WISDOM)
        .expect("cataloged");
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, nadu, bears)
}

fn settle(game: &mut Game) {
    for _ in 0..32 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .map(|option| option.id)
                .take(decision.minimum.max(1).min(decision.maximum))
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
    drain_pending(game);
}

/// Player One points a Giant Growth at `target` and lets it resolve.
fn pump(game: &mut Game, target: GameObjectId) {
    let card = game
        .build_zone(PlayerId::One, &[cards::GIANT_GROWTH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[0].hand.push(card);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    let action =
        game.legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == held
                        && choices.targets().iter().any(|selection| {
                            selection.targets().contains(&Target::Permanent(target))
                        })
                }
                _ => false,
            })
            .expect("it can point at that creature");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game);
}

/// A land on top goes to the battlefield.
#[test]
fn a_targeted_creature_puts_a_land_onto_the_battlefield() {
    let (mut game, _, bears) = staged(&[cards::ISLAND, cards::FOREST]);

    pump(&mut game, bears);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::FOREST),
        "the land arrived",
    );
    assert!(game.players[0].hand.is_empty(), "and not into the hand");
    assert_eq!(game.players[0].library.len(), 1);
}

/// Anything else goes to the hand.
#[test]
fn a_nonland_goes_to_the_hand() {
    let (mut game, _, bears) = staged(&[cards::ISLAND, cards::LIGHTNING_BOLT]);

    pump(&mut game, bears);

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "the Bolt is in hand",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::LIGHTNING_BOLT),
    );
}

/// Nadu is a creature you control, so pointing something at him counts too.
#[test]
fn nadu_triggers_off_himself() {
    let (mut game, nadu, _) = staged(&[cards::ISLAND, cards::FOREST]);

    pump(&mut game, nadu);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::FOREST),
        "he has the granted ability like everything else",
    );
}

/// "Only twice each turn": the third spell aimed at the same creature does
/// nothing, and a different creature has its own two.
#[test]
fn each_creature_gets_two_a_turn() {
    let (mut game, nadu, bears) = staged(&[
        cards::ISLAND,
        cards::LIGHTNING_BOLT,
        cards::LIGHTNING_BOLT,
        cards::LIGHTNING_BOLT,
        cards::LIGHTNING_BOLT,
    ]);

    pump(&mut game, bears);
    pump(&mut game, bears);
    assert_eq!(game.players[0].hand.len(), 2, "two off the bear");

    pump(&mut game, bears);
    assert_eq!(game.players[0].hand.len(), 2, "and the third gives nothing");

    pump(&mut game, nadu);
    assert_eq!(
        game.players[0].hand.len(),
        3,
        "but Nadu's own copy has not been spent",
    );
}

/// The cap is per turn: a new turn is two more.
#[test]
fn the_cap_resets_with_the_turn() {
    let (mut game, _, bears) = staged(&[
        cards::ISLAND,
        cards::LIGHTNING_BOLT,
        cards::LIGHTNING_BOLT,
        cards::LIGHTNING_BOLT,
    ]);
    pump(&mut game, bears);
    pump(&mut game, bears);
    pump(&mut game, bears);
    assert_eq!(game.players[0].hand.len(), 2);

    let turn = game.turn;
    for _ in 0..80 {
        if game.turn > turn + 1 {
            break;
        }
        game.advance_step();
        settle(&mut game);
    }
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    let before = game.players[0].hand.len();

    pump(&mut game, bears);

    assert_eq!(
        game.players[0].hand.len(),
        before + 1,
        "the count is a per-turn one",
    );
}

/// A creature an opponent controls has no such ability.
#[test]
fn their_creature_gives_them_nothing() {
    let (mut game, _, _) = staged(&[cards::ISLAND, cards::FOREST]);
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    drain_pending(&mut game);

    pump(&mut game, theirs);

    assert!(
        game.players[0].hand.is_empty(),
        "their creature carries nothing of Nadu's",
    );
    assert_eq!(game.players[0].library.len(), 2, "the library is untouched");
}

/// "If Nadu leaves the battlefield while one or more triggered abilities
/// generated by the ability it grants are still on the stack, those
/// abilities will still resolve as normal."
#[test]
fn a_trigger_already_on_the_stack_outlives_him() {
    let (mut game, nadu, bears) = staged(&[cards::LIGHTNING_BOLT]);
    let card = game
        .build_zone(PlayerId::One, &[cards::GIANT_GROWTH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[0].hand.push(card);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);

    let action =
        game.legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == held
                        && choices.targets().iter().any(|selection| {
                            selection.targets().contains(&Target::Permanent(bears))
                        })
                }
                _ => false,
            })
            .expect("it can point at the bear");
    game.apply(PlayerId::One, action).expect("it is cast");

    game.destroy_permanent(nadu);
    game.check_state_based_actions();
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == nadu),
        "he is gone before anything resolves",
    );
    settle(&mut game);

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "and the trigger he made still paid out",
    );
}

/// "If Nadu somehow enters the battlefield while a spell that targets a
/// creature you control is on the stack, the ability won't trigger, since
/// that creature didn't have the ability when it became the target."
#[test]
fn arriving_after_the_targeting_grants_nothing_for_it() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    let card = game
        .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    game.players[0].library.push(card);
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let growth = game
        .build_zone(PlayerId::One, &[cards::GIANT_GROWTH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = growth.id;
    game.players[0].hand.push(growth);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    let action =
        game.legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == held
                        && choices.targets().iter().any(|selection| {
                            selection.targets().contains(&Target::Permanent(bears))
                        })
                }
                _ => false,
            })
            .expect("it can point at the bear");
    game.apply(PlayerId::One, action).expect("it is cast");

    // He turns up while the spell that named the bear is still waiting.
    game.put_onto_battlefield(PlayerId::One, cards::NADU_WINGED_WISDOM)
        .expect("cataloged");
    settle(&mut game);

    assert!(
        game.players[0].hand.is_empty(),
        "the bear had no such ability when it was named",
    );
    assert_eq!(
        game.players[0].library.len(),
        1,
        "so nothing was revealed off the top",
    );
}

/// "Becomes the target of a spell *or ability*": a Manifold Key pointed at
/// your own bear is an ability, and the Bird reads it the same way it reads
/// a Giant Growth.
#[test]
fn an_ability_that_targets_a_creature_sets_it_off() {
    let (mut game, _nadu, bears) = staged(&[cards::LIGHTNING_BOLT]);
    let key = game
        .put_onto_battlefield(PlayerId::One, cards::MANIFOLD_KEY)
        .expect("cataloged");
    drain_pending(&mut game);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == key)
        .expect("it is there")
        .entered_controller_turn = 0;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    game.priority = PlayerId::One;
    let library = game.players[PlayerId::One.index()].library.len();

    let unblockable = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == key
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(bears))
            }
            _ => false,
        })
        .expect("the Key names a creature");
    game.apply(PlayerId::One, unblockable)
        .expect("the ability activates");
    settle(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        library - 1,
        "the top card was revealed and taken",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "and a nonland goes to hand",
    );
}

/// The clause says nothing about whose spell it is: their removal aimed at
/// your creature is a targeting like any other, and the card is drawn before
/// the creature dies.
#[test]
fn their_spell_aimed_at_your_creature_sets_it_off_too() {
    let (mut game, _nadu, bears) = staged(&[cards::LIGHTNING_BOLT]);
    let bolt = game
        .build_zone(PlayerId::Two, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let bolt_id = bolt.id;
    game.players[PlayerId::Two.index()].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.priority = PlayerId::Two;
    let library = game.players[PlayerId::One.index()].library.len();

    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == bolt_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(bears))
            }
            _ => false,
        })
        .expect("their Bolt can point at your bear");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    settle(&mut game);
    game.check_state_based_actions();

    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        library - 1,
        "your Bird read their targeting",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears),
        "and the bear still died to the Bolt",
    );
}
