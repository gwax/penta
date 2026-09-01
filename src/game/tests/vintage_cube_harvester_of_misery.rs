//! Harvester of Misery: a sweeper on a body, or a shrink from the hand when
//! the board does not need sweeping.

use super::*;

/// Player One holding the Spirit, with `mana` of every color.
fn staged(mana: u16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let harvester = game
        .build_zone(PlayerId::One, &[cards::HARVESTER_OF_MISERY])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let harvester_id = harvester.id;
    game.players[0].hand.push(harvester);
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    for color in ManaColor::COLORS {
        game.add_unrestricted_mana(PlayerId::One, color, mana);
    }
    (game, harvester_id)
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
    game.check_state_based_actions();
}

fn on_battlefield(game: &Game, definition: CardDefinitionId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.definition == ObjectKind::Card(definition))
}

/// Casting it sweeps every other creature and leaves the Spirit standing.
#[test]
fn arriving_shrinks_everything_else() {
    let (mut game, harvester) = staged(5);
    game.put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    game.put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    game.put_onto_battlefield(PlayerId::One, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    drain_pending(&mut game);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == harvester))
        .expect("five mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert!(
        !on_battlefield(&game, cards::GRIZZLY_BEARS),
        "a 2/2 does not survive -2/-2",
    );
    assert!(
        !on_battlefield(&game, cards::SAVANNAH_LIONS),
        "and yours dies with theirs",
    );
    let angel = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Card(cards::SERRA_ANGEL))
        .expect("a 4/4 survives it");
    assert_eq!(game.power(angel), Some(2));
    assert_eq!(game.toughness(angel), Some(2));

    let spirit = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Card(cards::HARVESTER_OF_MISERY))
        .expect("the Spirit is there");
    assert_eq!(game.power(spirit), Some(5), "it does not shrink itself");
    assert_eq!(game.toughness(spirit), Some(4));
}

/// Menace is printed on it, so one blocker cannot finish the declaration.
#[test]
fn it_has_menace() {
    let (mut game, harvester) = staged(5);
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == harvester))
        .expect("five mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);
    let spirit = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Card(cards::HARVESTER_OF_MISERY))
        .map(|permanent| permanent.card.id)
        .expect("the Spirit is there");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    let blockers: Vec<_> = (0..2)
        .map(|_| {
            game.put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
                .expect("cataloged")
        })
        .collect();
    drain_pending(&mut game);

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(spirit, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    settle(&mut game);
    game.step = Step::DeclareBlockers;
    game.declare_blocker(blockers[0], spirit);

    assert!(
        !game
            .legal_actions(PlayerId::Two)
            .iter()
            .any(|action| matches!(action, Action::FinishDeclaringBlockers)),
        "one blocker is not enough for a menacing attacker",
    );

    game.declare_blocker(blockers[1], spirit);
    assert!(
        game.legal_actions(PlayerId::Two)
            .iter()
            .any(|action| matches!(action, Action::FinishDeclaringBlockers)),
        "two are",
    );
}

/// From the hand for two mana, discarding itself, it shrinks one creature.
#[test]
fn discarding_it_shrinks_one_creature() {
    let (mut game, harvester) = staged(1);
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    let activation = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == harvester
                    && targets
                        .iter()
                        .any(|slot| slot.targets().contains(&Target::Permanent(bears)))
            }
            _ => false,
        })
        .expect("two mana activates it from hand");
    game.apply(PlayerId::One, activation)
        .expect("it is activated");
    settle(&mut game);

    assert!(!on_battlefield(&game, cards::GRIZZLY_BEARS), "the 2/2 died");
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::HARVESTER_OF_MISERY],
        "the card itself paid for it",
    );
    assert!(game.players[0].hand.is_empty());
}

/// The ability is a hand ability: once the Spirit is on the battlefield
/// there is nothing left to discard, so it is not offered there.
#[test]
fn the_ability_is_not_offered_from_the_battlefield() {
    let (mut game, _harvester) = staged(3);
    let spirit = game
        .put_onto_battlefield(PlayerId::One, cards::HARVESTER_OF_MISERY)
        .expect("cataloged");
    game.put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    settle(&mut game);

    assert!(
        !game.legal_actions(PlayerId::One).into_iter().any(|action| {
            matches!(action, Action::ActivateAbility { source, .. } if source == spirit)
        }),
        "the permanent has no activated ability of its own",
    );
}

/// "Creatures that enter the battlefield later in the turn won't get
/// -2/-2." The sweep is a one-shot effect that reads the battlefield as it
/// resolves, not a shrinking blanket over the rest of the turn.
#[test]
fn a_creature_arriving_afterwards_is_untouched() {
    let (mut game, harvester) = staged(5);
    game.put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == harvester))
        .expect("five mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    let angel = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Card(cards::SERRA_ANGEL))
        .expect("a 4/4 survives it");
    assert_eq!(game.toughness(angel), Some(2), "the sweep caught the Angel");

    // Same turn, after the trigger has resolved.
    game.put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.check_state_based_actions();

    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Card(cards::GRIZZLY_BEARS))
        .expect("a 2/2 that was never shrunk is still a 2/2");
    assert_eq!(game.power(bears), Some(2));
    assert_eq!(game.toughness(bears), Some(2));
}

