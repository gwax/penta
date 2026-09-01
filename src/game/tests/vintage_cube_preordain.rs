//! Preordain: scry 2 and then a card, and what happens when the library is
//! shorter than the look.
//!
//! The three ordinary outcomes -- keep both in an order, keep one and bury
//! the other, bury both -- are pinned in `vintage_cube_library`. What is
//! here is the short library, the private look, and the sorcery timing.

use super::*;

/// Player One holding a Preordain with the blue for it and `library`
/// stacked, the first entry on top.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for (index, definition) in library.iter().rev().enumerate() {
        game.players[0].library.push(card(
            124_000 + u32::try_from(index).expect("few cards"),
            *definition,
            PlayerId::One,
        ));
    }
    let preordain = card(124_100, cards::PREORDAIN, PlayerId::One);
    let preordain_id = preordain.id;
    game.players[0].hand.push(preordain);
    game.players[0].mana_pool.blue = 1;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, preordain_id)
}

/// Casts it, keeping everything the scry offers on top.
fn cast_keeping_everything(game: &mut Game, preordain: GameObjectId) {
    game.apply(
        PlayerId::One,
        cast_action(preordain, Vec::new(), Vec::new(), 0),
    )
    .expect("one blue casts it");
    for _ in 0..12 {
        let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        else {
            if game.stack.is_empty() && game.pending_triggers.is_empty() {
                break;
            }
            if game.apply(game.priority, Action::PassPriority).is_err() {
                break;
            }
            continue;
        };
        // Nothing chosen is nothing sent to the bottom.
        game.apply(
            decision.player,
            Action::ChooseDecision {
                decision: decision.id,
                options: Vec::new(),
            },
        )
        .expect("keeping everything is legal");
    }
    drain_pending(game);
}

/// "Look at the top two cards": a library holding one card is looked at as
/// far as it goes, and the draw takes that card.
#[test]
fn a_library_of_one_is_scried_as_far_as_it_goes() {
    let (mut game, preordain) = staged(&[cards::LIGHTNING_BOLT]);

    cast_keeping_everything(&mut game, preordain);

    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::LIGHTNING_BOLT],
        "the one card there was is the one drawn",
    );
    assert!(game.players[0].library.is_empty());
    assert_eq!(game.result(), None, "and the draw found something to take");
}

/// An empty library is scried over and then drawn from, which is how the
/// game ends.
#[test]
fn an_empty_library_scries_nothing_and_the_draw_ends_it() {
    let (mut game, preordain) = staged(&[]);

    cast_keeping_everything(&mut game, preordain);
    game.check_state_based_actions();

    assert_eq!(
        game.result(),
        Some(GameResult::Winner {
            winner: PlayerId::Two,
            reason: WinReason::OpponentTriedToDrawFromEmptyLibrary,
        }),
        "nothing to look at and nothing to draw",
    );
}

/// The scry is a look and not a reveal: the other player is not shown the
/// two cards being sorted.
#[test]
fn the_look_is_private_to_its_caster() {
    let (mut game, preordain) = staged(&[cards::LIGHTNING_BOLT, cards::SERRA_ANGEL]);

    game.apply(
        PlayerId::One,
        cast_action(preordain, Vec::new(), Vec::new(), 0),
    )
    .expect("one blue casts it");
    pass_until_decision(&mut game);

    assert!(
        game.observe(PlayerId::One).decision.is_some(),
        "its caster is asked how to sort them",
    );
    assert!(
        game.observe(PlayerId::Two).decision.is_none(),
        "and the other seat is shown nothing",
    );
}

/// It is a sorcery: their turn is no time for it, whatever the mana says.
#[test]
fn it_waits_for_your_own_main_phase() {
    let (mut game, preordain) = staged(&[cards::LIGHTNING_BOLT, cards::SERRA_ANGEL]);
    let castable = |game: &Game| {
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == preordain))
    };
    assert!(castable(&game), "your own main phase is its window");

    game.active_player = PlayerId::Two;
    assert!(!castable(&game), "and their turn is not");
}
