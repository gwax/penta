use super::tests::{creature, ready_game};
use super::*;
use crate::card::cards;

fn pass_priority_pair(game: &mut Game) {
    for _ in 0..2 {
        let player = game.priority;
        game.apply(player, Action::PassPriority).unwrap();
    }
}

#[test]
fn domri_ultimate_grants_two_combat_damage_steps() {
    let mut game = ready_game();
    let mut domri = creature(10_000, cards::DOMRI_RADE, PlayerId::One);
    domri.set_counters(CounterKind::Loyalty, 7);
    let domri_id = domri.card.id;
    let attacker = creature(10_001, cards::SAVANNAH_LIONS, PlayerId::One);
    let attacker_id = attacker.card.id;
    game.battlefield = vec![domri, attacker];

    let ultimate = Action::ActivateAbility {
        source: domri_id,
        ability: AbilityOrigin::Printed {
            definition: cards::DOMRI_RADE,
            part: CardPartId::PRIMARY,
            ability: AbilityId(2),
        },
        targets: Vec::new(),
        sacrifice: None,
        x: 0,
    };
    assert!(game.legal_actions(PlayerId::One).contains(&ultimate));
    game.apply(PlayerId::One, ultimate).unwrap();
    pass_priority_pair(&mut game);

    assert_eq!(game.emblems.len(), 1);
    let attacker = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == attacker_id)
        .unwrap();
    attacker.attacking = true;
    attacker.attack_defender = Some(AttackDefender::Player(PlayerId::Two));

    game.step = Step::DeclareBlockers;
    game.advance_step();

    assert_eq!(game.step, Step::CombatDamage);
    assert!(game.regular_combat_damage_pending());
    assert_eq!(game.players[PlayerId::Two.index()].life, 18);

    pass_priority_pair(&mut game);

    assert_eq!(game.step, Step::CombatDamage);
    assert!(!game.regular_combat_damage_pending());
    assert_eq!(game.players[PlayerId::Two.index()].life, 16);
}
