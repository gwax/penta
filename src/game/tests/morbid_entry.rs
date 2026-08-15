//! Morbid as an intervening if.
//!
//! With nothing dead the trigger is never created, which is not the same as a
//! trigger that resolves and does nothing. Ulvenwald Bear is where the
//! difference shows: an uncreated trigger never asks for a target, so nothing
//! goes on the stack pointing at a creature.

use super::*;
use crate::ImplementationStatus;

/// A board with a creature to point at and, if `a_death`, one creature
/// already dead this turn.
fn board(a_death: bool) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.turn = 5;
    game.turns_started[PlayerId::One.index()] = 5;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;

    let bystander = creature(10_000, cards::GRIZZLY_BEARS, PlayerId::One);
    let bystander_id = bystander.card.id;
    game.battlefield.push(bystander);

    if a_death {
        let victim = creature(10_500, cards::SEDGE_TROLL, PlayerId::Two);
        let victim_id = victim.card.id;
        game.battlefield.push(victim);
        game.destroy_permanent(victim_id);
        drain_pending(&mut game);
    }
    game.priority = PlayerId::One;
    (game, bystander_id)
}

/// Casts `spell` from hand, paying with a full pool.
fn cast(game: &mut Game, spell: CardDefinitionId) {
    let card_in_hand = card(20_000, spell, PlayerId::One);
    let spell_id = card_in_hand.id;
    game.players[PlayerId::One.index()].hand.push(card_in_hand);
    let pool = &mut game.players[PlayerId::One.index()].mana_pool;
    pool.black = 1;
    pool.green = 1;
    pool.colorless = 2;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell_id))
        .expect("the pool covers it");
    game.apply(PlayerId::One, action)
        .expect("the cast is legal");
    drain_pending(game);
}

fn zombies(game: &Game) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == cards::ZOMBIE_TOKEN_2_2_BLACK)
        .count()
}

fn counters(game: &Game, id: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("still there")
        .counters(CounterKind::PlusOnePlusOne)
}

#[test]
fn wakedancer_makes_a_zombie_when_something_died() {
    let (mut game, _bystander) = board(true);
    cast(&mut game, cards::WAKEDANCER);
    assert_eq!(zombies(&game), 1);
}

/// The control: a quiet turn, no token.
#[test]
fn wakedancer_makes_nothing_on_a_quiet_turn() {
    let (mut game, _bystander) = board(false);
    cast(&mut game, cards::WAKEDANCER);
    assert_eq!(zombies(&game), 0);
}

#[test]
fn ulvenwald_bear_adds_two_counters_when_something_died() {
    let (mut game, bystander) = board(true);
    cast(&mut game, cards::ULVENWALD_BEAR);
    assert_eq!(counters(&game, bystander), 2);
}

/// The sharper half of the control: with nothing dead the trigger is not
/// created, so no target is ever chosen and no counters land anywhere.
#[test]
fn ulvenwald_bear_asks_for_no_target_on_a_quiet_turn() {
    let (mut game, bystander) = board(false);
    cast(&mut game, cards::ULVENWALD_BEAR);

    assert_eq!(counters(&game, bystander), 0);
    let bear = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::ULVENWALD_BEAR)
        .expect("the Bear resolved");
    assert_eq!(
        counters(&game, bear.card.id),
        0,
        "and it did not point at itself either",
    );
}

#[test]
fn both_cards_report_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [cards::WAKEDANCER, cards::ULVENWALD_BEAR] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
