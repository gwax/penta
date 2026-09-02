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

/// "Add X mana ... where X is Vivi Ornitier's power." A Vivi that has not
/// grown yet is a 0/3, and there is no division of nothing to offer.
#[test]
fn a_vivi_that_has_not_grown_makes_no_mana() {
    let (game, vivi) = staged(0);

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(
                action,
                Action::ActivateManaAbility { source, .. } if *source == vivi
            )),
        "zero power is zero mana",
    );
}

/// The cost is {0} and nothing else: no tap symbol, so a Vivi that has
/// already attacked -- or is tapped for any other reason -- still turns its
/// power into mana.
#[test]
fn a_tapped_vivi_still_makes_its_mana() {
    let (mut game, vivi) = staged(2);
    game.tap_permanent(vivi);
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                Action::ActivateManaAbility { source, .. } if *source == vivi
            )
        })
        .expect("the ability wants no tap");
    game.apply(PlayerId::One, action).expect("it activates");

    assert_eq!(
        game.players[0].mana_pool.blue + game.players[0].mana_pool.red,
        2,
        "two power is still two mana",
    );
}

/// "Whenever you cast a noncreature spell" is once per spell: two Bolts are
/// two counters and two points, and the second one reads the size the first
/// left behind.
#[test]
fn every_noncreature_spell_grows_it_again() {
    let (mut game, vivi) = staged(0);
    for id in [116_300, 116_301] {
        game.players[0]
            .hand
            .push(card(id, cards::LIGHTNING_BOLT, PlayerId::One));
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    }
    let life = game.players[1].life;

    for id in [116_300, 116_301] {
        game.priority = PlayerId::One;
        let cast = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == CardInstanceId(id)
                        && choices
                            .iter_targets()
                            .any(|target| *target == Target::Player(PlayerId::Two))
                }
                _ => false,
            })
            .unwrap_or_else(|| panic!("one red points {id} at them"));
        game.apply(PlayerId::One, cast).expect("it is cast");
        drain_pending(&mut game);
    }

    assert_eq!(counters_on(&game, vivi), 2, "a counter for each spell");
    assert_eq!(
        game.players[1].life,
        life - 2 - 6,
        "two triggers and two Bolts",
    );
}

/// "Activate only during your turn." Their turn is not yours, however much
/// power is standing there, so the mana it would make is not available to
/// answer anything with.
#[test]
fn the_mana_waits_for_your_own_turn() {
    let (mut game, vivi) = staged(2);
    let offers = |game: &Game| {
        game.legal_actions(PlayerId::One).into_iter().any(
            |action| matches!(action, Action::ActivateManaAbility { source, .. } if source == vivi),
        )
    };
    assert!(offers(&game), "on your own turn two power is two mana");

    game.turn += 1;
    game.active_player = PlayerId::Two;
    game.turns_started[PlayerId::Two.index()] += 1;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    assert!(
        !offers(&game),
        "and on theirs it is a 2/5 that makes nothing",
    );
}

/// "And only once each turn." The second activation is not on offer however
/// much power is left to convert.
#[test]
fn it_makes_its_mana_once_a_turn() {
    let (mut game, vivi) = staged(2);
    let offer = |game: &Game| {
        game.legal_actions(PlayerId::One).into_iter().find(|action| {
            matches!(action, Action::ActivateManaAbility { source, .. } if *source == vivi)
        })
    };

    let first = offer(&game).expect("the first one is there");
    game.apply(PlayerId::One, first).expect("it activates");
    assert!(
        game.players[PlayerId::One.index()].mana_pool.total() >= 1,
        "and it made something",
    );

    assert!(
        offer(&game).is_none(),
        "once each turn, so there is no second helping",
    );
}

/// "Add X mana in any combination of {U} and/or {R}": two power is two mana
/// and both colours are on the menu, so the pair may be split rather than
/// taken all of one.
#[test]
fn the_two_mana_may_be_split_between_the_colours() {
    let (game, vivi) = staged(2);

    let colors = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateManaAbility { source, color, .. } if source == vivi => Some(color),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        colors.contains(&ManaColor::Blue) && colors.contains(&ManaColor::Red),
        "both halves of the combination are offered: {colors:?}",
    );
    assert!(
        !colors.contains(&ManaColor::Green)
            && !colors.contains(&ManaColor::White)
            && !colors.contains(&ManaColor::Black)
            && !colors.contains(&ManaColor::Colorless),
        "and nothing outside the two it names",
    );
}

/// Only the mana half waits for your turn. "Whenever you cast a noncreature
/// spell" names no turn at all, so an instant on their turn grows Vivi and
/// burns them just the same -- and the mana that growth is worth still has
/// to wait until the turn comes round.
#[test]
fn the_trigger_does_not_wait_for_your_turn() {
    let (mut game, vivi) = staged(0);
    game.players[0]
        .hand
        .push(card(116_400, cards::LIGHTNING_BOLT, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);

    game.turn += 1;
    game.active_player = PlayerId::Two;
    game.turns_started[PlayerId::Two.index()] += 1;
    game.step = Step::End;
    game.priority = PlayerId::One;
    let life = game.players[1].life;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == CardInstanceId(116_400)
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Player(PlayerId::Two))
            }
            _ => false,
        })
        .expect("an instant is castable in their end step");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(
        counters_on(&game, vivi),
        1,
        "the counter came on their turn"
    );
    assert_eq!(
        game.players[1].life,
        life - 1 - 3,
        "the trigger's point and the Bolt's three",
    );
    assert!(
        !game.legal_actions(PlayerId::One).into_iter().any(
            |action| matches!(action, Action::ActivateManaAbility { source, .. } if source == vivi)
        ),
        "and the mana the counter is worth is still a turn away",
    );
}
