//! Static Prison: energy counters, and a jail with a two-turn lease.

use super::*;

/// Answers every pending decision with the first option it offered, then
/// resolves whatever is left on the stack.
fn settle_paying(game: &mut Game, pay: bool) {
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            // Chosen by label rather than by position: the payment branch
            // is the one that says so.
            let paying = decision
                .options
                .iter()
                .find(|option| option.label.starts_with("Pay "));
            let wanted = match (pay, paying) {
                (true, Some(option)) => Some(option),
                (true, None) => decision.options.first(),
                (false, _) => decision
                    .options
                    .iter()
                    .find(|option| !option.label.starts_with("Pay "))
                    .or_else(|| decision.options.first()),
            };
            let options = wanted.map(|option| vec![option.id]).unwrap_or_default();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the decision accepts what it offered");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// A Prison on the battlefield with the opponent's Angel already exiled.
fn jailed() -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let angel = creature(93_100, cards::SERRA_ANGEL, PlayerId::Two);
    let angel_id = angel.card.id;
    game.battlefield.push(angel);
    let prison = game
        .put_onto_battlefield(PlayerId::One, cards::STATIC_PRISON)
        .expect("cataloged");
    settle_paying(&mut game, true);
    drain_pending(&mut game);
    (game, prison, angel_id)
}

/// The entry exiles their creature and pays out two energy.
#[test]
fn the_prison_exiles_a_permanent_and_gives_two_energy() {
    let (game, _prison, angel_id) = jailed();

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != angel_id),
        "the creature is exiled",
    );
    assert_eq!(
        game.observe(PlayerId::One).energy_counters[0],
        2,
        "and the two energy came with it",
    );
}

/// Paying the energy keeps the jail shut, and spends one.
#[test]
fn paying_the_energy_keeps_the_prison_and_the_prisoner() {
    let (mut game, prison, angel_id) = jailed();

    game.capture_battlefield_triggers(&CommittedTriggerEvent::StepBegins {
        step: TurnStepDef::PrecombatMain,
        player: PlayerId::One,
    });
    game.finish_rules_procedure();
    settle_paying(&mut game, true);
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == prison),
        "the Prison is still standing",
    );
    assert_eq!(
        game.observe(PlayerId::One).energy_counters[0],
        1,
        "one energy of the two is gone",
    );
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != angel_id),
        "and the prisoner is still exiled",
    );
}

/// Declining sacrifices the Prison, and the prisoner walks out.
#[test]
fn declining_frees_the_prisoner() {
    let (mut game, prison, _angel_id) = jailed();

    game.capture_battlefield_triggers(&CommittedTriggerEvent::StepBegins {
        step: TurnStepDef::PrecombatMain,
        player: PlayerId::One,
    });
    game.finish_rules_procedure();
    settle_paying(&mut game, false);
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != prison),
        "the Prison was sacrificed",
    );
    let angel = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SERRA_ANGEL)
        .expect("the prisoner is back on the battlefield");
    assert_eq!(angel.controller, PlayerId::Two, "under its owner's control");
    assert_eq!(
        game.observe(PlayerId::One).energy_counters[0],
        2,
        "and nothing was spent",
    );
}

/// Energy is spent in full or not at all: with none left, the payment is not
/// even offered and the Prison goes.
#[test]
fn a_player_out_of_energy_cannot_pay_at_all() {
    let (mut game, prison, _angel_id) = jailed();
    game.players[0]
        .counters
        .set(CounterKind::named("energy"), 0);

    game.capture_battlefield_triggers(&CommittedTriggerEvent::StepBegins {
        step: TurnStepDef::PrecombatMain,
        player: PlayerId::One,
    });
    game.finish_rules_procedure();
    settle_paying(&mut game, true);
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != prison),
        "there was nothing to pay with",
    );
}

/// The tax is on your own first main phase, not the opponent's.
#[test]
fn the_opponents_main_phase_costs_nothing() {
    let (mut game, prison, _angel_id) = jailed();

    game.capture_battlefield_triggers(&CommittedTriggerEvent::StepBegins {
        step: TurnStepDef::PrecombatMain,
        player: PlayerId::Two,
    });
    game.finish_rules_procedure();
    settle_paying(&mut game, true);
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == prison),
        "no trigger, so nothing to pay",
    );
    assert_eq!(game.observe(PlayerId::One).energy_counters[0], 2);
}

