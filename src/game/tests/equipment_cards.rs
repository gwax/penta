//! Five Equipment whose audit lines said the equip procedure was missing.
//!
//! It was not: equip, the attachment relation, and the Equipment host rules
//! were all built. What these pin is the equip activation itself -- sorcery
//! speed, your own creature -- and the two conditional bonuses, which read
//! the equipped creature's type live.

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

/// Puts the Equipment and a creature out, then equips.
fn equip_onto(
    equipment: crate::ids::CardDefinitionId,
    host: crate::ids::CardDefinitionId,
) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready();
    let gear = creature(10_000, equipment, PlayerId::One);
    let gear_id = gear.card.id;
    game.battlefield.push(gear);
    let creature_permanent = creature(10_100, host, PlayerId::One);
    let host_id = creature_permanent.card.id;
    game.battlefield.push(creature_permanent);
    game.players[PlayerId::One.index()].mana_pool.colorless = 4;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == gear_id),
        )
        .expect("equip is offered");
    game.apply(PlayerId::One, action).expect("legal");
    drain_pending(&mut game);
    (game, gear_id, host_id)
}

#[test]
fn riot_gear_and_kitesail_hand_out_their_printed_bonuses() {
    let (game, _, host) = equip_onto(cards::RIOT_GEAR, cards::GRIZZLY_BEARS);
    assert_eq!(stats(&game, host), (Some(3), Some(4)), "a 2/2 with +1/+2");

    let (game, _, host) = equip_onto(cards::KITESAIL, cards::GRIZZLY_BEARS);
    assert_eq!(stats(&game, host), (Some(3), Some(2)));
    assert!(
        game.permanent_has_executable_keyword(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == host)
                .expect("still there"),
            KeywordAbility::Flying,
        ),
        "and the flying comes with it",
    );
}

#[test]
fn the_hood_hands_out_intimidate() {
    let (game, _, host) = equip_onto(cards::EXECUTIONERS_HOOD, cards::GRIZZLY_BEARS);
    assert!(
        game.permanent_has_executable_keyword(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == host)
                .expect("still there"),
            KeywordAbility::Intimidate,
        )
    );
}

/// The Mattock's second +1/+1 reads the equipped creature's type, so it is
/// worth one more on a Human than on anything else.
#[test]
fn the_mattock_pays_a_human_twice() {
    let (game, _, human) = equip_onto(cards::HEAVY_MATTOCK, cards::ELITE_INQUISITOR);
    assert_eq!(stats(&game, human), (Some(4), Some(4)), "a 2/2 with +2/+2");

    let (game, _, other) = equip_onto(cards::HEAVY_MATTOCK, cards::GRIZZLY_BEARS);
    assert_eq!(stats(&game, other), (Some(3), Some(3)), "+1/+1 only");
}

/// The Bracers' size is unconditional and only the vigilance reads the type.
#[test]
fn the_bracers_split_their_two_clauses() {
    let vigilant = |game: &Game, id: GameObjectId| {
        game.permanent_has_executable_keyword(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == id)
                .expect("still there"),
            KeywordAbility::Vigilance,
        )
    };

    // A Human with no printed vigilance, so the grant is the only source.
    let (game, _, human) = equip_onto(cards::BLADED_BRACERS, cards::HUMAN_TOKEN_1_1_WHITE);
    assert_eq!(stats(&game, human), (Some(2), Some(2)));
    assert!(vigilant(&game, human), "a Human gets the vigilance");

    let (game, _, zombie) = equip_onto(cards::BLADED_BRACERS, cards::ZOMBIE_TOKEN_2_2_BLACK);
    assert_eq!(
        stats(&game, zombie),
        (Some(3), Some(3)),
        "the size is unconditional",
    );
    assert!(!vigilant(&game, zombie), "but the vigilance is not");
}

/// Equip is sorcery-speed and aims at your own creature.
#[test]
fn equip_is_restricted_to_your_own_creatures_at_sorcery_speed() {
    let mut game = ready();
    let gear = creature(10_000, cards::RIOT_GEAR, PlayerId::One);
    let gear_id = gear.card.id;
    game.battlefield.push(gear);
    let theirs = creature(10_100, cards::GRIZZLY_BEARS, PlayerId::Two);
    let theirs_id = theirs.card.id;
    game.battlefield.push(theirs);
    game.players[PlayerId::One.index()].mana_pool.colorless = 4;

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::ActivateAbility { source, targets, .. }
                if *source == gear_id
                    && targets.iter().flat_map(crate::TargetSelection::targets)
                        .any(|target| *target == Target::Permanent(theirs_id)))
        }),
        "equip names a creature you control",
    );

    game.battlefield
        .push(creature(10_101, cards::GRIZZLY_BEARS, PlayerId::One));
    assert!(
        game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == gear_id)
        ),
        "and one of yours is fine",
    );

    game.step = Step::DeclareBlockers;
    assert!(
        !game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == gear_id)
        ),
        "equip only as a sorcery",
    );
}

#[test]
fn all_five_report_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [
        cards::RIOT_GEAR,
        cards::KITESAIL,
        cards::EXECUTIONERS_HOOD,
        cards::HEAVY_MATTOCK,
        cards::BLADED_BRACERS,
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
