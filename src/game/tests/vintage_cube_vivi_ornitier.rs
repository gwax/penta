//! Vivi Ornitier: a 0/3 that grows on every noncreature spell and turns the
//! size into mana once a turn.
//!
//! How the mana divides, the once-a-turn gate, and the growth are pinned in
//! `vintage_cube_mana`. What is here is the pair of rulings around them: the
//! trigger that resolves before its own spell, and the mana ability that
//! answers to nobody.

use super::*;

/// Vivi on the battlefield with `mana` red available and `counters` already
/// on it.
fn staged(counters: u16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    let mut vivi = creature(116_000, cards::VIVI_ORNITIER, PlayerId::One);
    vivi.set_counters(CounterKind::PlusOnePlusOne, counters);
    let vivi_id = vivi.card.id;
    game.battlefield.push(vivi);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, vivi_id)
}

fn counters_on(game: &Game, vivi: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == vivi)
        .map_or(0, |permanent| {
            permanent.counters(CounterKind::PlusOnePlusOne)
        })
}

/// "Vivi Ornitier's last ability resolves before the spell that caused it to
/// trigger. It resolves even if that spell is countered." The Bolt is
/// answered and the counter and the damage happen anyway.
#[test]
fn the_trigger_outlives_the_spell_that_made_it() {
    let (mut game, vivi) = staged(0);
    game.players[0]
        .hand
        .push(card(116_100, cards::LIGHTNING_BOLT, PlayerId::One));
    game.players[1]
        .hand
        .push(card(116_101, cards::COUNTERSPELL, PlayerId::Two));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);
    let life = game.players[1].life;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, .. } if *card == CardInstanceId(116_100))
        })
        .expect("one red casts the Bolt");
    game.apply(PlayerId::One, cast).expect("it is cast");

    game.priority = PlayerId::Two;
    let counter = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, .. } if *card == CardInstanceId(116_101))
        })
        .expect("two blue answers it");
    game.apply(PlayerId::Two, counter).expect("it is cast");
    drain_pending(&mut game);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "the Bolt was countered",
    );
    assert_eq!(counters_on(&game, vivi), 1, "and the counter still landed");
    assert_eq!(
        game.players[1].life,
        life - 1,
        "with the one damage that came with it",
    );
}

/// "Vivi Ornitier's first ability is a mana ability. It doesn't use the
/// stack and it can't be responded to."
#[test]
fn the_mana_ability_uses_no_stack() {
    let (mut game, vivi) = staged(2);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateManaAbility { source, color, .. } => {
                *source == vivi && *color == ManaColor::Blue
            }
            _ => false,
        })
        .expect("two power is two mana to divide");
    game.apply(PlayerId::One, action).expect("it activates");

    assert!(
        game.stack.is_empty(),
        "a mana ability resolves where it stands",
    );
    assert!(
        game.players[0].mana_pool.blue + game.players[0].mana_pool.red >= 1,
        "and the mana is already in the pool",
    );
}

/// "Whenever you cast a *noncreature* spell": a creature spell grows it by
/// nothing and burns nobody.
#[test]
fn a_creature_spell_does_not_grow_it() {
    let (mut game, vivi) = staged(0);
    game.players[0]
        .hand
        .push(card(116_200, cards::GRIZZLY_BEARS, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    let life = game.players[1].life;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, .. } if *card == CardInstanceId(116_200))
        })
        .expect("two mana casts the Bears");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(counters_on(&game, vivi), 0, "no counter for a creature");
    assert_eq!(game.players[1].life, life, "and no damage either");
}
