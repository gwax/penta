//! Llanowar Elves: one green mana a turn from a body, which is a different
//! thing from a land in every way that matters -- it waits a turn, it uses
//! no stack, and what it makes is gone when the step ends.

use super::*;

/// The Elf on the battlefield since last turn, in Player One's main phase.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let elf = game
        .put_onto_battlefield(PlayerId::One, cards::LLANOWAR_ELVES)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.empty_mana_pools();
    (game, elf)
}

fn tap_for_green(game: &mut Game, elf: GameObjectId) {
    let green = Action::ActivateManaAbility {
        source: elf,
        ability: mana_ability_for(game, elf, ManaColor::Green),
        color: ManaColor::Green,
        counters_removed: None,
        cost_object: None,
        combination: None,
    };
    game.apply(PlayerId::One, green).expect("it taps for green");
}

/// One green, the Elf tapped, and nothing on the stack to answer: a mana
/// ability resolves as it is activated (CR 605.3a).
#[test]
fn it_taps_for_one_green_without_using_the_stack() {
    let (mut game, elf) = staged();
    tap_for_green(&mut game, elf);

    assert_eq!(game.players[0].mana_pool.green, 1);
    assert_eq!(game.players[0].mana_pool.total(), 1, "and nothing else");
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == elf && permanent.tapped),
        "the tap was the cost",
    );
    assert!(
        game.stack.is_empty(),
        "a mana ability never goes on the stack, so it cannot be answered",
    );
}

/// A tapped Elf has nothing left to give until it untaps.
#[test]
fn it_only_taps_once() {
    let (mut game, elf) = staged();
    tap_for_green(&mut game, elf);

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(
                action,
                Action::ActivateManaAbility { source, .. } if *source == elf
            )),
        "the cost is a tap and it is already tapped",
    );
}

/// "Mana pools empty at the end of each step and phase" (CR 500.4): the
/// green the Elf made in the main phase is not there to spend in combat.
#[test]
fn the_green_does_not_survive_the_step() {
    let (mut game, elf) = staged();
    tap_for_green(&mut game, elf);
    assert_eq!(game.players[0].mana_pool.green, 1);

    game.advance_step();

    assert_eq!(
        game.players[0].mana_pool.total(),
        0,
        "the step ended and took the mana with it",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == elf && permanent.tapped),
        "and the Elf stays tapped for the rest of the turn",
    );
}
