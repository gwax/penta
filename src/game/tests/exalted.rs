//! Exalted.
//!
//! It reads as a keyword and is defined as a triggered ability, which matters
//! twice: several instances each trigger, and the permanent carrying it need
//! not be a creature. The condition is decided by the attack declaration as a
//! whole, so these drive it through a real declaration rather than by setting
//! flags.

use super::*;
use crate::ImplementationStatus;

fn attack_with(game: &mut Game, attackers: &[GameObjectId]) {
    game.step = Step::DeclareAttackers;
    for attacker in attackers {
        game.declare_attacker(*attacker, AttackDefender::Player(PlayerId::Two));
    }
    game.finish_declaring_attackers();
    drain_pending(game);
}

fn power_of(game: &Game, id: GameObjectId) -> i16 {
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("the creature is on the battlefield");
    game.power(permanent).expect("it is a creature")
}

fn board(extra: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.turns_started[PlayerId::One.index()] = 1;
    let attacker = creature(10_000, cards::SAVANNAH_LIONS, PlayerId::One);
    let attacker_id = attacker.card.id;
    game.battlefield.push(attacker);
    for (index, definition) in extra.iter().enumerate() {
        game.battlefield.push(creature(
            10_100 + u32::try_from(index).expect("few permanents"),
            *definition,
            PlayerId::One,
        ));
    }
    (game, attacker_id)
}

#[test]
fn one_attacker_alone_is_pumped() {
    let (mut game, attacker_id) = board(&[cards::SERVANT_OF_NEFAROX]);
    assert_eq!(power_of(&game, attacker_id), 2);

    attack_with(&mut game, &[attacker_id]);

    assert_eq!(
        power_of(&game, attacker_id),
        3,
        "attacking alone is the trigger"
    );
}

/// Two attackers is not one, and the whole point of the keyword is that it
/// does nothing then.
#[test]
fn two_attackers_get_nothing() {
    let (mut game, attacker_id) = board(&[cards::SERVANT_OF_NEFAROX]);
    let second = creature(10_001, cards::SAVANNAH_LIONS, PlayerId::One);
    let second_id = second.card.id;
    game.battlefield.push(second);

    attack_with(&mut game, &[attacker_id, second_id]);

    assert_eq!(power_of(&game, attacker_id), 2);
    assert_eq!(power_of(&game, second_id), 2);
}

/// Each instance is its own triggered ability, so two of them stack.
#[test]
fn two_instances_each_trigger() {
    let (mut game, attacker_id) = board(&[cards::SERVANT_OF_NEFAROX, cards::AVEN_SQUIRE]);

    attack_with(&mut game, &[attacker_id]);

    assert_eq!(power_of(&game, attacker_id), 4, "+1/+1 twice");
}

/// Cathedral of War is a land, which is why exalted is a trigger on any
/// permanent rather than a creature keyword.
#[test]
fn a_land_can_carry_exalted() {
    let (mut game, attacker_id) = board(&[cards::CATHEDRAL_OF_WAR]);

    attack_with(&mut game, &[attacker_id]);

    assert_eq!(power_of(&game, attacker_id), 3);
}

/// The exalted creature can be the lone attacker itself.
#[test]
fn the_exalted_creature_pumps_itself_when_it_attacks_alone() {
    let mut game = ready_game();
    game.turns_started[PlayerId::One.index()] = 1;
    let squire = creature(10_000, cards::AVEN_SQUIRE, PlayerId::One);
    let squire_id = squire.card.id;
    game.battlefield.push(squire);

    attack_with(&mut game, &[squire_id]);

    assert_eq!(power_of(&game, squire_id), 2);
}

#[test]
fn every_exalted_identity_reports_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [
        cards::ANGELIC_BENEDICTION,
        cards::AVEN_SQUIRE,
        cards::GUARDIANS_OF_AKRASA,
        cards::DUSKMANTLE_PROWLER,
        cards::SERVANT_OF_NEFAROX,
        cards::CATHEDRAL_OF_WAR,
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
