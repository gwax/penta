//! Shallow Grave: the newest creature back for a turn, and the exile that
//! only collects what is still standing.
//!
//! The ordinary line -- the top creature card rather than the top card,
//! haste, and the exile at the beginning of the end step -- is pinned in
//! `premodern_hermit`. What is here is its ruling and the empty case.

use super::*;

/// Player One holding a Shallow Grave with the mana for it and `graveyard`
/// behind it, oldest first.
fn staged(graveyard: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    for (index, definition) in graveyard.iter().enumerate() {
        game.players[0].graveyard.push(card(
            123_000 + u32::try_from(index).expect("few cards"),
            *definition,
            PlayerId::One,
        ));
    }
    let grave = card(123_100, cards::SHALLOW_GRAVE, PlayerId::One);
    let grave_id = grave.id;
    game.players[0].hand.push(grave);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, grave_id)
}

/// Casts it and lets it resolve.
fn cast(game: &mut Game, grave: GameObjectId) {
    game.apply(PlayerId::One, cast_action(grave, Vec::new(), Vec::new(), 0))
        .expect("two mana casts it");
    drain_pending(game);
}

/// Runs the turn to its end step and lets the delayed clause fire.
fn reach_the_end_step(game: &mut Game) {
    for _ in 0..8 {
        if game.step == Step::End {
            break;
        }
        game.advance_step();
    }
    game.finish_rules_procedure();
    pass_until_decision(game);
    drain_pending(game);
}

/// Its ruling: "only exiles the creature if the creature is still on the
/// battlefield at the beginning of the end step." A Psychatog answered
/// before then is in the graveyard, and the delayed clause does not follow
/// it there.
#[test]
fn a_creature_that_died_first_is_not_exiled_from_the_graveyard() {
    let (mut game, grave) = staged(&[cards::PSYCHATOG]);
    cast(&mut game, grave);
    let atog = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::PSYCHATOG)
        .expect("it came back")
        .card
        .id;

    game.move_permanents_to_graveyard(&[atog]);
    game.check_state_based_actions();
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::PSYCHATOG),
        "it went back where it came from",
    );

    reach_the_end_step(&mut game);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::PSYCHATOG),
        "and the end step left it lying there",
    );
    assert!(
        game.players[0]
            .exile
            .iter()
            .all(|card| card.definition != cards::PSYCHATOG),
        "rather than reaching into the graveyard for it",
    );
}

/// "The top creature card of your graveyard": with no creature card in it
/// the spell resolves and returns nothing, and the end step has nothing to
/// collect either.
#[test]
fn a_graveyard_with_no_creature_returns_nothing() {
    let (mut game, grave) = staged(&[cards::LIGHTNING_BOLT, cards::FOREST]);

    cast(&mut game, grave);

    assert!(
        game.battlefield.is_empty(),
        "a Bolt and a land are no creature card",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SHALLOW_GRAVE),
        "and the Grave resolved into the graveyard all the same",
    );

    reach_the_end_step(&mut game);

    assert!(game.players[0].exile.is_empty(), "nothing to exile");
}

/// "Your graveyard": what is lying in theirs is no part of it, however
/// tempting.
#[test]
fn it_will_not_reach_into_their_graveyard() {
    let (mut game, grave) = staged(&[]);
    game.players[1].graveyard.clear();
    game.players[1]
        .graveyard
        .push(card(123_200, cards::SERRA_ANGEL, PlayerId::Two));

    cast(&mut game, grave);

    assert!(
        game.battlefield.is_empty(),
        "their Angel stayed where it was",
    );
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "still theirs, still buried",
    );
}
