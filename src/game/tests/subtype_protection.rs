//! Protection from a creature type rather than from a color.
//!
//! Protection is one keyword per quality, and until now every quality was a
//! color. These tests pin the two halves that are easiest to get wrong --
//! damage from a matching source, and being blocked by one -- along with the
//! fact that naming Zombies leaves everything else unbothered.

use super::*;
use crate::ImplementationStatus;

fn ready() -> Game {
    let mut game = ready_game();
    game.turn = 5;
    game.turns_started[PlayerId::One.index()] = 5;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.battlefield.clear();
    game
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("still there")
}

/// A Zombie's damage does not land on the Bramble.
#[test]
fn a_zombie_cannot_damage_what_is_protected_from_zombies() {
    let mut game = ready();
    let bramble = creature(10_000, cards::GRAVE_BRAMBLE, PlayerId::One);
    let bramble_id = bramble.card.id;
    game.battlefield.push(bramble);
    let zombie = creature(10_100, cards::ZOMBIE_TOKEN_2_2_BLACK, PlayerId::Two);
    let zombie_id = zombie.card.id;
    game.battlefield.push(zombie);
    let bear = creature(10_101, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bear_id = bear.card.id;
    game.battlefield.push(bear);

    game.damage_target_from(Some(zombie_id), Some(Target::Permanent(bramble_id)), 2);
    assert_eq!(permanent(&game, bramble_id).damage, 0, "protection held");

    game.damage_target_from(Some(bear_id), Some(Target::Permanent(bramble_id)), 2);
    assert_eq!(
        permanent(&game, bramble_id).damage,
        2,
        "a Bear is not a Zombie",
    );
}

/// CR 702.16e: a Zombie cannot block the protected creature. The other
/// direction is deliberately not protection -- the Inquisitor may block a
/// Zombie perfectly well.
#[test]
fn a_zombie_cannot_block_a_creature_protected_from_zombies() {
    let mut game = ready();
    let mut inquisitor = creature(10_000, cards::ELITE_INQUISITOR, PlayerId::One);
    inquisitor.attacking = true;
    let inquisitor_id = inquisitor.card.id;
    game.battlefield.push(inquisitor);
    let mut bear = creature(10_001, cards::GRIZZLY_BEARS, PlayerId::One);
    bear.attacking = true;
    let bear_id = bear.card.id;
    game.battlefield.push(bear);
    let zombie = creature(10_100, cards::ZOMBIE_TOKEN_2_2_BLACK, PlayerId::Two);
    let zombie_id = zombie.card.id;
    game.battlefield.push(zombie);
    let wall = creature(10_101, cards::WALL_OF_STONE, PlayerId::Two);
    let wall_id = wall.card.id;
    game.battlefield.push(wall);

    game.step = Step::DeclareBlockers;
    game.attackers_declared = true;
    game.priority = PlayerId::Two;

    let offered = |game: &Game, blocker: GameObjectId, attacker: GameObjectId| {
        game.legal_actions(PlayerId::Two)
            .contains(&Action::DeclareBlocker { blocker, attacker })
    };
    assert!(
        !offered(&game, zombie_id, inquisitor_id),
        "a Zombie cannot block what is protected from Zombies",
    );
    assert!(
        offered(&game, zombie_id, bear_id),
        "the Bear it may block, having no protection at all",
    );
    assert!(
        offered(&game, wall_id, inquisitor_id),
        "and anything that is not a Zombie may block the Inquisitor",
    );
}

/// And a spell cast by nobody in particular still gets through -- protection
/// reads the source's own types, not its controller's.
#[test]
fn a_vampires_protection_ignores_zombies_entirely() {
    let mut game = ready();
    let duelist = creature(10_000, cards::MIDNIGHT_DUELIST, PlayerId::One);
    let duelist_id = duelist.card.id;
    game.battlefield.push(duelist);
    let zombie = creature(10_100, cards::ZOMBIE_TOKEN_2_2_BLACK, PlayerId::Two);
    let zombie_id = zombie.card.id;
    game.battlefield.push(zombie);

    game.damage_target_from(Some(zombie_id), Some(Target::Permanent(duelist_id)), 1);
    assert_eq!(
        permanent(&game, duelist_id).damage,
        1,
        "the Duelist names Vampires, so a Zombie is nothing special",
    );
}

/// The Inquisitor names three types in one printed clause, which is three
/// instances -- each has to work on its own.
#[test]
fn the_inquisitor_carries_all_three_instances() {
    let mut game = ready();
    let inquisitor = creature(10_000, cards::ELITE_INQUISITOR, PlayerId::One);
    let inquisitor_id = inquisitor.card.id;
    game.battlefield.push(inquisitor);

    for creature_type in [
        ProtectedCreatureType::Vampire,
        ProtectedCreatureType::Werewolf,
        ProtectedCreatureType::Zombie,
    ] {
        assert!(
            game.permanent_has_executable_keyword(
                permanent(&game, inquisitor_id),
                KeywordAbility::ProtectionFromCreatureType(creature_type),
            ),
            "{} should be among the qualities",
            creature_type.subtype(),
        );
    }

    let zombie = creature(10_100, cards::ZOMBIE_TOKEN_2_2_BLACK, PlayerId::Two);
    let zombie_id = zombie.card.id;
    game.battlefield.push(zombie);
    game.damage_target_from(Some(zombie_id), Some(Target::Permanent(inquisitor_id)), 2);
    assert_eq!(permanent(&game, inquisitor_id).damage, 0);
}

#[test]
fn all_three_report_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [
        cards::ELITE_INQUISITOR,
        cards::GRAVE_BRAMBLE,
        cards::MIDNIGHT_DUELIST,
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
