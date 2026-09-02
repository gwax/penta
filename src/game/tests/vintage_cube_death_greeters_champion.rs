//! Death-Greeter's Champion: backup, which is a counter for anything and
//! double strike for anything that is not itself.

use super::*;

/// The Champion in hand with `mine` on the battlefield under Player One and
/// mana enough for either cost.
fn staged(mine: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let mut ids = Vec::new();
    for definition in mine {
        ids.push(
            game.put_onto_battlefield(PlayerId::One, *definition)
                .expect("cataloged"),
        );
    }
    let champion = game
        .build_zone(PlayerId::One, &[cards::DEATH_GREETER_S_CHAMPION])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let champion_id = champion.id;
    game.players[0].hand.push(champion);
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 4);
    (game, champion_id, ids)
}

/// Casts the Champion for the cost that pays `x` mana total, aiming its
/// backup trigger at `subject`.
fn cast_and_back_up(game: &mut Game, champion: GameObjectId, subject: Option<GameObjectId>) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == champion))
        .expect("there is mana for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(game, subject);
}

/// Answers whatever is asked, preferring an option that names `wanted`.
fn settle(game: &mut Game, wanted: Option<GameObjectId>) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let mut options = wanted
                .into_iter()
                .flat_map(|wanted| {
                    decision
                        .options
                        .iter()
                        .filter(move |option| option.card.is_some_and(|(card, _)| card == wanted))
                })
                .map(|option| option.id)
                .take(1)
                .collect::<Vec<_>>();
            // Not every question this turn is the backup target: the dash
            // return asks its own, and it does not name a card of ours.
            if options.is_empty() {
                options = decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(decision.minimum.max(1))
                    .collect();
            }
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the offered choice is legal");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

fn champion_on_battlefield(game: &Game) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::DEATH_GREETER_S_CHAMPION)
        .expect("the Champion resolved")
}

fn has_double_strike(game: &Game, permanent: &Permanent) -> bool {
    game.permanent_has_executable_keyword(permanent, KeywordAbility::DoubleStrike)
}

/// The counter goes on the other creature, and so does the double strike.
#[test]
fn backing_up_a_bear_lends_it_the_double_strike() {
    let (mut game, champion, mine) = staged(&[cards::GRIZZLY_BEARS]);
    let bears = mine[0];

    cast_and_back_up(&mut game, champion, Some(bears));

    let bear = permanent(&game, bears);
    assert_eq!(bear.counters(CounterKind::PlusOnePlusOne), 1, "one counter");
    assert_eq!(game.power(bear), Some(3), "a 3/3 now");
    assert!(has_double_strike(&game, bear), "and it hits twice");
}

/// The Champion has double strike printed on it, whoever it backs up.
#[test]
fn the_champion_keeps_its_own_double_strike() {
    let (mut game, champion, mine) = staged(&[cards::GRIZZLY_BEARS]);

    cast_and_back_up(&mut game, champion, Some(mine[0]));

    assert!(
        has_double_strike(&game, champion_on_battlefield(&game)),
        "the listed ability is printed on it as well",
    );
}

/// Backing itself up is legal: the counter lands on the Champion, and the
/// loan clause finds nobody to lend to.
#[test]
fn it_may_back_itself_up() {
    let (mut game, champion, _mine) = staged(&[]);

    cast_and_back_up(&mut game, champion, None);

    let champion = champion_on_battlefield(&game);
    assert_eq!(
        champion.counters(CounterKind::PlusOnePlusOne),
        1,
        "the counter went on itself",
    );
    assert_eq!(game.power(champion), Some(3));
}

/// The loan is until end of turn, and the counter is not.
#[test]
fn the_double_strike_wears_off_but_the_counter_stays() {
    let (mut game, champion, mine) = staged(&[cards::GRIZZLY_BEARS]);
    let bears = mine[0];
    cast_and_back_up(&mut game, champion, Some(bears));

    game.cleanup();

    let bear = permanent(&game, bears);
    assert_eq!(bear.counters(CounterKind::PlusOnePlusOne), 1, "still a 3/3");
    assert!(
        !has_double_strike(&game, bear),
        "but the double strike was a loan",
    );
}

/// Dashed, it arrives with haste and goes home at the end step -- and the
/// backup trigger happens either way.
#[test]
fn dashing_it_still_backs_something_up() {
    let (mut game, champion, mine) = staged(&[cards::GRIZZLY_BEARS]);
    let bears = mine[0];
    let dash = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .rfind(|action| matches!(action, Action::CastSpell { card, .. } if *card == champion))
        .expect("the dash cost is offered as well");
    game.apply(PlayerId::One, dash).expect("it is dashed");
    settle(&mut game, Some(bears));

    let dashed = champion_on_battlefield(&game);
    assert!(
        game.permanent_has_executable_keyword(dashed, KeywordAbility::Haste),
        "dashed creatures have haste",
    );
    assert_eq!(
        permanent(&game, bears).counters(CounterKind::PlusOnePlusOne),
        1,
        "and the backup trigger happened all the same",
    );

    game.step = Step::End;
    game.begin_step_triggers();
    settle(&mut game, None);

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::DEATH_GREETER_S_CHAMPION),
        "and it went home at the end step",
    );
}

/// Backup says "target creature" and stops there: no controller is named, so
/// their creature is as legal a subject as yours. Doing it hands them the
/// counter and the double strike, which is why the choice is worth having
/// tested rather than assumed away.
#[test]
fn backup_may_be_pointed_at_a_creature_they_control() {
    let (mut game, champion, _mine) = staged(&[]);
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    assert!(
        !has_double_strike(&game, permanent(&game, theirs)),
        "a bear does not strike twice on its own",
    );

    cast_and_back_up(&mut game, champion, Some(theirs));

    let bear = permanent(&game, theirs);
    assert_eq!(bear.controller, PlayerId::Two, "it is still theirs");
    assert_eq!(
        bear.counters(CounterKind::PlusOnePlusOne),
        1,
        "and the counter went to them",
    );
    assert!(
        has_double_strike(&game, bear),
        "along with the loan: backup lends to whoever it named",
    );
    assert_eq!(
        (game.power(bear), game.toughness(bear)),
        (Some(3), Some(3)),
        "a 2/2 wearing a counter",
    );
    assert!(
        has_double_strike(&game, champion_on_battlefield(&game)),
        "and the Champion keeps its own regardless",
    );
}
