//! Lotus Cobra: two mana that turns every land after it into a Lotus Petal.

use super::*;

/// The Cobra on the battlefield since last turn, with a land in hand.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let cobra = game
        .put_onto_battlefield(PlayerId::One, cards::LOTUS_COBRA)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    game.empty_mana_pools();
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].lands_played_this_turn = 0;
    (game, cobra)
}

/// Answers the colour question with `color` and lets the trigger finish.
fn settle_choosing(game: &mut Game, color: &str) {
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .filter(|option| option.label == color)
                .map(|option| option.id)
                .take(1)
                .collect::<Vec<_>>();
            assert!(!options.is_empty(), "{color} is on offer: {decision:?}");
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the offered colour is legal");
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

/// Plays a land from hand, which is what the trigger watches for.
fn play_land(game: &mut Game, definition: CardDefinitionId) {
    let land = game
        .build_zone(PlayerId::One, &[definition])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = land.id;
    game.players[0].hand.push(land);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == id))
        .expect("the land can be played");
    game.apply(PlayerId::One, action).expect("it is played");
}

fn pool(game: &Game, color: ManaColor) -> u16 {
    game.players[0].mana_pool.amount(color)
}

/// A land of yours entering makes a mana of the colour you name.
#[test]
fn a_land_you_play_makes_a_mana_of_any_color() {
    let (mut game, _cobra) = staged();

    play_land(&mut game, cards::FOREST);
    settle_choosing(&mut game, "Blue");

    assert_eq!(pool(&game, ManaColor::Blue), 1, "the colour you asked for");
    // The Forest itself is untapped and unused; nothing tapped it.
    assert_eq!(pool(&game, ManaColor::Green), 0);
}

/// It is not a mana ability: the trigger goes on the stack and the mana
/// arrives when it resolves.
#[test]
fn the_trigger_uses_the_stack() {
    let (mut game, _cobra) = staged();

    play_land(&mut game, cards::ISLAND);

    assert_eq!(game.stack.len(), 1, "the landfall trigger is waiting");
    assert_eq!(
        pool(&game, ManaColor::Red),
        0,
        "and nothing has been added yet",
    );

    settle_choosing(&mut game, "Red");

    assert_eq!(pool(&game, ManaColor::Red), 1, "now it has");
}

/// Their land is not one of yours.
#[test]
fn their_land_does_nothing() {
    let (mut game, _cobra) = staged();

    game.put_onto_battlefield(PlayerId::Two, cards::MOUNTAIN)
        .expect("cataloged");
    drain_pending(&mut game);

    assert!(game.stack.is_empty(), "nothing triggered");
    assert_eq!(game.players[0].mana_pool.total(), 0);
}

/// Every land is another trigger: a second one this turn pays again.
#[test]
fn a_second_land_pays_again() {
    let (mut game, _cobra) = staged();
    play_land(&mut game, cards::FOREST);
    settle_choosing(&mut game, "Green");
    // A second land play needs the allowance; the trigger does not care how
    // the land arrived, so put this one straight onto the battlefield.
    game.put_onto_battlefield(PlayerId::One, cards::ISLAND)
        .expect("cataloged");
    settle_choosing(&mut game, "Green");

    assert_eq!(pool(&game, ManaColor::Green), 2, "one for each land");
}
