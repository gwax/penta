//! Slickshot Show-Off: a hasty flier that grows with every spell after it,
//! and a plot cost that pays for it a turn early.

use super::*;

/// Player One holding the Bird, with `hand` beside it and mana to spare.
fn staged(hand: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let card = game
        .build_zone(PlayerId::One, &[cards::SLICKSHOT_SHOW_OFF])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[0].hand.push(card);
    let mut others = Vec::new();
    for definition in hand {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        others.push(card.id);
        game.players[0].hand.push(card);
    }
    game.players[1].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held, others)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(game);
}

/// Casts `card` from Player One's hand.
fn cast(game: &mut Game, card: GameObjectId) {
    for color in [ManaColor::Red, ManaColor::Colorless] {
        game.add_unrestricted_mana(PlayerId::One, color, 3);
    }
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card: id, .. } if *id == card))
        .expect("it is castable");
    game.apply(PlayerId::One, action).expect("it is cast");
    settle(game);
}

fn bird(game: &Game) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SLICKSHOT_SHOW_OFF)
        .expect("the Bird is on the battlefield")
}

/// It arrives as a hasty 1/2 flier.
#[test]
fn it_arrives_flying_and_hasty() {
    let (mut game, held, _) = staged(&[]);

    cast(&mut game, held);

    let body = bird(&game);
    assert_eq!(game.power(body), Some(1));
    assert_eq!(game.toughness(body), Some(2));
    assert!(game.has_flying(body));
    assert!(game.permanent_has_executable_keyword(body, KeywordAbility::Haste));
}

/// Every noncreature spell after it is +2/+0, and they stack.
#[test]
fn each_noncreature_spell_grows_it() {
    let (mut game, held, others) = staged(&[cards::SHOCK, cards::SHOCK]);
    cast(&mut game, held);
    assert_eq!(game.power(bird(&game)), Some(1));

    cast(&mut game, others[0]);
    assert_eq!(game.power(bird(&game)), Some(3), "+2/+0");

    cast(&mut game, others[1]);
    assert_eq!(game.power(bird(&game)), Some(5), "and again");
    assert_eq!(game.toughness(bird(&game)), Some(2), "toughness untouched");
}

/// A creature spell is not a noncreature spell.
#[test]
fn a_creature_spell_does_nothing() {
    let (mut game, held, others) = staged(&[cards::GRIZZLY_BEARS]);
    cast(&mut game, held);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);

    cast(&mut game, others[0]);

    assert_eq!(game.power(bird(&game)), Some(1), "still a 1/2");
}

/// The pump is until end of turn.
#[test]
fn the_pump_wears_off() {
    let (mut game, held, others) = staged(&[cards::SHOCK]);
    cast(&mut game, held);
    cast(&mut game, others[0]);
    assert_eq!(game.power(bird(&game)), Some(3));

    let turn = game.turn;
    for _ in 0..60 {
        if game.turn > turn {
            break;
        }
        game.advance_step();
        drain_pending(&mut game);
    }

    assert_eq!(game.power(bird(&game)), Some(1), "back to a 1/2");
}

/// Plot pays the two now and casts it for nothing on a later turn.
#[test]
fn plotting_it_pays_a_turn_early() {
    let (mut game, held, _) = staged(&[]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    let plot = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::Plot { card } if *card == held))
        .expect("two mana pays the plot cost");
    game.apply(PlayerId::One, plot).expect("it plots");

    assert!(game.players[0].hand.is_empty());
    assert_eq!(game.players[0].exile.len(), 1, "it waits in exile");
    let plotted = game.players[0].exile[0].id;
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == plotted)),
        "not on the turn it was plotted",
    );

    game.commit_next_turn(PlayerId::Two, Vec::new());
    game.commit_next_turn(PlayerId::One, Vec::new());
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].mana_pool = ManaPool::default();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == plotted))
        .expect("a plotted card is castable on a later turn");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert_eq!(game.power(bird(&game)), Some(1), "the Bird arrived");
    assert_eq!(
        game.players[0].mana_pool.total(),
        0,
        "and cost nothing to cast",
    );
}
