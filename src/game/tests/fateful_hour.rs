//! Fateful hour: a clause that only applies at five life or less.
//!
//! The two cards read the threshold differently, and the difference matters.
//! Break of Day checks once as it resolves, so what it grants survives a
//! later life gain; Gavony Ironwright's "as long as" is continuous, so the
//! anthem switches off the moment life climbs back above five.

use super::*;
use crate::ImplementationStatus;

fn ready(life: i16) -> Game {
    let mut game = ready_game();
    game.turn = 5;
    game.turns_started[PlayerId::One.index()] = 5;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.players[PlayerId::One.index()].life = life;
    game.priority = PlayerId::One;
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

fn indestructible(game: &Game, id: GameObjectId) -> bool {
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("still there");
    game.permanent_has_executable_keyword(permanent, KeywordAbility::Indestructible)
}

/// Casts Break of Day and returns the creature it affected.
fn break_of_day(game: &mut Game) -> GameObjectId {
    let bear = creature(10_000, cards::GRIZZLY_BEARS, PlayerId::One);
    let bear_id = bear.card.id;
    game.battlefield.push(bear);

    let spell = card(20_000, cards::BREAK_OF_DAY, PlayerId::One);
    let spell_id = spell.id;
    game.players[PlayerId::One.index()].hand.push(spell);
    game.players[PlayerId::One.index()].mana_pool.white = 1;
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell_id))
        .expect("two mana covers it");
    game.apply(PlayerId::One, action)
        .expect("the cast is legal");
    drain_pending(game);
    bear_id
}

#[test]
fn break_of_day_grants_indestructible_at_five_life() {
    let mut game = ready(5);
    let bear = break_of_day(&mut game);

    assert_eq!(stats(&game, bear), (Some(3), Some(3)), "the pump landed");
    assert!(indestructible(&game, bear), "and so did the fateful hour");
}

/// The control: above the threshold only the pump happens.
#[test]
fn break_of_day_only_pumps_above_five_life() {
    let mut game = ready(6);
    let bear = break_of_day(&mut game);

    assert_eq!(stats(&game, bear), (Some(3), Some(3)));
    assert!(!indestructible(&game, bear), "six life is not fateful");
}

/// The Ironwright's anthem is continuous, so life moving turns it on and off.
#[test]
fn the_ironwrights_anthem_follows_the_life_total() {
    let mut game = ready(5);
    game.battlefield
        .push(creature(10_000, cards::GAVONY_IRONWRIGHT, PlayerId::One));
    let bear = creature(10_100, cards::GRIZZLY_BEARS, PlayerId::One);
    let bear_id = bear.card.id;
    game.battlefield.push(bear);

    assert_eq!(
        stats(&game, bear_id),
        (Some(3), Some(6)),
        "a 2/2 with +1/+4",
    );

    game.players[PlayerId::One.index()].life = 6;
    assert_eq!(
        stats(&game, bear_id),
        (Some(2), Some(2)),
        "one life back and the anthem is gone",
    );

    game.players[PlayerId::One.index()].life = 1;
    assert_eq!(stats(&game, bear_id), (Some(3), Some(6)), "and back again");
}

/// "Other creatures", so the Ironwright does not pump itself.
#[test]
fn the_ironwright_does_not_pump_itself() {
    let mut game = ready(3);
    let iron = creature(10_000, cards::GAVONY_IRONWRIGHT, PlayerId::One);
    let iron_id = iron.card.id;
    game.battlefield.push(iron);

    assert_eq!(stats(&game, iron_id), (Some(1), Some(4)), "printed size");
}

#[test]
fn both_cards_report_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [cards::BREAK_OF_DAY, cards::GAVONY_IRONWRIGHT] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
