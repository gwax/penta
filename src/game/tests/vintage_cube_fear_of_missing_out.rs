//! Fear of Missing Out: a two-mana body that fills its own graveyard and,
//! once the graveyard is deep enough, buys a second attack.

use super::*;

/// Him on the battlefield since last turn, with `graveyard` behind him.
fn staged(graveyard: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    for (index, definition) in graveyard.iter().enumerate() {
        game.players[0].graveyard.push(card(
            260_000 + u32::try_from(index).expect("few cards"),
            *definition,
            PlayerId::One,
        ));
    }
    let fomo = game
        .put_onto_battlefield(PlayerId::One, cards::FEAR_OF_MISSING_OUT)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.priority = PlayerId::One;
    (game, fomo)
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
                .map(|option| option.id)
                .take(decision.minimum.max(1))
                .collect();
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

/// A graveyard with four card types in it. An artifact, a creature, an
/// instant, and a land are four; the Nightmare's own discard is often the
/// fourth in practice.
const FOUR_TYPES: [CardDefinitionId; 4] = [
    cards::MANIFOLD_KEY,
    cards::GRIZZLY_BEARS,
    cards::LIGHTNING_BOLT,
    cards::MOUNTAIN,
];

/// Attacks, naming `wanted` wherever the trigger asks for a target.
fn attack_with(game: &mut Game, attacker: GameObjectId, wanted: Option<GameObjectId>) {
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(attacker, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options: Vec<_> = decision
                .options
                .iter()
                .filter(|option| {
                    wanted.is_some_and(|wanted| {
                        option.card.is_some_and(|(object, _)| object == wanted)
                    })
                })
                .map(|option| option.id)
                .take(1)
                .collect();
            let options = if options.len() < decision.minimum {
                decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(decision.minimum)
                    .collect()
            } else {
                options
            };
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

/// He loots on the way in.
#[test]
fn he_loots_when_he_enters() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[0].library.clear();
    let drawn = game
        .build_zone(PlayerId::One, &[cards::SERRA_ANGEL])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    game.players[0].library.push(drawn);
    game.players[0]
        .hand
        .push(card(260_100, cards::MOUNTAIN, PlayerId::One));

    game.put_onto_battlefield(PlayerId::One, cards::FEAR_OF_MISSING_OUT)
        .expect("cataloged");
    settle(&mut game);

    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::SERRA_ANGEL],
        "the land went and a card came",
    );
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::MOUNTAIN],
    );
}

/// With delirium, attacking untaps a creature and buys a second combat.
#[test]
fn delirium_untaps_a_creature_and_repeats_combat() {
    let (mut game, fomo) = staged(&FOUR_TYPES);
    let lions = game
        .put_onto_battlefield(PlayerId::One, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.tap_permanent(lions);

    attack_with(&mut game, fomo, Some(lions));

    assert!(
        !game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == lions)
            .expect("the Lions are still there")
            .tapped,
        "the trigger untapped what it named",
    );

    let mut seen_second_combat = false;
    for _ in 0..40 {
        if game.step == Step::PostcombatMain {
            break;
        }
        if game.step == Step::EndOfCombat {
            game.advance_step();
            if game.step == Step::BeginningOfCombat {
                seen_second_combat = true;
            }
            continue;
        }
        game.advance_step();
    }
    assert!(seen_second_combat, "an additional combat phase happened");
}

/// Without delirium the trigger does nothing at all.
#[test]
fn three_card_types_are_not_enough() {
    let (mut game, fomo) = staged(&[cards::GRIZZLY_BEARS, cards::LIGHTNING_BOLT, cards::MOUNTAIN]);
    let lions = game
        .put_onto_battlefield(PlayerId::One, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.tap_permanent(lions);

    attack_with(&mut game, fomo, Some(lions));

    assert!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == lions)
            .expect("still there")
            .tapped,
        "three types is not delirium",
    );

    let mut seen_second_combat = false;
    for _ in 0..40 {
        if game.step == Step::PostcombatMain {
            break;
        }
        if game.step == Step::EndOfCombat {
            game.advance_step();
            if game.step == Step::BeginningOfCombat {
                seen_second_combat = true;
            }
            continue;
        }
        game.advance_step();
    }
    assert!(!seen_second_combat, "and no extra combat either");
}

/// "For the first time each turn": the second attack of the turn does not
/// buy a third combat.
#[test]
fn only_the_first_attack_each_turn_counts() {
    let (mut game, fomo) = staged(&FOUR_TYPES);

    attack_with(&mut game, fomo, Some(fomo));
    // Out of combat and back in, which is what the extra phase gives him.
    game.clear_combat();
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == fomo)
    {
        permanent.tapped = false;
    }
    attack_with(&mut game, fomo, Some(fomo));

    assert!(
        game.pending_triggers.is_empty() && game.stack.is_empty(),
        "the second attack of the turn triggers nothing",
    );
}

/// "If the target creature is an illegal target as the ability tries to
/// resolve, it won't resolve and none of its effects will happen. There
/// won't be an additional combat phase." The untap and the extra combat are
/// one ability, so losing the target loses both.
#[test]
fn a_dead_target_costs_the_extra_combat_too() {
    let (mut game, fomo) = staged(&FOUR_TYPES);
    let lions = game
        .put_onto_battlefield(PlayerId::One, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.tap_permanent(lions);

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(fomo, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();

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
        .expect("the trigger asks who to untap");
    let option = decision
        .options
        .iter()
        .find(|option| option.card.is_some_and(|(object, _)| object == lions))
        .expect("the Lions are a legal target")
        .id;
    game.apply(
        decision.player,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![option],
        },
    )
    .expect("naming them is legal");

    // The named creature is gone before the ability resolves.
    game.move_permanents_to_graveyard(&[lions]);
    settle(&mut game);

    let mut seen_second_combat = false;
    let mut reached_postcombat = false;
    for _ in 0..40 {
        if game.step == Step::PostcombatMain {
            reached_postcombat = true;
            break;
        }
        if game.step == Step::EndOfCombat {
            game.advance_step();
            if game.step == Step::BeginningOfCombat {
                seen_second_combat = true;
            }
            continue;
        }
        game.advance_step();
    }
    assert!(
        reached_postcombat,
        "combat ended into the ordinary postcombat main phase",
    );
    assert!(
        !seen_second_combat,
        "and the ability that would have bought a second one never resolved",
    );
}
