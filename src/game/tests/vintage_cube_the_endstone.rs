//! The Endstone: a card for everything you do, and the ten life handed back
//! every end step that makes the seven mana payable.

use super::*;

/// The Endstone on the battlefield under Player One, with a stocked library
/// and `hand` in hand.
fn staged(hand: &[CardDefinitionId]) -> (Game, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..10 {
        game.players[0]
            .library
            .push(card(119_000 + index, cards::ISLAND, PlayerId::One));
    }
    game.put_onto_battlefield(PlayerId::One, cards::THE_ENDSTONE)
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
    game.players[0].lands_played_this_turn = 0;
    game.players[0].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
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

/// A land drop draws a card.
#[test]
fn playing_a_land_draws() {
    let (mut game, held) = staged(&[cards::FOREST]);
    let library = game.players[0].library.len();

    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == held[0]))
        .expect("the land drop is available");
    game.apply(PlayerId::One, play).expect("it is played");
    settle(&mut game);

    assert_eq!(game.players[0].library.len(), library - 1, "one card drawn");
    assert_eq!(game.players[0].hand.len(), 1);
}

/// So does casting a spell.
#[test]
fn casting_a_spell_draws() {
    let (mut game, held) = staged(&[cards::LIGHTNING_BOLT]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    let library = game.players[0].library.len();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held[0]))
        .expect("one red mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert_eq!(game.players[0].library.len(), library - 1, "one card drawn");
}

/// A land an effect puts onto the battlefield was never played, so nothing
/// is drawn for it.
#[test]
fn a_land_put_onto_the_battlefield_draws_nothing() {
    let (mut game, _) = staged(&[]);
    let library = game.players[0].library.len();

    game.put_onto_battlefield(PlayerId::One, cards::FOREST)
        .expect("cataloged");
    settle(&mut game);

    assert_eq!(game.players[0].library.len(), library, "nothing was played");
}

/// Their land drop is not yours.
#[test]
fn their_land_drop_draws_nothing() {
    let (mut game, _) = staged(&[]);
    let theirs = game
        .build_zone(PlayerId::Two, &[cards::FOREST])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = theirs.id;
    game.players[1].hand.push(theirs);
    game.players[1].lands_played_this_turn = 0;
    game.active_player = PlayerId::Two;
    game.priority = PlayerId::Two;
    let library = game.players[0].library.len();

    let play = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == held))
        .expect("their land drop is available");
    game.apply(PlayerId::Two, play).expect("it is played");
    settle(&mut game);

    assert_eq!(
        game.players[0].library.len(),
        library,
        "\"you play\" is you"
    );
}

/// The end step sets your life to ten, from above and from below alike.
#[test]
fn the_end_step_sets_life_to_half_the_start() {
    for before in [3, 20, 40] {
        let (mut game, _) = staged(&[]);
        game.players[0].life = before;
        game.step = Step::End;
        game.begin_step_triggers();
        settle(&mut game);

        assert_eq!(
            game.players[0].life, 10,
            "twenty to start makes ten either way, from {before}",
        );
    }
}

/// It is your end step and not theirs.
#[test]
fn their_end_step_leaves_your_life_alone() {
    let (mut game, _) = staged(&[]);
    game.players[0].life = 3;
    game.active_player = PlayerId::Two;
    game.priority = PlayerId::Two;
    game.step = Step::End;
    game.begin_step_triggers();
    settle(&mut game);

    assert_eq!(game.players[0].life, 3, "their end step is not yours");
}
