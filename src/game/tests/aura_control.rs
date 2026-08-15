//! Auras that take the permanent they are on.
//!
//! The printed clause is a static, and the engine's control change is a
//! resolving effect that lasts while its source remains. For an Aura those
//! come to the same thing: an Aura with nothing under it is put into its
//! owner's graveyard, so "while the Aura remains" is "while it is attached".

use super::*;
use crate::ImplementationStatus;

/// Player two owns `host`; player one casts `aura` onto it.
fn stolen(aura: CardDefinitionId, host: CardDefinitionId) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.turns_started[PlayerId::One.index()] = 5;
    let target = creature(10_000, host, PlayerId::Two);
    let target_id = target.card.id;
    game.battlefield.push(target);

    let spell = card(20_000, aura, PlayerId::One);
    let spell_id = spell.id;
    game.players[PlayerId::One.index()].hand.push(spell);
    game.players[PlayerId::One.index()].mana_pool.blue = 4;
    game.players[PlayerId::One.index()].mana_pool.colorless = 4;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell_id))
        .expect("the Aura is castable onto it");
    game.apply(PlayerId::One, action)
        .expect("the cast is legal");
    drain_pending(&mut game);
    // Resolving gives the Aura a battlefield object of its own, which is the
    // one the control effect is scoped to.
    let aura_id = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == aura)
        .expect("the Aura resolved onto the battlefield")
        .card
        .id;
    (game, target_id, aura_id)
}

fn controller(game: &Game, id: GameObjectId) -> Option<PlayerId> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .map(|permanent| permanent.controller)
}

#[test]
fn control_magic_takes_the_creature() {
    let (game, creature_id, _aura) = stolen(cards::CONTROL_MAGIC, cards::SERRA_ANGEL);

    assert_eq!(controller(&game, creature_id), Some(PlayerId::One));
}

/// The control is scoped to the Aura: destroy it and the creature goes home.
#[test]
fn destroying_control_magic_hands_the_creature_back() {
    let (mut game, creature_id, aura_id) = stolen(cards::CONTROL_MAGIC, cards::SERRA_ANGEL);
    assert_eq!(controller(&game, creature_id), Some(PlayerId::One));

    game.destroy_permanent(aura_id);
    drain_pending(&mut game);
    // Reverting control is a state-based action, checked the next time the
    // game looks rather than as the Aura leaves.
    game.check_state_based_actions();
    drain_pending(&mut game);

    assert_eq!(
        controller(&game, creature_id),
        Some(PlayerId::Two),
        "the Aura is what was holding it"
    );
}

#[test]
fn steal_artifact_takes_the_artifact() {
    let (game, artifact_id, _aura) = stolen(cards::STEAL_ARTIFACT, cards::JUGGERNAUT);

    assert_eq!(controller(&game, artifact_id), Some(PlayerId::One));
}

#[test]
fn both_auras_report_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [cards::CONTROL_MAGIC, cards::STEAL_ARTIFACT] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
