//! Control changes that last as long as a permanent does.
//!
//! The turn-scoped form is ended by cleanup. This one outlives the turn and
//! ends when its holder does, so what these drive is the difference: the turn
//! passing without giving the permanent back, and the holder leaving giving it
//! back immediately.

use super::*;
use crate::ImplementationStatus;

fn aladdin_game() -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.turns_started[PlayerId::One.index()] = 1;
    let aladdin = creature(10_000, cards::ALADDIN, PlayerId::One);
    let aladdin_id = aladdin.card.id;
    game.battlefield.push(aladdin);
    let ring = creature(10_001, cards::SOL_RING, PlayerId::Two);
    let ring_id = ring.card.id;
    game.battlefield.push(ring);
    game.players[PlayerId::One.index()].mana_pool.red = 2;
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    (game, aladdin_id, ring_id)
}

fn steal(game: &mut Game, source: GameObjectId, victim: GameObjectId) {
    // Thrull Champion is itself a Thrull, so the intended target has to be
    // named rather than taken from whichever activation comes first.
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source: actual,
                targets,
                ..
            } => {
                *actual == source
                    && targets
                        .iter()
                        .flat_map(crate::casting::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(victim))
            }
            _ => false,
        })
        .expect("the ability is offered against that target");
    game.apply(PlayerId::One, action)
        .expect("the ability activates");
    pass_priority_pair(game);
}

fn controller(game: &Game, id: GameObjectId) -> PlayerId {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("the permanent is on the battlefield")
        .controller
}

#[test]
fn the_stolen_permanent_stays_stolen_across_the_turn() {
    let (mut game, aladdin_id, ring_id) = aladdin_game();
    steal(&mut game, aladdin_id, ring_id);
    assert_eq!(controller(&game, ring_id), PlayerId::One);

    game.finish_cleanup();

    assert_eq!(
        controller(&game, ring_id),
        PlayerId::One,
        "this is not a turn-scoped steal, so cleanup does not give it back"
    );
}

#[test]
fn losing_the_holder_gives_the_permanent_back() {
    let (mut game, aladdin_id, ring_id) = aladdin_game();
    steal(&mut game, aladdin_id, ring_id);
    assert_eq!(controller(&game, ring_id), PlayerId::One);

    game.battlefield
        .retain(|permanent| permanent.card.id != aladdin_id);
    game.check_state_based_actions();

    assert_eq!(
        controller(&game, ring_id),
        PlayerId::Two,
        "the holder left, so the artifact went home"
    );
}

/// "For as long as *you control* this creature" is not the same as "for as
/// long as this creature is on the battlefield": losing the holder to someone
/// else ends the steal too.
#[test]
fn losing_control_of_the_holder_gives_the_permanent_back() {
    let (mut game, aladdin_id, ring_id) = aladdin_game();
    steal(&mut game, aladdin_id, ring_id);

    if let Some(aladdin) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == aladdin_id)
    {
        aladdin.controller = PlayerId::Two;
    }
    game.check_state_based_actions();

    assert_eq!(
        controller(&game, ring_id),
        PlayerId::Two,
        "the holder changed hands, so the steal ended"
    );
}

/// Thrull Champion's own anthem applies to the Thrull it takes, which is the
/// check that the two clauses see the same board.
#[test]
fn thrull_champion_pumps_the_thrull_it_steals() {
    let mut game = ready_game();
    game.turns_started[PlayerId::One.index()] = 1;
    let champion = creature(10_000, cards::THRULL_CHAMPION, PlayerId::One);
    let champion_id = champion.card.id;
    game.battlefield.push(champion);
    let thrull = creature(10_001, cards::BASAL_THRULL, PlayerId::Two);
    let thrull_id = thrull.card.id;
    game.battlefield.push(thrull);

    steal(&mut game, champion_id, thrull_id);

    assert_eq!(controller(&game, thrull_id), PlayerId::One);
    let stolen = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == thrull_id)
        .expect("still there");
    assert_eq!(
        game.power(stolen),
        Some(2),
        "a 1/1 Thrull under the Champion's anthem"
    );
}

#[test]
fn both_identities_report_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [cards::ALADDIN, cards::THRULL_CHAMPION] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
