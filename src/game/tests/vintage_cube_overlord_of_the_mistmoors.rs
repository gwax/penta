//! Overlord of the Mistmoors: a seven-mana 6/6 that most decks would rather
//! cast for four and wait four turns for the body.

use super::*;

/// Player One holding an Overlord with enough mana for either price.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let overlord = game
        .build_zone(PlayerId::One, &[cards::OVERLORD_OF_THE_MISTMOORS])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = overlord.id;
    game.players[0].hand.push(overlord);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 7);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, id)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .take(decision.minimum.max(1))
                .map(|option| option.id)
                .collect();
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
            return;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            return;
        }
    }
}

/// Casts it, taking the cheaper price when `impending` is set.
fn cast(game: &mut Game, overlord: GameObjectId, impending: bool) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == overlord && choices.costs().alternative().is_some() == impending)
        })
        .unwrap_or_else(|| panic!("it is castable (impending: {impending})"));
    game.apply(PlayerId::One, action).expect("it is castable");
    settle(game);
}

fn on_battlefield(game: &Game) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::OVERLORD_OF_THE_MISTMOORS)
}

fn is_a_creature(game: &Game) -> bool {
    on_battlefield(game).is_some_and(|permanent| {
        game.permanent_types(permanent)
            .is_some_and(|types| types.contains(CardType::Creature))
    })
}

fn time_counters(game: &Game) -> u16 {
    on_battlefield(game).map_or(0, |permanent| {
        permanent.counters(CounterKind::named("time"))
    })
}

fn insects(game: &Game) -> Vec<&Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Token)
        .collect()
}

/// Runs Player One's end step.
fn end_step(game: &mut Game) {
    game.active_player = PlayerId::One;
    game.step = Step::PostcombatMain;
    game.advance_step();
    settle(game);
}

/// Casts a Phantasmal Image and answers what it may enter as, returning the
/// permanents it was offered as copies without taking any of them.
fn copy_offers(game: &mut Game) -> Vec<GameObjectId> {
    let image = game
        .build_zone(PlayerId::One, &[cards::PHANTASMAL_IMAGE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let image_id = image.id;
    game.players[0].hand.push(image);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);
    // A creature spell wants a main phase, which an end step is not.
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == image_id))
        .expect("the Image is castable");
    game.apply(PlayerId::One, cast).expect("it is cast");
    let mut offered = Vec::new();
    for _ in 0..8 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            offered = decision
                .options
                .iter()
                .filter_map(|option| option.card.map(|(id, _)| id))
                .collect();
            // Enter as itself, which is a 0/0 that dies at once and leaves
            // the board as it was.
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: vec![decision.options[0].id],
                },
            )
            .expect("entering as itself is offered");
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    settle(game);
    game.check_state_based_actions();
    offered
}

/// Either price makes the Insects: the trigger watches the permanent, not
/// the creature.
#[test]
fn it_makes_two_fliers_as_it_enters() {
    let (mut game, overlord) = staged();

    cast(&mut game, overlord, true);

    let made = insects(&game);
    assert_eq!(made.len(), 2, "two Insects");
    assert_eq!(game.power(made[0]), Some(2));
    assert_eq!(game.toughness(made[0]), Some(1));
    assert!(game.has_flying(made[0]), "with flying");
}

/// The impending price gets four counters and no body.
#[test]
fn impending_enters_with_counters_and_no_body() {
    let (mut game, overlord) = staged();

    cast(&mut game, overlord, true);

    assert_eq!(time_counters(&game), 4);
    assert!(
        !is_a_creature(&game),
        "the enchantment is here; the creature is not",
    );
}

/// The printed price gets the 6/6 at once.
#[test]
fn hard_cast_it_is_a_creature_at_once() {
    let (mut game, overlord) = staged();

    cast(&mut game, overlord, false);

    assert!(is_a_creature(&game), "seven mana buys the body");
    assert_eq!(time_counters(&game), 0);
    let body = on_battlefield(&game).expect("it is here");
    assert_eq!(game.power(body), Some(6));
}

