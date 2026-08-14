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