/// Nothing on the ability says when: it is an activated ability from hand,
/// so it answers an attacker in the middle of their combat, which is the
/// half of the card that plays like a trick.
#[test]
fn it_shrinks_an_attacker_in_the_middle_of_their_combat() {
    let (mut game, harvester) = staged(1);
    let attacker = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == attacker)
        .expect("it is there")
        .entered_controller_turn = 0;
    game.active_player = PlayerId::Two;
    game.step = Step::DeclareAttackers;
    game.priority = PlayerId::Two;
    game.declare_attacker(attacker, AttackDefender::Player(PlayerId::One));
    game.finish_declaring_attackers();
    game.priority = PlayerId::One;

    let activation = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == harvester
                    && targets
                        .iter()
                        .any(|slot| slot.targets().contains(&Target::Permanent(attacker)))
            }
            _ => false,
        })
        .expect("their attack step is as good a time as any");
    game.apply(PlayerId::One, activation)
        .expect("it is activated");
    settle(&mut game);

    assert!(
        !on_battlefield(&game, cards::GRIZZLY_BEARS),
        "the attacker died before it dealt any damage",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        20,
        "which is the whole point of doing it then",
    );
}

/// The discard is a cost rather than an effect: paid as the ability is
/// activated, and gone whether or not the ability does anything. A target
/// that leaves in response takes the ability and leaves the card in the
/// graveyard.
#[test]
fn the_card_is_spent_even_when_the_target_is_gone() {
    let (mut game, harvester) = staged(1);
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    let angel = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);

    let activation = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == harvester
                    && targets
                        .iter()
                        .any(|slot| slot.targets().contains(&Target::Permanent(bears)))
            }
            _ => false,
        })
        .expect("two mana activates it from hand");
    game.apply(PlayerId::One, activation)
        .expect("it is activated");
    assert!(
        game.players[PlayerId::One.index()].hand.is_empty(),
        "the card left the hand as the cost was paid",
    );

    // In response, the creature it named is gone.
    game.move_permanents_to_graveyard(&[bears]);
    settle(&mut game);

    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::HARVESTER_OF_MISERY),
        "the Spirit was spent regardless",
    );
    let survivor = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == angel)
        .expect("the Angel was never named");
    assert_eq!(
        (game.power(survivor), game.toughness(survivor)),
        (Some(4), Some(4)),
        "and nothing else was shrunk in its place",
    );
}

/// "Until end of turn": what survived the sweep is its own size again the
/// next turn, so a 3/3 that stood there as a 1/1 is a 3/3 once cleanup has
/// been through.
#[test]
fn the_sweep_wears_off_with_the_turn() {
    let (mut game, harvester) = staged(2);
    let spider = game
        .put_onto_battlefield(PlayerId::Two, cards::GIANT_SPIDER)
        .expect("cataloged");
    drain_pending(&mut game);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == harvester))
        .expect("five mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    let shrunk = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == spider)
        .expect("four toughness survived two");
    assert_eq!(
        (game.power(shrunk), game.toughness(shrunk)),
        (Some(0), Some(2)),
        "a 2/4 shrunk by two",
    );

    game.cleanup();
    game.finish_cleanup();
    drain_pending(&mut game);

    let after = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == spider)
        .expect("it is still there");
    assert_eq!(
        (game.power(after), game.toughness(after)),
        (Some(2), Some(4)),
        "and its own size again afterwards",
    );
}

/// The shrink modifies rather than sets, so two of them stack: a second
/// Harvester discarded at the same creature takes another two off it.
#[test]
fn two_shrinks_stack_on_one_creature() {
    let (mut game, first) = staged(2);
    let second = game
        .build_zone(PlayerId::One, &[cards::HARVESTER_OF_MISERY])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let second_id = second.id;
    game.players[0].hand.push(second);
    let spider = game
        .put_onto_battlefield(PlayerId::Two, cards::GIANT_SPIDER)
        .expect("cataloged");
    drain_pending(&mut game);

    for source in [first, second_id] {
        let activation = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::ActivateAbility {
                    source: activated,
                    targets,
                    ..
                } => {
                    *activated == source
                        && targets
                            .iter()
                            .any(|slot| slot.targets().contains(&Target::Permanent(spider)))
                }
                _ => false,
            })
            .expect("two mana activates it from hand");
        game.apply(PlayerId::One, activation)
            .expect("it is activated");
        settle(&mut game);
    }

    assert!(
        !on_battlefield(&game, cards::GIANT_SPIDER),
        "two and two is four, which a 2/4 does not survive",
    );
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .filter(|card| card.definition == cards::HARVESTER_OF_MISERY)
            .count(),
        2,
        "and both cards paid for it",
    );
}
