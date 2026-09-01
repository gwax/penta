//! Griselbrand: seven life for seven cards, and a body that pays the life
//! back.

use super::*;

/// The Demon on the battlefield since last turn, at `life`.
fn staged(life: i16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].library.clear();
    for index in 0..20 {
        game.players[PlayerId::One.index()].library.push(card(
            114_000 + index,
            cards::MOUNTAIN,
            PlayerId::One,
        ));
    }
    let demon = game
        .put_onto_battlefield(PlayerId::One, cards::GRISELBRAND)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.players[PlayerId::One.index()].life = life;
    game.players[PlayerId::Two.index()].life = 20;
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, demon)
}

fn draw_offered(game: &Game, demon: GameObjectId) -> bool {
    game.legal_actions(PlayerId::One)
        .iter()
        .any(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == demon))
}

fn draw_seven(game: &mut Game, demon: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == demon))
        .expect("seven life buys seven cards");
    game.apply(PlayerId::One, action).expect("it activates");
    pass_priority_pair(game);
    drain_pending(game);
}

/// "You can't activate Griselbrand's ability if you have 6 or less life."
/// Seven is exactly payable (CR 118.4 allows paying down to zero) -- and
/// paying it is a loss before a single card is drawn, because the life is a
/// cost and zero life is checked before the ability resolves.
#[test]
fn seven_life_is_payable_and_six_is_not() {
    let (game, demon) = staged(6);
    assert!(!draw_offered(&game, demon), "six is not seven");

    let (mut game, demon) = staged(7);
    assert!(draw_offered(&game, demon), "and seven is");
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == demon))
        .expect("seven life buys the activation");
    game.apply(PlayerId::One, action).expect("it activates");
    game.check_state_based_actions();

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        0,
        "the life is paid as the cost, before anything resolves",
    );
    assert!(
        matches!(
            game.result,
            Some(GameResult::Winner {
                winner: PlayerId::Two,
                ..
            })
        ),
        "which is a loss: {:?}",
        game.result,
    );
    assert!(
        game.players[PlayerId::One.index()].hand.is_empty(),
        "and the seven cards were never drawn",
    );
}

/// The body is what makes the ability repeatable: seven damage with lifelink
/// is seven life back, which is exactly one more activation.
#[test]
fn his_lifelink_pays_for_the_next_seven() {
    let (mut game, demon) = staged(20);
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == demon)
        .expect("he is there");
    assert!(game.permanent_has_executable_keyword(permanent, KeywordAbility::Flying));
    assert!(game.permanent_has_executable_keyword(permanent, KeywordAbility::Lifelink));

    draw_seven(&mut game, demon);
    assert_eq!(game.players[PlayerId::One.index()].life, 13, "seven paid");

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;
    game.declare_attacker(demon, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    game.finish_declaring_blockers();
    game.deal_combat_damage();
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        13,
        "seven in the air",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        20,
        "and lifelink gave the seven back",
    );

    game.step = Step::PostcombatMain;
    game.priority = PlayerId::One;
    draw_seven(&mut game, demon);

    assert_eq!(
        game.players[PlayerId::One.index()].hand.len(),
        14,
        "fourteen cards off one Demon and one attack",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        13,
        "for the same seven life, twice",
    );
}