/// "If a token is exiled this way, it will cease to exist and won't return
/// to the battlefield." Letting the Prison go frees nothing.
#[test]
fn a_token_it_jails_never_comes_back() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.put_onto_battlefield(PlayerId::Two, cards::ESIKA_S_CHARIOT)
        .expect("cataloged");
    settle_paying(&mut game, true);
    drain_pending(&mut game);
    let cats = game
        .battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Token)
        .count();
    assert_eq!(cats, 2, "the Chariot brought two Cats");

    let cat = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Token)
        .expect("a Cat is there")
        .card
        .id;
    let prison = game
        .put_onto_battlefield(PlayerId::One, cards::STATIC_PRISON)
        .expect("cataloged");
    // Name a Cat rather than the Chariot that made them.
    for _ in 0..8 {
        if game.pending_decisions.is_empty() {
            let priority = game.priority;
            if game.apply(priority, Action::PassPriority).is_err() {
                break;
            }
        }
        let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        else {
            break;
        };
        let options = decision
            .options
            .iter()
            .find(|option| option.card.is_some_and(|(id, _)| id == cat))
            .or_else(|| decision.options.first())
            .map(|option| vec![option.id])
            .unwrap_or_default();
        game.apply(
            decision.player,
            Action::ChooseDecision {
                decision: decision.id,
                options,
            },
        )
        .expect("the offered choice is legal");
    }
    settle_paying(&mut game, true);
    drain_pending(&mut game);
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == ObjectKind::Token)
            .count(),
        cats - 1,
        "one of them is in jail",
    );

    game.move_permanents_to_graveyard(&[prison]);
    settle_paying(&mut game, true);
    drain_pending(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == ObjectKind::Token)
            .count(),
        cats - 1,
        "and it does not walk out: a token in exile stops existing",
    );
}

/// "If Static Prison leaves the battlefield before its first triggered
/// ability resolves, the target permanent won't be exiled."
#[test]
fn a_prison_answered_on_the_way_in_jails_nobody() {
    let mut game = ready_game();
    game.battlefield.clear();
    let angel = creature(93_200, cards::SERRA_ANGEL, PlayerId::Two);
    let angel_id = angel.card.id;
    game.battlefield.push(angel);
    let prison = game
        .put_onto_battlefield(PlayerId::One, cards::STATIC_PRISON)
        .expect("cataloged");

    // The enters trigger is on the stack; the Prison is answered first.
    game.move_permanents_to_graveyard(&[prison]);
    settle_paying(&mut game, true);
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == angel_id),
        "the Angel never left the battlefield",
    );
    assert!(
        game.players[1].exile.is_empty(),
        "and nothing is in exile waiting on a Prison that is already gone",
    );
}

/// "Target nonland permanent an opponent controls": your own board is not on
/// the list, and neither is a land on theirs.
#[test]
fn it_names_only_a_nonland_permanent_of_theirs() {
    let mut game = ready_game();
    game.battlefield.clear();
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    game.put_onto_battlefield(PlayerId::Two, cards::ISLAND)
        .expect("cataloged");
    game.put_onto_battlefield(PlayerId::One, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);

    game.put_onto_battlefield(PlayerId::One, cards::STATIC_PRISON)
        .expect("cataloged");
    for _ in 0..8 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("it asks what to jail");
    assert_eq!(
        decision
            .options
            .iter()
            .filter_map(|option| option.card.map(|(id, _)| id))
            .collect::<Vec<_>>(),
        vec![theirs],
        "their Bears and nothing else: not their land, and not your Angel",
    );
}

/// "Any counters on the exiled permanent will cease to exist. When the card
/// returns to the battlefield, it will be a new object with no connection to
/// the card that was exiled." The prisoner comes back the size it was
/// printed.
#[test]
fn the_prisoner_leaves_its_counters_behind() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == bears)
        .expect("it is there")
        .add_counters(CounterKind::PlusOnePlusOne, 3);
    let grown = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears)
        .expect("it is there");
    assert_eq!(
        (game.power(grown), game.toughness(grown)),
        (Some(5), Some(5)),
        "a 2/2 with three counters on it",
    );

    let prison = game
        .put_onto_battlefield(PlayerId::One, cards::STATIC_PRISON)
        .expect("cataloged");
    settle_paying(&mut game, true);
    drain_pending(&mut game);
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != bears),
        "the Bears is exiled",
    );

    game.capture_battlefield_triggers(&CommittedTriggerEvent::StepBegins {
        step: TurnStepDef::PrecombatMain,
        player: PlayerId::One,
    });
    game.finish_rules_procedure();
    settle_paying(&mut game, false);
    drain_pending(&mut game);
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != prison),
        "the Prison went unpaid for",
    );

    let returned = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS)
        .expect("and the prisoner walked out");
    assert_eq!(
        returned.counters(CounterKind::PlusOnePlusOne),
        0,
        "the counters ceased to exist while it was gone",
    );
    assert_eq!(
        (game.power(returned), game.toughness(returned)),
        (Some(2), Some(2)),
        "so what came back is the printed 2/2",
    );
}
