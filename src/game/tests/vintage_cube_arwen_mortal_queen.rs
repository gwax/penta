//! Arwen, Mortal Queen: hard to kill until the turn she spends that on
//! somebody else, and both of them stay bigger for good.

use super::*;

/// Arwen on the battlefield under Player One since last turn, with a bear
/// beside her and a mana up.
fn staged() -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let arwen = game
        .put_onto_battlefield(PlayerId::One, cards::ARWEN_MORTAL_QUEEN)
        .expect("cataloged");
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.players[0].life = 20;
    game.players[1].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, arwen, bears)
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

/// Every way Arwen's activated ability can be used right now.
fn blessings(game: &Game, arwen: GameObjectId) -> Vec<(Action, Vec<Target>)> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match &action {
            Action::ActivateAbility {
                source, targets, ..
            } if *source == arwen => {
                let aimed = targets
                    .iter()
                    .flat_map(|selection| selection.targets().iter().copied())
                    .collect();
                Some((action, aimed))
            }
            _ => None,
        })
        .collect()
}

/// She arrives carrying the counter, which makes her indestructible.
#[test]
fn she_enters_indestructible() {
    let (game, arwen, _) = staged();
    let queen = permanent(&game, arwen);

    assert_eq!(queen.counters(CounterKind::Indestructible), 1);
    assert!(
        game.permanent_has_executable_keyword(queen, KeywordAbility::Indestructible),
        "the counter is what grants it",
    );
}

/// Spending the counter hands the other creature a turn of
/// indestructibility and leaves both of them bigger.
#[test]
fn the_counter_buys_a_creature_a_turn_and_two_counters() {
    let (mut game, arwen, bears) = staged();

    let offers = blessings(&game, arwen);
    assert_eq!(offers.len(), 1, "the bear is the only other creature");
    assert_eq!(offers[0].1, vec![Target::Permanent(bears)]);
    game.apply(PlayerId::One, offers[0].0.clone())
        .expect("it activates");
    drain_pending(&mut game);

    let bear = permanent(&game, bears);
    assert_eq!(game.power(bear), Some(3), "a +1/+1 counter");
    assert_eq!(bear.counters(CounterKind::Lifelink), 1);
    assert!(
        game.permanent_has_executable_keyword(bear, KeywordAbility::Indestructible),
        "and indestructible for the turn",
    );
    assert!(
        game.permanent_has_executable_keyword(bear, KeywordAbility::Lifelink),
        "which the counter grants for good",
    );

    let queen = permanent(&game, arwen);
    assert_eq!(game.power(queen), Some(3), "she grows too");
    assert_eq!(queen.counters(CounterKind::Lifelink), 1);
    assert_eq!(
        queen.counters(CounterKind::Indestructible),
        0,
        "and the counter she spent is gone",
    );
    assert!(
        !game.permanent_has_executable_keyword(queen, KeywordAbility::Indestructible),
        "so she is killable now",
    );
}

/// "Another target creature": she is not among the choices.
#[test]
fn she_cannot_target_herself() {
    let (mut game, arwen, _) = staged();
    game.battlefield
        .retain(|permanent| permanent.card.definition != cards::GRIZZLY_BEARS);

    assert!(
        blessings(&game, arwen).is_empty(),
        "with nobody else out there is nothing to point at",
    );
}

/// Without the counter the ability is gone, however much mana is up.
#[test]
fn one_counter_is_one_activation() {
    let (mut game, arwen, _) = staged();
    let offers = blessings(&game, arwen);
    game.apply(PlayerId::One, offers[0].0.clone())
        .expect("it activates");
    drain_pending(&mut game);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 4);

    assert!(
        blessings(&game, arwen).is_empty(),
        "the counter is what pays, and there is only ever one",
    );
}

/// The turn of indestructibility is until end of turn; the counters stay.
#[test]
fn the_grant_wears_off_and_the_counters_do_not() {
    let (mut game, arwen, bears) = staged();
    let offers = blessings(&game, arwen);
    game.apply(PlayerId::One, offers[0].0.clone())
        .expect("it activates");
    drain_pending(&mut game);

    let turn = game.turn;
    for _ in 0..60 {
        if game.turn > turn {
            break;
        }
        game.advance_step();
        drain_pending(&mut game);
    }

    let bear = permanent(&game, bears);
    assert_eq!(game.power(bear), Some(3), "the +1/+1 counter stayed");
    assert!(
        game.permanent_has_executable_keyword(bear, KeywordAbility::Lifelink),
        "and so did the lifelink counter",
    );
    assert!(
        !game.permanent_has_executable_keyword(bear, KeywordAbility::Indestructible),
        "but the turn of indestructibility is over",
    );
}

