//! Auras that trigger on their host dying.
//!
//! The amount is the dead creature's power, which is only knowable from last
//! known information: by the time the trigger resolves the creature is in a
//! graveyard. What these check is that the number is read from the creature
//! as it last was, pumps included, rather than from its printed stats.

use super::*;
use crate::ImplementationStatus;

/// An Aura on a creature, both controlled by player one.
fn enchanted(aura: CardDefinitionId, pump: i16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    let mut host = creature(10_000, cards::SEDGE_TROLL, PlayerId::One);
    host.power_bonus = pump;
    let host_id = host.card.id;
    game.battlefield.push(host);

    let mut enchantment = creature(10_001, aura, PlayerId::One);
    enchantment.attached_to = Some(host_id);
    game.battlefield.push(enchantment);
    game.check_state_based_actions();
    (game, host_id)
}

#[test]
fn murder_investigation_makes_one_soldier_per_power() {
    // Sedge Troll is a 2/2, pumped to a 4/2.
    let (mut game, host) = enchanted(cards::MURDER_INVESTIGATION, 2);
    game.destroy_permanent(host);
    drain_pending(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == cards::SOLDIER_TOKEN_1_1_WHITE)
            .count(),
        4,
        "four Soldiers for a creature that died as a 4/2"
    );
}

/// The Aura is in the graveyard by the time this resolves too, so the trigger
/// has to survive its own source leaving.
#[test]
fn the_aura_itself_is_gone_when_the_trigger_resolves() {
    let (mut game, host) = enchanted(cards::MURDER_INVESTIGATION, 0);
    game.destroy_permanent(host);
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::MURDER_INVESTIGATION),
        "the Aura fell off and died with its host"
    );
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == cards::SOLDIER_TOKEN_1_1_WHITE)
            .count(),
        2,
        "and its trigger still made two Soldiers"
    );
}

#[test]
fn murder_investigation_reports_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    let card = catalog
        .get(cards::MURDER_INVESTIGATION)
        .expect("the card is cataloged");
    assert_eq!(
        card.rules.implementation_status(),
        ImplementationStatus::Complete,
        "{} should be fully executable",
        card.name,
    );
}
