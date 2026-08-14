//! Printed "Activate only during ..." windows.
//!
//! The restriction narrows when an ability may be activated and says nothing
//! about priority, so these drive it the way a seat meets it: by asking what
//! the legal-action list offers in each step and on each player's turn.

use super::*;
use crate::ImplementationStatus;

fn offers(game: &Game, player: PlayerId, source: GameObjectId) -> bool {
    game.legal_actions(player).iter().any(
        |action| matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source),
    )
}

fn scepter_game() -> (Game, GameObjectId) {
    let mut game = ready_game();
    let scepter = creature(10_000, cards::DISRUPTING_SCEPTER, PlayerId::One);
    let scepter_id = scepter.card.id;
    game.battlefield.push(scepter);
    game.players[PlayerId::One.index()].mana_pool.colorless = 3;
    game.players[PlayerId::Two.index()]
        .hand
        .push(card(10_500, cards::SEDGE_TROLL, PlayerId::Two));
    (game, scepter_id)
}

/// "Only during your turn" is about whose turn it is, not which step, so it
/// holds all the way through a turn its controller is taking.
#[test]
fn a_your_turn_ability_is_offered_in_every_step_of_your_own_turn() {
    let (mut game, scepter_id) = scepter_game();
    // Declare-attackers is excluded because the active player owes the game
    // an attack declaration there before anyone holds priority; that is a
    // priority rule, not this window.
    for step in [
        Step::Upkeep,
        Step::PrecombatMain,
        Step::PostcombatMain,
        Step::End,
    ] {
        game.step = step;
        game.priority = PlayerId::One;
        assert!(
            offers(&game, PlayerId::One, scepter_id),
            "the window is open in {step:?}"
        );
    }
}

#[test]
fn a_your_turn_ability_is_not_offered_on_the_opposing_turn() {
    let (mut game, scepter_id) = scepter_game();
    game.active_player = PlayerId::Two;
    game.priority = PlayerId::One;
    assert!(
        !offers(&game, PlayerId::One, scepter_id),
        "the controller's own turn is the whole window"
    );
}

/// The restriction follows the ability, not the permanent: an opponent who
/// somehow held priority still could not use it, and neither can its
/// controller once the turn has passed.
#[test]
fn an_upkeep_ability_is_offered_only_in_your_own_upkeep() {
    let mut game = ready_game();
    game.turns_started[PlayerId::One.index()] = 1;
    let smith = creature(10_000, cards::DWARVEN_WEAPONSMITH, PlayerId::One);
    let smith_id = smith.card.id;
    game.battlefield.push(smith);
    // Mishra's Factory is a Land until it animates, so the artifact this
    // cost eats has to be one that is printed as such.
    game.battlefield
        .push(creature(10_001, cards::SOL_RING, PlayerId::One));
    game.battlefield
        .push(creature(10_002, cards::SEDGE_TROLL, PlayerId::One));

    game.step = Step::PrecombatMain;
    assert!(
        !offers(&game, PlayerId::One, smith_id),
        "a main phase is not an upkeep"
    );

    game.step = Step::Upkeep;
    assert!(
        offers(&game, PlayerId::One, smith_id),
        "its own upkeep is the window"
    );

    game.active_player = PlayerId::Two;
    assert!(
        !offers(&game, PlayerId::One, smith_id),
        "and it has to be a turn its controller is taking"
    );
}

/// The window gates the ability, not the card: everything else about the
/// activation still has to hold, so a tapped source is still unavailable in
/// the open window.
#[test]
fn the_window_does_not_excuse_the_rest_of_the_cost() {
    let (mut game, scepter_id) = scepter_game();
    game.step = Step::PrecombatMain;
    assert!(offers(&game, PlayerId::One, scepter_id));

    if let Some(scepter) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == scepter_id)
    {
        scepter.tapped = true;
    }
    assert!(
        !offers(&game, PlayerId::One, scepter_id),
        "an open window does not untap the source"
    );
}

#[test]
fn every_timing_restricted_identity_reports_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [
        cards::DISRUPTING_SCEPTER,
        cards::DWARVEN_WEAPONSMITH,
        cards::SVYELUNITE_PRIEST,
        cards::GWENDLYN_DI_CORCI,
    ] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
