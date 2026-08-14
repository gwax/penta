//! Fog: all combat damage this turn is prevented.
//!
//! The engine could already prevent combat damage per permanent, which is
//! enough for a Maze of Ith but not for a Fog: the Fog has no permanent to
//! attach to, and it has to cover creatures that were not on the battlefield
//! when it resolved. So the shield is game state and lives until cleanup.

use super::*;

fn fogged_combat(cast_fog: bool) -> Game {
    let mut game = ready_game();
    game.step = Step::DeclareBlockers;
    let mut attacker = creature(10_000, cards::SEA_SERPENT, PlayerId::One);
    attacker.attacking = true;
    attacker.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    game.battlefield.push(attacker);
    let mut blocker = creature(10_001, cards::SAVANNAH_LIONS, PlayerId::Two);
    blocker.blocking = Some(GameObjectId(10_000));
    game.battlefield.push(blocker);
    if cast_fog {
        game.all_combat_damage_prevented = true;
    }
    game
}

fn resolve_combat_damage(game: &mut Game) {
    game.finish_declaring_blockers();
    game.start_combat_damage();
    game.finish_rules_procedure();
}

#[test]
fn combat_damage_lands_without_a_fog() {
    let mut game = fogged_combat(false);
    resolve_combat_damage(&mut game);
    let serpent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == GameObjectId(10_000));
    assert!(
        serpent.is_some_and(|permanent| permanent.damage > 0),
        "the blocker's damage is marked"
    );
}

#[test]
fn a_fog_prevents_damage_in_both_directions() {
    let mut game = fogged_combat(true);
    resolve_combat_damage(&mut game);
    for id in [GameObjectId(10_000), GameObjectId(10_001)] {
        let permanent = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == id)
            .expect("both combatants survive a Fog");
        assert_eq!(permanent.damage, 0, "{id:?} took no combat damage");
    }
}

/// The shield covers what the attacker would have dealt to the player too.
#[test]
fn a_fog_prevents_damage_to_the_defending_player() {
    let mut game = ready_game();
    game.step = Step::DeclareBlockers;
    let mut attacker = creature(10_000, cards::SEA_SERPENT, PlayerId::One);
    attacker.attacking = true;
    attacker.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    game.battlefield.push(attacker);
    game.all_combat_damage_prevented = true;
    let before = game.players[PlayerId::Two.index()].life;

    resolve_combat_damage(&mut game);

    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        before,
        "an unblocked attacker deals nothing through a Fog"
    );
}

/// It is a turn-scoped shield, not a permanent one.
#[test]
fn a_fog_does_not_survive_cleanup() {
    let mut game = fogged_combat(true);
    game.finish_cleanup();
    assert!(
        !game.all_combat_damage_prevented,
        "the shield expires with the turn"
    );
}

#[test]
fn every_newly_unblocked_fog_reports_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [cards::FOG, cards::HOLY_DAY, cards::DARKNESS] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            crate::ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}

fn shielded_creature(game: &mut Game) -> GameObjectId {
    let creature = creature(20_000, cards::SAVANNAH_LIONS, PlayerId::One);
    let id = creature.card.id;
    game.battlefield.push(creature);
    id
}

/// A shield waits for damage rather than acting now, and is spent by the
/// damage it covers.
#[test]
fn a_shield_absorbs_up_to_its_amount_and_is_then_gone() {
    let mut game = ready_game();
    let target = shielded_creature(&mut game);
    game.prevention_shields.push(PreventionShield {
        recipient: Target::Permanent(target),
        remaining: Some(2),
    });

    game.damage_target(Some(Target::Permanent(target)), 1);
    let marked = |game: &Game| {
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == target)
            .map_or(0, |permanent| permanent.damage)
    };
    assert_eq!(marked(&game), 0, "the first point is prevented");

    game.damage_target(Some(Target::Permanent(target)), 3);
    assert_eq!(
        marked(&game),
        2,
        "one point of the shield was left, so two of the three land"
    );
    assert!(game.prevention_shields.is_empty(), "a spent shield is gone");
}

/// "Prevent all damage" is never spent, so it holds for the whole turn.
#[test]
fn a_prevent_all_shield_is_not_consumed() {
    let mut game = ready_game();
    let target = shielded_creature(&mut game);
    game.prevention_shields.push(PreventionShield {
        recipient: Target::Permanent(target),
        remaining: None,
    });

    for _ in 0..3 {
        game.damage_target(Some(Target::Permanent(target)), 5);
    }
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == target)
        .expect("the creature survives");
    assert_eq!(permanent.damage, 0, "every point was prevented");
    assert_eq!(game.prevention_shields.len(), 1, "the shield still holds");
}

#[test]
fn a_shield_only_covers_the_recipient_it_names() {
    let mut game = ready_game();
    let shielded = shielded_creature(&mut game);
    let other = creature(20_001, cards::SAVANNAH_LIONS, PlayerId::Two);
    let other_id = other.card.id;
    game.battlefield.push(other);
    game.prevention_shields.push(PreventionShield {
        recipient: Target::Permanent(shielded),
        remaining: Some(5),
    });

    game.damage_target(Some(Target::Permanent(other_id)), 1);
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == other_id)
        .expect("the other creature is on the battlefield");
    assert_eq!(permanent.damage, 1, "an unshielded creature takes damage");
}

/// Shields cover players too, which is what "any target" means.
#[test]
fn a_shield_can_cover_a_player() {
    let mut game = ready_game();
    let before = game.players[PlayerId::Two.index()].life;
    game.prevention_shields.push(PreventionShield {
        recipient: Target::Player(PlayerId::Two),
        remaining: Some(3),
    });

    game.damage_target(Some(Target::Player(PlayerId::Two)), 2);
    assert_eq!(game.players[PlayerId::Two.index()].life, before);
}

#[test]
fn shields_do_not_survive_cleanup() {
    let mut game = ready_game();
    let target = shielded_creature(&mut game);
    game.prevention_shields.push(PreventionShield {
        recipient: Target::Permanent(target),
        remaining: None,
    });
    game.finish_cleanup();
    assert!(game.prevention_shields.is_empty());
}

#[test]
fn every_newly_unblocked_prevention_card_reports_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [
        cards::SAMITE_HEALER,
        cards::INDESTRUCTIBLE_AURA,
        cards::AMULET_OF_KROOG,
    ] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            crate::ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
