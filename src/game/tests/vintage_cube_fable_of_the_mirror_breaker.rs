//! Fable of the Mirror-Breaker: three chapters read one lore counter at a
//! time, and the third one turns the Saga into a creature.

use super::*;

/// The Saga on the battlefield under Player One, with `hand` in hand.
fn staged(hand: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for (index, definition) in [cards::MOUNTAIN, cards::FOREST, cards::PLAINS]
        .into_iter()
        .enumerate()
    {
        let id = 280_000 + u32::try_from(index).expect("three cards");
        game.players[0]
            .library
            .push(card(id, definition, PlayerId::One));
    }
    for (index, definition) in hand.iter().enumerate() {
        let id = 280_100 + u32::try_from(index).expect("a short hand");
        game.players[0]
            .hand
            .push(card(id, *definition, PlayerId::One));
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    let fable = game
        .put_onto_battlefield(PlayerId::One, cards::FABLE_OF_THE_MIRROR_BREAKER)
        .expect("cataloged");
    settle(&mut game);
    (game, fable)
}

fn settle(game: &mut Game) {
    for _ in 0..40 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .map(|option| option.id)
                .take(decision.minimum)
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

fn lore(game: &Game, saga: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == saga)
        .map_or(0, |permanent| permanent.counters(CounterKind::Lore))
}

fn tokens(game: &Game) -> Vec<GameObjectId> {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Token)
        .map(|permanent| permanent.card.id)
        .collect()
}

/// Carries the turn round to Player One's next precombat main phase, which
/// is where the next lore counter goes on.
fn next_turn(game: &mut Game) {
    // At least one step first: the caller is standing in the main phase this
    // is meant to leave.
    game.advance_step();
    settle(game);
    for _ in 0..64 {
        if game.step == Step::PrecombatMain && game.active_player == PlayerId::One {
            break;
        }
        game.advance_step();
        settle(game);
    }
}

/// It arrives with one lore counter and reads its first chapter at once.
#[test]
fn it_enters_reading_its_first_chapter() {
    let (game, fable) = staged(&[]);

    assert_eq!(lore(&game, fable), 1, "a lore counter as it enters");
    let made = tokens(&game);
    assert_eq!(made.len(), 1, "and the Goblin it makes");
    let goblin = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == made[0])
        .expect("still there");
    assert_eq!(game.power(goblin), Some(2));
    assert_eq!(game.toughness(goblin), Some(2));
}

/// The Goblin brings a Treasure with every attack.
#[test]
fn the_goblin_makes_treasure_when_it_attacks() {
    let (mut game, _fable) = staged(&[]);
    let goblin = tokens(&game).into_iter().next().expect("one Goblin");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(goblin, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    settle(&mut game);

    assert_eq!(tokens(&game).len(), 2, "the Goblin and its Treasure");
}

/// The second chapter comes after the next draw step, and loots.
#[test]
fn the_second_chapter_loots() {
    let (mut game, fable) = staged(&[cards::LIGHTNING_BOLT, cards::SERRA_ANGEL]);
    assert_eq!(game.players[0].hand.len(), 2);

    next_turn(&mut game);

    assert_eq!(lore(&game, fable), 2, "one more lore counter");
    assert_eq!(
        game.players[0].graveyard.len(),
        0,
        "the settle helper declines the discard",
    );
    assert_eq!(game.players[0].hand.len(), 3, "and the turn's draw arrived");
}

/// The third chapter exiles the Saga and brings it back as a creature.
#[test]
fn the_third_chapter_turns_it_into_a_creature() {
    let (mut game, fable) = staged(&[]);
    next_turn(&mut game);
    next_turn(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == fable),
        "the Saga that was there is gone",
    );
    let reflection = game
        .battlefield
        .iter()
        .find(|permanent| {
            permanent.card.definition == ObjectKind::Card(cards::FABLE_OF_THE_MIRROR_BREAKER)
        })
        .expect("and something of it came back");
    assert_eq!(game.power(reflection), Some(2), "as a 2/2");
    assert_eq!(game.toughness(reflection), Some(2));
    assert_eq!(
        reflection.counters(CounterKind::Lore),
        0,
        "a new object, with none of the counters it had",
    );
    assert!(
        game.players[0].graveyard.is_empty(),
        "and it was never sacrificed",
    );
}

/// The Reflection copies another creature, with haste, until end of turn.
#[test]
fn the_reflection_copies_a_creature_for_the_turn() {
    let (mut game, _fable) = staged(&[]);
    next_turn(&mut game);
    next_turn(&mut game);
    let reflection = game
        .battlefield
        .iter()
        .find(|permanent| {
            permanent.card.definition == ObjectKind::Card(cards::FABLE_OF_THE_MIRROR_BREAKER)
        })
        .map(|permanent| permanent.card.id)
        .expect("the Reflection is there");
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    settle(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    let before = tokens(&game);

    let activation = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == reflection
                    && targets
                        .iter()
                        .any(|slot| slot.targets().contains(&Target::Permanent(bears)))
            }
            _ => false,
        })
        .expect("one mana and a tap copies the Bears");
    game.apply(PlayerId::One, activation)
        .expect("it is activated");
    settle(&mut game);

    // The Goblin from the first chapter is still around, so the copy is
    // whichever token was not there before.
    let copy_id = tokens(&game)
        .into_iter()
        .find(|token| !before.contains(token))
        .expect("a copy arrived");
    let copy = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == copy_id)
        .expect("still there");
    assert_eq!(game.power(copy), Some(2), "a copy of the Bears");
    assert!(
        game.permanent_has_executable_keyword(copy, KeywordAbility::Haste),
        "except it has haste",
    );
    game.step = Step::End;
    game.begin_step_triggers();
    settle(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == copy_id),
        "and it is sacrificed at the end step",
    );
}

/// It may not copy itself: "another" leaves the Reflection out.
#[test]
fn the_reflection_cannot_copy_itself() {
    let (mut game, _fable) = staged(&[]);
    next_turn(&mut game);
    next_turn(&mut game);
    let reflection = game
        .battlefield
        .iter()
        .find(|permanent| {
            permanent.card.definition == ObjectKind::Card(cards::FABLE_OF_THE_MIRROR_BREAKER)
        })
        .map(|permanent| permanent.card.id)
        .expect("the Reflection is there");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);

    assert!(
        !game.legal_actions(PlayerId::One).into_iter().any(|action| {
            matches!(action, Action::ActivateAbility { source, targets, .. }
                if source == reflection
                    && targets
                        .iter()
                        .any(|slot| slot.targets().contains(&Target::Permanent(reflection))))
        }),
        "it is not among its own choices",
    );
}
