//! Orcish Lumberjack: a land for three mana, and the body was never the
//! point.

use super::*;
use crate::ManaSplit;

/// The Orc on the battlefield since last turn, with `lands` beside it.
fn staged(lands: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    for definition in lands {
        game.put_onto_battlefield(PlayerId::One, *definition)
            .expect("cataloged");
    }
    let orc = game
        .put_onto_battlefield(PlayerId::One, cards::ORCISH_LUMBERJACK)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    drain_pending(&mut game);
    (game, orc)
}

/// The splits the Orc offers, each as its (red, green) pair.
fn offered_splits(game: &Game, orc: GameObjectId) -> Vec<(u16, u16)> {
    let mut splits = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateManaAbility {
                source,
                combination,
                color,
                ..
            } if source == orc => Some(combination.unwrap_or_else(|| {
                let mut split = ManaSplit::empty();
                split.add(color, 3);
                split
            })),
            _ => None,
        })
        .map(|split| (split.get(ManaColor::Red), split.get(ManaColor::Green)))
        .collect::<Vec<_>>();
    splits.sort_unstable();
    splits.dedup();
    splits
}

fn activation(game: &Game, orc: GameObjectId, red: u16, green: u16) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateManaAbility {
                source,
                combination,
                color,
                ..
            } => {
                let split = combination.unwrap_or_else(|| {
                    let mut split = ManaSplit::empty();
                    split.add(*color, 3);
                    split
                });
                *source == orc
                    && split.get(ManaColor::Red) == red
                    && split.get(ManaColor::Green) == green
            }
            _ => false,
        })
}

fn lands_of(game: &Game, definition: CardDefinitionId) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == definition)
        .count()
}

/// Every way to split three between the two colours is its own activation.
#[test]
fn it_offers_every_split_of_three() {
    let (game, orc) = staged(&[cards::FOREST]);

    assert_eq!(
        offered_splits(&game, orc),
        vec![(0, 3), (1, 2), (2, 1), (3, 0)],
        "three mana in any combination of red and green",
    );
}

/// The Forest is the cost: it is gone, and so is the tap.
#[test]
fn it_eats_a_forest_for_three_mana() {
    let (mut game, orc) = staged(&[cards::FOREST, cards::MOUNTAIN]);

    let action = activation(&game, orc, 2, 1).expect("two red and a green is on offer");
    game.apply(PlayerId::One, action).expect("it activates");

    assert_eq!(game.players[0].mana_pool.red, 2);
    assert_eq!(game.players[0].mana_pool.green, 1);
    assert_eq!(lands_of(&game, cards::FOREST), 0, "the Forest is gone");
    assert_eq!(lands_of(&game, cards::MOUNTAIN), 1, "and only the Forest");
    assert!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == orc)
            .expect("the Orc is still there")
            .tapped,
    );
}

/// No Forest, no ability: the sacrifice is part of the cost.
#[test]
fn without_a_forest_it_does_nothing() {
    let (game, orc) = staged(&[cards::MOUNTAIN, cards::ISLAND]);

    assert!(
        offered_splits(&game, orc).is_empty(),
        "a Mountain is not a Forest",
    );
}

/// "A Forest" is the land type, not the card name: a Taiga is a Forest.
#[test]
fn a_dual_land_with_the_type_counts() {
    let (mut game, orc) = staged(&[cards::TAIGA]);

    let action = activation(&game, orc, 3, 0).expect("the Taiga pays for it");
    game.apply(PlayerId::One, action).expect("it activates");

    assert_eq!(game.players[0].mana_pool.red, 3);
    assert_eq!(lands_of(&game, cards::TAIGA), 0, "the Taiga was sacrificed");
}

/// It taps, so an Orc that just arrived cannot do it yet.
#[test]
fn a_freshly_played_orc_has_to_wait() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.put_onto_battlefield(PlayerId::One, cards::FOREST)
        .expect("cataloged");
    let orc = game
        .put_onto_battlefield(PlayerId::One, cards::ORCISH_LUMBERJACK)
        .expect("cataloged");
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    // Arrived this turn, which is what summoning sickness reads.
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == orc)
        .expect("the Orc is there")
        .entered_controller_turn = game.turns_started[PlayerId::One.index()];
    drain_pending(&mut game);

    assert!(
        offered_splits(&game, orc).is_empty(),
        "summoning sickness stops the tap",
    );
}
