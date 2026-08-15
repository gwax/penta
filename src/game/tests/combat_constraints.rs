//! Combat triggers that change what a creature is afterwards.
//!
//! Three cards whose audit lines all named a missing combat constraint, and
//! none of which needed one: a blocker that stops being a Wall, a Dwarf that
//! grows against Orcs, and a Ram that takes the Wall down with it.

use super::*;
use crate::ImplementationStatus;

/// `attacker` attacking player two, blocked by `blocker`, with the blockers
/// declared through the real procedure so the triggers fire.
fn blocked_attack(
    attacker: CardDefinitionId,
    blocker: CardDefinitionId,
) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.turns_started[PlayerId::One.index()] = 5;
    game.active_player = PlayerId::One;
    let mut attacking = creature(10_000, attacker, PlayerId::One);
    attacking.attacking = true;
    attacking.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    let attacker_id = attacking.card.id;
    game.battlefield.push(attacking);
    let blocking = creature(10_001, blocker, PlayerId::Two);
    let blocker_id = blocking.card.id;
    game.battlefield.push(blocking);

    game.step = Step::DeclareBlockers;
    game.attackers_declared = true;
    game.declare_blocker(blocker_id, attacker_id);
    game.finish_declaring_blockers();
    drain_pending(&mut game);
    (game, attacker_id, blocker_id)
}

fn permanent(game: &Game, id: GameObjectId) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
}

#[test]
fn the_land_wurm_stops_being_a_wall_once_it_blocks() {
    let (game, _attacker, wurm) = blocked_attack(cards::SEDGE_TROLL, cards::ELDER_LAND_WURM);

    let wurm = permanent(&game, wurm).expect("still there");
    assert!(
        !game.permanent_has_executable_keyword(wurm, KeywordAbility::Defender),
        "blocking took defender off it"
    );
    assert!(
        game.permanent_has_executable_keyword(wurm, KeywordAbility::Trample),
        "and left the rest of its printed keywords alone"
    );
}

/// The control: a Wurm that has not blocked still cannot attack.
#[test]
fn the_land_wurm_keeps_defender_until_it_blocks() {
    let mut game = ready_game();
    let wurm = creature(10_000, cards::ELDER_LAND_WURM, PlayerId::One);
    let wurm_id = wurm.card.id;
    game.battlefield.push(wurm);

    let wurm = permanent(&game, wurm_id).expect("still there");
    assert!(game.permanent_has_executable_keyword(wurm, KeywordAbility::Defender));
}

#[test]
fn the_dwarven_soldier_grows_against_an_orc() {
    let (game, _attacker, soldier) =
        blocked_attack(cards::ORCISH_ARTILLERY, cards::DWARVEN_SOLDIER);

    let soldier = permanent(&game, soldier).expect("still there");
    assert_eq!(
        game.toughness(soldier),
        Some(3),
        "a 2/1 that blocked an Orc is a 2/3"
    );
}

/// The control, and the point of the subtype in the trigger: anything that is
/// not an Orc leaves it alone.
#[test]
fn the_dwarven_soldier_ignores_everything_else() {
    let (game, _attacker, soldier) = blocked_attack(cards::SEDGE_TROLL, cards::DWARVEN_SOLDIER);

    let soldier = permanent(&game, soldier).expect("still there");
    assert_eq!(game.toughness(soldier), Some(1), "a Troll is not an Orc");
}

#[test]
fn the_battering_ram_marks_the_wall_that_blocked_it() {
    let (game, _ram, wall) = blocked_attack(cards::BATTERING_RAM, cards::WALL_OF_STONE);

    let wall = permanent(&game, wall).expect("still there for now");
    assert!(
        wall.destroy_at_end_of_combat,
        "the Wall is marked, and dies when combat ends"
    );
}

/// The control: an ordinary blocker is not a Wall, so nothing is marked.
#[test]
fn the_battering_ram_leaves_a_non_wall_blocker_alone() {
    let (game, _ram, blocker) = blocked_attack(cards::BATTERING_RAM, cards::SEDGE_TROLL);

    let blocker = permanent(&game, blocker).expect("still there");
    assert!(!blocker.destroy_at_end_of_combat);
}

#[test]
fn the_three_identities_report_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [
        cards::ELDER_LAND_WURM,
        cards::DWARVEN_SOLDIER,
        cards::BATTERING_RAM,
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
