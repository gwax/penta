//! Blocking restrictions that compare power against the attacker's.
//!
//! "Creatures with power less than this creature's power can't block it" is
//! read live against the source, so pumping the attacker widens the
//! restriction mid-combat rather than being fixed when it attacked.

use super::*;
use crate::ImplementationStatus;

/// Wandering Wolf attacking, with one prospective blocker of `power`.
fn blocking_board(blocker: CardDefinitionId) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    let wolf = creature(10_000, cards::WANDERING_WOLF, PlayerId::One);
    let wolf_id = wolf.card.id;
    game.battlefield.push(wolf);
    let defender = creature(10_001, blocker, PlayerId::Two);
    let defender_id = defender.card.id;
    game.battlefield.push(defender);

    game.step = Step::DeclareBlockers;
    game.attackers_declared = true;
    game.active_player = PlayerId::One;
    game.priority = PlayerId::Two;
    let wolf = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == wolf_id)
        .expect("just pushed");
    wolf.attacking = true;
    wolf.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    (game, wolf_id, defender_id)
}

fn can_block(game: &Game, blocker: GameObjectId) -> bool {
    game.legal_actions(PlayerId::Two).iter().any(
        |action| matches!(action, Action::DeclareBlocker { blocker: actual, .. } if *actual == blocker),
    )
}

#[test]
fn a_weaker_creature_cannot_block_it() {
    // Wandering Wolf is a 2/1. Savannah Lions is 2/1 as well, so it is not
    // weaker and blocks fine.
    let (game, _, lions) = blocking_board(cards::SAVANNAH_LIONS);
    assert!(
        can_block(&game, lions),
        "equal power is not less than, so it may block"
    );

    // Icatian Moneychanger is a 0/2, which is weaker.
    let (game, _, changer) = blocking_board(cards::ICATIAN_MONEYCHANGER);
    assert!(!can_block(&game, changer), "a 0/2 has power less than two");
}

/// The comparison is against current power, so a pump changes the answer.
#[test]
fn pumping_the_attacker_widens_the_restriction() {
    let (mut game, wolf_id, lions) = blocking_board(cards::SAVANNAH_LIONS);
    assert!(can_block(&game, lions));

    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == wolf_id)
        .expect("there")
        .power_bonus += 1;

    assert!(
        !can_block(&game, lions),
        "a 3/1 attacker now outclasses the 2/1 blocker"
    );
}

#[test]
fn every_power_blocking_identity_reports_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [cards::HOWLGEIST, cards::WANDERING_WOLF] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
