//! Two spells that eat a creature on the way to the stack.
//!
//! The sacrifice is an additional cost, so it is paid as the spell is cast
//! rather than as it resolves: with no creature out there is nothing to cast,
//! and the creature is already gone by the time the spell does its work.

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
    game.players[PlayerId::One.index()].hand.clear();
    game
}

fn casts(game: &Game, spell: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
        .collect()
}

/// No creature, no cast: the additional cost has to be payable.
#[test]
fn bone_splinters_needs_a_creature_to_eat() {
    let mut game = ready();
    let spell = card(20_000, cards::BONE_SPLINTERS, PlayerId::One);
    let spell_id = spell.id;
    game.players[PlayerId::One.index()].hand.push(spell);
    game.players[PlayerId::One.index()].mana_pool.black = 1;

    let victim = creature(10_100, cards::AIR_ELEMENTAL, PlayerId::Two);
    let victim_id = victim.card.id;
    game.battlefield.push(victim);
    assert!(
        casts(&game, spell_id).is_empty(),
        "a target but nothing of yours to sacrifice",
    );

    game.battlefield
        .push(creature(10_000, cards::GRIZZLY_BEARS, PlayerId::One));
    assert!(!casts(&game, spell_id).is_empty(), "now it can be paid");

    let action = casts(&game, spell_id)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { sacrifices, choices, .. }
                if sacrifices.contains(&GameObjectId(10_000))
                    && choices.targets().iter().flat_map(crate::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(victim_id)))
        })
        .expect("sacrifice the Bear, destroy the Elemental");
    game.apply(PlayerId::One, action)
        .expect("the cast is legal");
    drain_pending(&mut game);

    assert!(
        game.battlefield.is_empty(),
        "the Bear paid on the way up and the Elemental died on the way down",
    );
}

/// The Plunge is a sorcery, so its three red arrive as it resolves.
#[test]
fn infernal_plunge_trades_a_creature_for_three_red() {
    let mut game = ready();
    game.battlefield
        .push(creature(10_000, cards::GRIZZLY_BEARS, PlayerId::One));
    let spell = card(20_000, cards::INFERNAL_PLUNGE, PlayerId::One);
    let spell_id = spell.id;
    game.players[PlayerId::One.index()].hand.push(spell);
    game.players[PlayerId::One.index()].mana_pool.red = 1;

    let action = casts(&game, spell_id)
        .into_iter()
        .next()
        .expect("one creature, one way to pay");
    game.apply(PlayerId::One, action)
        .expect("the cast is legal");
    drain_pending(&mut game);

    assert!(game.battlefield.is_empty(), "the Bear was the cost");
    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.red,
        3,
        "and three red came back",
    );
}

#[test]
fn both_cards_report_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [cards::BONE_SPLINTERS, cards::INFERNAL_PLUNGE] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
