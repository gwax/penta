//! Two statics whose answer is a battlefield count.
//!
//! Both were audited as blocked on the counting, and both now read it live:
//! the Armor's bonus grows as enchantments arrive, and the Jailbreaker's
//! permission comes and goes with the Gate.

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

fn stats(game: &Game, id: GameObjectId) -> (Option<i16>, Option<i16>) {
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("still there");
    (game.power(permanent), game.toughness(permanent))
}

/// The Armor counts itself, and every enchantment added after it.
#[test]
fn the_armor_recounts_your_enchantments() {
    let mut game = ready();
    let bear = creature(10_000, cards::GRIZZLY_BEARS, PlayerId::One);
    let bear_id = bear.card.id;
    game.battlefield.push(bear);
    let mut armor = creature(10_001, cards::ETHEREAL_ARMOR, PlayerId::One);
    armor.attached_to = Some(bear_id);
    game.battlefield.push(armor);

    assert_eq!(
        stats(&game, bear_id),
        (Some(3), Some(3)),
        "the Armor counts itself",
    );
    assert!(
        game.permanent_has_executable_keyword(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == bear_id)
                .expect("still there"),
            KeywordAbility::FirstStrike,
        ),
        "and the first strike is unconditional",
    );

    game.battlefield
        .push(creature(10_002, cards::INTANGIBLE_VIRTUE, PlayerId::One));
    assert_eq!(stats(&game, bear_id), (Some(4), Some(4)), "two now");

    // An opponent's enchantment is not yours.
    game.battlefield
        .push(creature(10_100, cards::INTANGIBLE_VIRTUE, PlayerId::Two));
    assert_eq!(stats(&game, bear_id), (Some(4), Some(4)), "still two");
}

/// The Jailbreaker keeps defender and gains permission only while a Gate is
/// out.
#[test]
fn the_jailbreaker_needs_a_gate_to_attack() {
    let mut game = ready();
    let ogre = creature(10_000, cards::OGRE_JAILBREAKER, PlayerId::One);
    let ogre_id = ogre.card.id;
    game.battlefield.push(ogre);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.step = Step::DeclareAttackers;

    let can_attack = |game: &Game| {
        game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::DeclareAttacker { attacker, .. } if *attacker == ogre_id),
        )
    };
    assert!(!can_attack(&game), "defender, and no Gate");

    game.put_onto_battlefield(PlayerId::One, cards::BOROS_GUILDGATE)
        .expect("cataloged");
    assert!(can_attack(&game), "a Gate unlocks it");

    assert!(
        game.permanent_has_executable_keyword(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == ogre_id)
                .expect("still there"),
            KeywordAbility::Defender,
        ),
        "it is a permission, not an ability removal",
    );

    let guildgate = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::BOROS_GUILDGATE)
        .expect("it is there")
        .card
        .id;
    game.battlefield
        .retain(|permanent| permanent.card.id != guildgate);
    assert!(!can_attack(&game), "and it locks again when the Gate goes");
}

#[test]
fn both_cards_report_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [cards::ETHEREAL_ARMOR, cards::OGRE_JAILBREAKER] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
