//! Prevention that names the damage's source by its relationship.
//!
//! "By creatures it's blocking" is read from the Wall, not from the
//! attacker: the attacker's own record does not name who is blocking it. And
//! "by enchanted creatures" is read off the battlefield, so an Aura arriving
//! or leaving changes the answer without the Wall being touched.

use super::*;
use crate::ImplementationStatus;

/// A big attacker for player one, blocked by `wall` for player two.
fn blocked_by(wall: CardDefinitionId) -> (Game, GameObjectId, GameObjectId) {
    blocked_by_attacker(cards::SERRA_ANGEL, wall)
}

fn blocked_by_attacker(
    attacker: CardDefinitionId,
    wall: CardDefinitionId,
) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    let mut attacker = creature(10_000, attacker, PlayerId::One);
    attacker.attacking = true;
    attacker.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    let attacker_id = attacker.card.id;
    game.battlefield.push(attacker);

    let mut blocker = creature(10_001, wall, PlayerId::Two);
    blocker.blocking = Some(attacker_id);
    let blocker_id = blocker.card.id;
    game.battlefield.push(blocker);
    (game, attacker_id, blocker_id)
}

fn damage_on(game: &Game, permanent: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|candidate| candidate.card.id == permanent)
        .map_or(0, |candidate| candidate.damage)
}

fn survives(game: &Game, permanent: GameObjectId) -> bool {
    game.battlefield
        .iter()
        .any(|candidate| candidate.card.id == permanent)
}

#[test]
fn a_wall_takes_nothing_from_what_it_blocks() {
    let (mut game, _, wall) = blocked_by(cards::WALL_OF_VAPOR);
    game.deal_combat_damage();

    assert!(survives(&game, wall), "a 0/1 survives a 4/4 it blocked");
    assert_eq!(damage_on(&game, wall), 0);
}

/// The prevention names one relationship, not a blanket shield: damage from
/// something the Wall is not blocking still lands.
#[test]
fn damage_from_elsewhere_still_lands() {
    let (mut game, _, wall) = blocked_by(cards::WALL_OF_VAPOR);
    let other = creature(10_002, cards::SAVANNAH_LIONS, PlayerId::One);
    let other_id = other.card.id;
    game.battlefield.push(other);

    game.damage_target_from(Some(other_id), Some(Target::Permanent(wall)), 1);
    game.check_state_based_actions();

    assert!(
        !survives(&game, wall),
        "a creature it never blocked killed it"
    );
}

/// An ordinary blocker is not covered, which is what shows the effect is the
/// Wall's own rather than something about blocking.
#[test]
fn an_ordinary_blocker_takes_the_damage() {
    let (mut game, _, blocker) = blocked_by(cards::SAVANNAH_LIONS);
    game.deal_combat_damage();

    assert!(!survives(&game, blocker), "a 2/1 dies to a 4/4");
}

/// Wall of Putrid Flesh reads the battlefield: the same attacker is
/// prevented or not depending on whether an Aura is attached to it.
#[test]
fn an_aura_on_the_attacker_is_what_turns_the_prevention_on() {
    // A red attacker: the Wall's protection from white would answer a white
    // one before the prevention ever came up.
    let (mut game, attacker, wall) =
        blocked_by_attacker(cards::SHIVAN_DRAGON, cards::WALL_OF_PUTRID_FLESH);
    let mut aura = creature(10_002, cards::UNHOLY_STRENGTH, PlayerId::One);
    aura.attached_to = Some(attacker);
    game.battlefield.push(aura);
    game.check_state_based_actions();

    game.deal_combat_damage();
    assert!(
        survives(&game, wall),
        "an enchanted attacker cannot hurt it"
    );
    assert_eq!(damage_on(&game, wall), 0);

    let (mut game, _, wall) =
        blocked_by_attacker(cards::SHIVAN_DRAGON, cards::WALL_OF_PUTRID_FLESH);
    game.deal_combat_damage();
    assert!(
        !survives(&game, wall),
        "and an unenchanted one kills the 2/4"
    );
}

#[test]
fn every_wall_identity_reports_its_audited_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for (definition, expected) in [
        (cards::WALL_OF_VAPOR, ImplementationStatus::Complete),
        (cards::WALL_OF_PUTRID_FLESH, ImplementationStatus::Complete),
        (cards::WALL_OF_SHADOWS, ImplementationStatus::Partial),
    ] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            expected,
            "{} reports the coverage its audit line claims",
            card.name,
        );
    }
}
