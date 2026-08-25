//! Relic of Sauron: a three-colour rock that turns into a card-advantage
//! engine once the mana is no longer the problem.

use super::*;
use crate::ManaSplit;

/// The Relic on the battlefield since last turn, with a library to draw
/// from and `mana` colourless available.
fn staged(mana: u16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..8 {
        game.players[0]
            .library
            .push(card(88_000 + index, cards::ISLAND, PlayerId::One));
    }
    let relic = game
        .put_onto_battlefield(PlayerId::One, cards::RELIC_OF_SAURON)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, mana);
    drain_pending(&mut game);
    (game, relic)
}

/// The splits the Relic offers, each as its (blue, black, red) triple.
fn offered_splits(game: &Game, relic: GameObjectId) -> Vec<(u16, u16, u16)> {
    let mut splits = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateManaAbility {
                source,
                combination,
                color,
                ..
            } if source == relic => Some(combination.unwrap_or_else(|| {
                let mut split = ManaSplit::empty();
                split.add(color, 2);
                split
            })),
            _ => None,
        })
        .map(|split| {
            (
                split.get(ManaColor::Blue),
                split.get(ManaColor::Black),
                split.get(ManaColor::Red),
            )
        })
        .collect::<Vec<_>>();
    splits.sort_unstable();
    splits.dedup();
    splits
}

fn settle(game: &mut Game) {
    for _ in 0..16 {
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

/// Every way to split two between the three colours is its own activation.
#[test]
fn it_offers_every_split_of_two() {
    let (game, relic) = staged(0);

    assert_eq!(
        offered_splits(&game, relic),
        vec![
            (0, 0, 2),
            (0, 1, 1),
            (0, 2, 0),
            (1, 0, 1),
            (1, 1, 0),
            (2, 0, 0),
        ],
        "two mana in any combination of the three",
    );
}

/// One activation can pay two different pips, which is the whole of "in any
/// combination".
#[test]
fn one_activation_can_make_two_colours() {
    let (mut game, relic) = staged(0);

    let mixed = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateManaAbility {
                source,
                combination: Some(split),
                ..
            } => {
                *source == relic
                    && split.get(ManaColor::Blue) == 1
                    && split.get(ManaColor::Red) == 1
            }
            _ => false,
        })
        .expect("a blue and a red is on offer");
    game.apply(PlayerId::One, mixed).expect("it activates");

    assert_eq!(game.players[0].mana_pool.blue, 1);
    assert_eq!(game.players[0].mana_pool.red, 1);
}

/// Three mana and the tap draws two and bins one.
#[test]
fn three_mana_loots_two_for_one() {
    let (mut game, relic) = staged(3);
    let hand = game.players[0].hand.len();

    let loot = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == relic))
        .expect("three mana buys the draw");
    game.apply(PlayerId::One, loot).expect("it activates");
    settle(&mut game);

    assert_eq!(
        game.players[0].hand.len(),
        hand + 1,
        "two drawn and one discarded",
    );
    assert_eq!(game.players[0].graveyard.len(), 1, "the discard happened");
}

/// The tap is shared: a Relic that made mana this turn cannot also draw.
#[test]
fn the_tap_pays_for_only_one_of_them() {
    let (mut game, relic) = staged(3);

    let mana = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateManaAbility { source, .. } if *source == relic))
        .expect("the mana ability is offered");
    game.apply(PlayerId::One, mana).expect("it taps");

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == relic)
        ),
        "tapped for mana, it cannot also loot",
    );
    assert!(offered_splits(&game, relic).is_empty(), "or tap again");
}

/// Without the three mana the loot is not on offer at all.
#[test]
fn the_loot_needs_its_three_mana() {
    let (game, relic) = staged(2);

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == relic)
        ),
        "two mana is not three",
    );
}