/// "If the target is illegal as it tries to resolve, the ability does
/// nothing. You won't get to put any counters on Arwen." The counter she
/// spent is spent all the same: it was the cost.
#[test]
fn a_dead_target_costs_her_the_counter_and_gains_her_nothing() {
    let (mut game, arwen, bears) = staged();

    let offers = blessings(&game, arwen);
    game.apply(PlayerId::One, offers[0].0.clone())
        .expect("it activates");
    assert_eq!(
        permanent(&game, arwen).counters(CounterKind::Indestructible),
        0,
        "the counter went as the cost was paid",
    );

    game.move_permanents_to_graveyard(&[bears]);
    drain_pending(&mut game);

    let queen = permanent(&game, arwen);
    assert_eq!(game.power(queen), Some(2), "she is no bigger");
    assert_eq!(
        queen.counters(CounterKind::PlusOnePlusOne),
        0,
        "the countered-out ability put nothing on her",
    );
    assert_eq!(queen.counters(CounterKind::Lifelink), 0);
}

/// "You remove the counter as a cost. If Arwen already received 2 damage
/// earlier in the turn, it will be destroyed before you get to put a +1/+1
/// counter on it." The blessing still lands on its target; she is simply
/// not there to collect her half.
#[test]
fn two_damage_kills_her_the_moment_the_counter_is_spent() {
    let (mut game, arwen, bears) = staged();
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == arwen)
        .expect("she is there")
        .damage = 2;
    game.check_state_based_actions();
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == arwen),
        "the counter is holding her up",
    );

    let offers = blessings(&game, arwen);
    game.apply(PlayerId::One, offers[0].0.clone())
        .expect("it activates");
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == arwen),
        "with the counter spent, two damage on a 2/2 is lethal",
    );

    drain_pending(&mut game);
    let bear = permanent(&game, bears);
    assert_eq!(
        game.power(bear),
        Some(3),
        "and the blessing she paid for still arrives",
    );
    assert!(game.permanent_has_executable_keyword(bear, KeywordAbility::Indestructible));
}

/// A lifelink counter is lifelink in the doing: the bear she blessed attacks
/// as a 3/3 and the three damage it deals comes back as life.
#[test]
fn the_lifelink_counter_gains_life_in_combat() {
    let (mut game, arwen, bears) = staged();
    let offers = blessings(&game, arwen);
    game.apply(PlayerId::One, offers[0].0.clone())
        .expect("it activates");
    drain_pending(&mut game);
    let life = game.players[PlayerId::One.index()].life;

    game.step = Step::DeclareAttackers;
    game.priority = PlayerId::One;
    game.declare_attacker(bears, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    game.finish_declaring_blockers();
    game.deal_combat_damage();
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        17,
        "a 2/2 with a +1/+1 counter hits for three",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life + 3,
        "and the lifelink counter is worth exactly that much",
    );
}

/// "Another target creature" names no controller: the blessing may be spent
/// on a creature across the table, counters and indestructibility and all,
/// and she still collects her own half.
#[test]
fn she_may_bless_a_creature_across_the_table() {
    let (mut game, arwen, _bears) = staged();
    game.battlefield
        .retain(|permanent| permanent.card.definition != cards::GRIZZLY_BEARS);
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let offers = blessings(&game, arwen);
    assert_eq!(
        offers.len(),
        1,
        "their Angel is the only other creature, and it is a legal one",
    );
    assert_eq!(offers[0].1, vec![Target::Permanent(theirs)]);
    game.apply(PlayerId::One, offers[0].0.clone())
        .expect("it activates");
    drain_pending(&mut game);

    let angel = permanent(&game, theirs);
    assert_eq!(
        game.power(angel),
        Some(5),
        "their creature is the bigger one"
    );
    assert!(
        game.permanent_has_executable_keyword(angel, KeywordAbility::Indestructible),
        "and it is the one that cannot be destroyed this turn",
    );
    let queen = permanent(&game, arwen);
    assert_eq!(
        game.power(queen),
        Some(3),
        "she takes her half of the bargain whoever the other half went to",
    );
}
