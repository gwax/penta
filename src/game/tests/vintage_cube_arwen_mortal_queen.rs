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
