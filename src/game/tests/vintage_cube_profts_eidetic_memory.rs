//! Proft's Eidetic Memory: a cantrip that turns every later draw into
//! permanent power.

use super::*;

/// The Memory in hand with two mana up, a creature out, and a library to
/// draw from.
fn staged() -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..10 {
        game.players[0]
            .library
            .push(card(91_000 + index, cards::ISLAND, PlayerId::One));
    }
    let bears = creature(91_500, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let memory = game
        .build_zone(PlayerId::One, &[cards::PROFT_S_EIDETIC_MEMORY])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let memory_id = memory.id;
    game.players[0].hand.push(memory);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.cards_drawn_this_turn = [0, 0];
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    (game, memory_id, bears_id)
}

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

fn cast_memory(game: &mut Game, memory: GameObjectId) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == memory))
        .expect("two mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(game);
}

/// Runs the beginning of combat and answers the trigger if it fires.
fn begin_combat(game: &mut Game) {
    game.step = Step::BeginningOfCombat;
    game.begin_step_triggers();
    settle(game);
}

fn counters_on(game: &Game, permanent: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|candidate| candidate.card.id == permanent)
        .map_or(0, |candidate| {
            candidate.counters(CounterKind::PlusOnePlusOne)
        })
}

/// It replaces itself, which is the first draw of the turn.
#[test]
fn it_draws_a_card_as_it_enters() {
    let (mut game, memory, _) = staged();
    let hand = game.players[0].hand.len();

    cast_memory(&mut game, memory);

    assert_eq!(
        game.players[0].hand.len(),
        hand,
        "one card out of hand and one in",
    );
    assert_eq!(game.cards_drawn_this_turn[0], 1);
}

/// One draw is not more than one: the trigger does not fire on the turn it
/// only replaced itself.
#[test]
fn one_draw_is_not_enough() {
    let (mut game, memory, bears) = staged();
    cast_memory(&mut game, memory);

    begin_combat(&mut game);

    assert_eq!(counters_on(&game, bears), 0, "no counters for one draw");
}

/// The second draw is the first counter, and the third is the second.
#[test]
fn every_draw_past_the_first_is_a_counter() {
    let (mut game, memory, bears) = staged();
    cast_memory(&mut game, memory);
    game.draw_card(PlayerId::One);
    game.draw_card(PlayerId::One);

    begin_combat(&mut game);

    assert_eq!(
        counters_on(&game, bears),
        2,
        "three draws minus the first one",
    );
}

/// The counters are permanent, and the count resets with the turn.
#[test]
fn the_count_resets_each_turn() {
    let (mut game, memory, bears) = staged();
    cast_memory(&mut game, memory);
    game.draw_card(PlayerId::One);
    begin_combat(&mut game);
    assert_eq!(counters_on(&game, bears), 1);

    game.start_next_turn();
    game.start_next_turn();
    game.cards_drawn_this_turn = [1, 0];
    begin_combat(&mut game);

    assert_eq!(
        counters_on(&game, bears),
        1,
        "one draw next turn adds nothing, and the counter stays",
    );
}

/// No maximum hand size: a hand of ten is discarded down to nothing.
#[test]
fn you_have_no_maximum_hand_size() {
    let (mut game, memory, _) = staged();
    cast_memory(&mut game, memory);
    for _ in 0..10 {
        game.draw_card(PlayerId::One);
    }
    let hand = game.players[0].hand.len();
    assert!(hand > 7, "a hand over the ordinary limit");

    game.step = Step::Cleanup;
    game.cleanup();
    settle(&mut game);

    assert_eq!(
        game.players[0].hand.len(),
        hand,
        "nothing was discarded at cleanup",
    );
}
