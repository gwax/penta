//! Esika's Chariot: four mana that brings its own crew, and copies one of
//! them every time it swings.

use super::*;

/// The Chariot on the battlefield with its enters trigger already answered.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let chariot = game
        .put_onto_battlefield(PlayerId::One, cards::ESIKA_S_CHARIOT)
        .expect("cataloged");
    drain_pending(&mut game);
    settle(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [4, 4];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, chariot)
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

fn cats(game: &Game) -> Vec<&Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| {
            is_token_with(
                permanent,
                tokens::creature(&["Cat"], &[ManaColor::Green], 2, 2),
            )
        })
        .collect()
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

fn is_creature(game: &Game, id: GameObjectId) -> bool {
    game.permanent_types(permanent(game, id))
        .is_some_and(|types| types.contains(CardType::Creature))
}

/// Two Cats on the way in, which are exactly enough power to crew it.
#[test]
fn it_brings_two_cats() {
    let (game, _chariot) = staged();

    let cats = cats(&game);
    assert_eq!(cats.len(), 2, "two of them");
    assert_eq!(game.power(cats[0]), Some(2), "each a 2/2");
}

/// Crew 4: the two Cats it came with are exactly four power between them.
#[test]
fn its_own_cats_crew_it() {
    let (mut game, chariot) = staged();
    assert!(!is_creature(&game, chariot), "a Vehicle until crewed");

    let crew = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == chariot),
        )
        .expect("four power is on the battlefield");
    game.apply(PlayerId::One, crew).expect("it crews");
    settle(&mut game);

    assert!(is_creature(&game, chariot));
    assert_eq!(game.power(permanent(&game, chariot)), Some(4));
    assert!(
        cats(&game).iter().all(|cat| cat.tapped),
        "and both Cats tapped to do it",
    );
}

/// Attacking copies a token: another Cat, which is another two power for
/// next turn's crew.
#[test]
fn attacking_copies_one_of_them() {
    let (mut game, chariot) = staged();
    let crew = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == chariot),
        )
        .expect("four power is on the battlefield");
    game.apply(PlayerId::One, crew).expect("it crews");
    settle(&mut game);

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: chariot,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("a crewed Chariot may attack");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration is complete");
    settle(&mut game);

    assert_eq!(cats(&game).len(), 3, "the attack made a third Cat");
}

/// "Target token you control": the Chariot is not a token and neither is
/// anything else on an empty board, so only the Cats are on offer.
#[test]
fn only_tokens_are_legal_targets() {
    let (mut game, chariot) = staged();
    game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    let cats = cats(&game)
        .iter()
        .map(|cat| cat.card.id)
        .collect::<Vec<_>>();
    let crew = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == chariot),
        )
        .expect("four power is on the battlefield");
    game.apply(PlayerId::One, crew).expect("it crews");
    settle(&mut game);

    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: chariot,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("a crewed Chariot may attack");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration is complete");
    for _ in 0..8 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    let mut offered = game
        .pending_decisions
        .first()
        .expect("the attack trigger is asking")
        .observation
        .options
        .iter()
        .filter_map(|option| option.card.map(|(id, _)| id))
        .collect::<Vec<_>>();
    offered.sort_unstable();
    let mut expected = cats;
    expected.sort_unstable();
    assert_eq!(offered, expected, "the Cats and nothing else");
}
