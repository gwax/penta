//! Basalt Monolith: three mana that makes three mana, and will not stand
//! itself back up without being paid for.

use super::*;

/// The Monolith untapped on the battlefield since last turn, with an empty
/// pool.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let monolith = game
        .put_onto_battlefield(PlayerId::One, cards::BASALT_MONOLITH)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    drain_pending(&mut game);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, monolith)
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

/// The `{T}: Add {C}{C}{C}` half. A mana ability is its own kind of action
/// and uses no stack.
fn tap_action(game: &Game, monolith: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateManaAbility { source, color, .. }
                if *source == monolith && *color == ManaColor::Colorless)
        })
}

/// The `{3}: Untap` half, which is an ordinary activated ability and does
/// use the stack.
fn untap_action(game: &Game, monolith: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One).into_iter().find(
        |action| matches!(action, Action::ActivateAbility { source, .. } if *source == monolith),
    )
}

fn pool(game: &Game) -> usize {
    game.players[0].mana.len()
}

/// Three at once, and colorless rather than any colour you like.
#[test]
fn it_taps_for_three_colourless() {
    let (mut game, monolith) = staged();

    let tap = tap_action(&game, monolith).expect("an untapped Monolith taps for mana");
    game.apply(PlayerId::One, tap)
        .expect("it costs only the tap");

    assert_eq!(pool(&game), 3, "three mana, not one");
    assert!(
        game.players[0]
            .mana
            .iter()
            .all(|mana| mana.color == ManaColor::Colorless),
        "and all of it colorless",
    );
    assert!(permanent(&game, monolith).tapped, "it tapped to do it");
    assert!(
        tap_action(&game, monolith).is_none(),
        "and a tapped Monolith has no tap left to give",
    );
}

/// "This artifact doesn't untap during your untap step": the step passes it
/// by and leaves it where it is.
#[test]
fn the_untap_step_passes_it_by() {
    let (mut game, monolith) = staged();
    let tap = tap_action(&game, monolith).expect("it taps");
    game.apply(PlayerId::One, tap).expect("it activates");

    game.choose_untap(PlayerId::One, &[monolith]);

    assert!(
        permanent(&game, monolith).tapped,
        "still tapped after the step that untaps everything else",
    );
}

/// The untap costs exactly what the tap produced, so the pair is
/// mana-neutral: three in, three out, and the Monolith standing again with
/// nothing gained. That is why it takes a third card to break.
#[test]
fn untapping_it_costs_back_what_it_made() {
    let (mut game, monolith) = staged();
    let tap = tap_action(&game, monolith).expect("it taps");
    game.apply(PlayerId::One, tap).expect("it activates");
    assert_eq!(pool(&game), 3);

    let untap = untap_action(&game, monolith).expect("three colorless pays for it");
    game.apply(PlayerId::One, untap).expect("it activates");
    drain_pending(&mut game);

    assert!(!permanent(&game, monolith).tapped, "it is back up");
    assert_eq!(
        pool(&game),
        0,
        "and the three it made paid for standing back up",
    );
}

/// "Basalt Monolith's last ability can untap it as often as you can pay for
/// it." Nothing about it is once a turn, and three trips round leave the
/// board exactly where it started.
#[test]
fn it_untaps_as_often_as_you_pay() {
    let (mut game, monolith) = staged();

    for round in 0..3 {
        let tap = tap_action(&game, monolith)
            .unwrap_or_else(|| panic!("it is untapped at the top of round {round}"));
        game.apply(PlayerId::One, tap).expect("it activates");
        let untap = untap_action(&game, monolith)
            .unwrap_or_else(|| panic!("its own three pays the untap on round {round}"));
        game.apply(PlayerId::One, untap).expect("it activates");
        drain_pending(&mut game);
        assert!(
            !permanent(&game, monolith).tapped,
            "back up at the end of round {round}",
        );
    }

    assert_eq!(pool(&game), 0, "three trips round and not a mana ahead");
}

/// It pays for its own untap: with an empty pool the {3} is still payable,
/// because tapping the Monolith is where the three comes from. The cost taps
/// it and the ability stands it back up, which is a treadmill rather than a
/// combo -- the pool is empty at both ends.
#[test]
fn it_pays_for_its_own_untap_and_gains_nothing() {
    let (mut game, monolith) = staged();
    assert_eq!(pool(&game), 0, "nothing in the pool to start");

    let untap = untap_action(&game, monolith).expect("its own tap can pay the {3}");
    game.apply(PlayerId::One, untap).expect("it activates");
    assert!(
        permanent(&game, monolith).tapped,
        "paying the cost tapped it, with the ability still on the stack",
    );

    drain_pending(&mut game);

    assert!(!permanent(&game, monolith).tapped, "which then untaps it");
    assert_eq!(pool(&game), 0, "and the pool is where it started");
}