/// A counter comes off at each of your end steps, and the body arrives when
/// the last one goes.
#[test]
fn the_counters_come_off_one_end_step_at_a_time() {
    let (mut game, overlord) = staged();
    cast(&mut game, overlord, true);

    for remaining in [3, 2, 1] {
        end_step(&mut game);
        assert_eq!(time_counters(&game), remaining);
        assert!(!is_a_creature(&game), "still no body at {remaining}");
    }

    end_step(&mut game);

    assert_eq!(time_counters(&game), 0);
    assert!(is_a_creature(&game), "the last counter buys the body");
}

/// "You can cast that spell for its impending cost only when you could
/// normally cast that creature spell." The cheaper price is an alternative
/// cost, not flash: it waits for your own main phase with an empty stack
/// exactly as the printed price does.
#[test]
fn the_impending_price_is_still_a_creature_spell() {
    let (mut game, overlord) = staged();
    let impending = |game: &Game| {
        game.legal_actions(PlayerId::One).into_iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if card == overlord && choices.costs().alternative().is_some())
        })
    };
    assert!(impending(&game), "your own main phase is the window");

    game.step = Step::Upkeep;
    assert!(!impending(&game), "an upkeep is not");

    game.step = Step::PrecombatMain;
    game.active_player = PlayerId::Two;
    assert!(!impending(&game), "and neither is their turn");

    game.active_player = PlayerId::One;
    game.stack
        .push(spell(88_400, cards::LIGHTNING_BOLT, PlayerId::Two, 0));
    assert!(
        !impending(&game),
        "nor is a main phase with something already on the stack",
    );
}

/// What suppresses the body is a type-layer effect, so it is not only the
/// Overlord's own attacks that notice: an impending Overlord is not a
/// creature, and a copy effect that names one cannot name it. The Insects it
/// made are on offer the whole time, and the Overlord joins them once the
/// last time counter goes.
#[test]
fn nothing_can_copy_it_while_it_is_still_impending() {
    let (mut game, overlord) = staged();
    cast(&mut game, overlord, true);
    let insects = insects(&game)
        .iter()
        .map(|permanent| permanent.card.id)
        .collect::<Vec<_>>();
    assert_eq!(insects.len(), 2, "the enters trigger fired either way");

    assert_eq!(
        copy_offers(&mut game),
        insects,
        "the Insects are copyable and the Overlord waiting behind them is not",
    );

    for _ in 0..4 {
        end_step(&mut game);
    }
    assert!(is_a_creature(&game), "the last counter came off");

    let body = on_battlefield(&game).expect("he is still there").card.id;
    let offers = copy_offers(&mut game);
    assert!(
        offers.contains(&body),
        "and now there is a creature there to copy",
    );
}

/// "Whenever this permanent enters or attacks": one printed ability with two
/// ways in, and every test above takes the first. Swinging with the 6/6
/// makes two more Insects.
#[test]
fn attacking_makes_two_more_insects() {
    let (mut game, overlord) = staged();
    cast(&mut game, overlord, false);
    assert_eq!(insects(&game).len(), 2, "two from the way in");
    let body = on_battlefield(&game).expect("the 6/6 is here").card.id;
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }

    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(body, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    settle(&mut game);
    drain_pending(&mut game);

    assert_eq!(
        insects(&game).len(),
        4,
        "two more for the attack, and the first two are still there",
    );
    assert!(
        insects(&game).iter().all(|insect| game.has_flying(insect)),
        "all of them fliers",
    );
}

/// An Overlord still counting down is not a creature, so there is nothing to
/// declare: the attack half waits for the body the same way the blocking
/// does.
#[test]
fn an_impending_overlord_cannot_attack_for_its_own_trigger() {
    let (mut game, overlord) = staged();
    cast(&mut game, overlord, true);
    let enchantment = on_battlefield(&game).expect("it is here").card.id;
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }

    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::DeclareAttacker { attacker, .. } if *attacker == enchantment)
        }),
        "an enchantment with time counters is no attacker",
    );
    assert_eq!(insects(&game).len(), 2, "so it made nothing further");
}
