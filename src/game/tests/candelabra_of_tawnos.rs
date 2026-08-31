//! A target count that is the X you paid.
//!
//! "Untap X target lands" links the two numbers: the declaration offers
//! exactly as many lands as the X paid for, so an X larger than the board
//! offers nothing at all rather than silently untapping fewer.

use super::*;

/// A Candelabra and `lands` tapped Islands, with `mana` colorless banked.
fn candelabra(lands: u32, mana: u8) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.turn = 5;
    game.turns_started[PlayerId::One.index()] = 5;
    let candelabra = creature(10_000, cards::CANDELABRA_OF_TAWNOS, PlayerId::One);
    let candelabra_id = candelabra.card.id;
    game.battlefield.push(candelabra);

    let mut island_ids = Vec::new();
    for index in 0..lands {
        let mut island = creature(10_100 + index, cards::ISLAND, PlayerId::One);
        island.tapped = true;
        island_ids.push(island.card.id);
        game.battlefield.push(island);
    }
    game.players[PlayerId::One.index()].mana_pool.colorless = u16::from(mana);
    game.priority = PlayerId::One;
    (game, candelabra_id, island_ids)
}

/// Every activation of the Candelabra currently on offer, as (X, target
/// count) pairs.
fn offered_shapes(game: &Game, candelabra: GameObjectId) -> Vec<(u16, usize)> {
    let mut shapes = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateAbility {
                source, x, targets, ..
            } if source == candelabra => {
                Some((x, targets.iter().map(|slot| slot.targets().len()).sum()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    shapes.sort_unstable();
    shapes.dedup();
    shapes
}

fn tapped(game: &Game, id: GameObjectId) -> bool {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("still there")
        .tapped
}

/// The heart of the card: the number of targets is never anything but X.
#[test]
fn every_offer_targets_exactly_as_many_lands_as_the_x_paid() {
    let (game, candelabra, _islands) = candelabra(3, 3);
    let shapes = offered_shapes(&game, candelabra);
    assert!(!shapes.is_empty(), "the ability is on offer");
    for (x, count) in shapes {
        assert_eq!(
            usize::from(u8::try_from(x).expect("small X")),
            count,
            "X={x} offered {count} targets",
        );
    }
}

/// Three lands on the battlefield, so X=3 is the largest offer even with more
/// mana available. An X of four has no legal declaration.
#[test]
fn an_x_larger_than_the_board_is_not_offered() {
    let (game, candelabra, _islands) = candelabra(3, 6);
    let largest = offered_shapes(&game, candelabra)
        .into_iter()
        .map(|(x, _)| x)
        .max()
        .expect("something is on offer");
    assert_eq!(largest, 3, "three lands is the ceiling, not the mana");
}

#[test]
fn it_untaps_exactly_the_lands_chosen() {
    let (mut game, candelabra, islands) = candelabra(3, 2);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, x: 2, targets, .. }
                if *source == candelabra
                    && targets.iter().any(|slot| slot.targets()
                        == [Target::Permanent(islands[0]), Target::Permanent(islands[1])]))
        })
        .expect("two of the three lands is a legal choice");
    game.apply(PlayerId::One, action)
        .expect("two mana is enough");
    drain_pending(&mut game);

    assert!(!tapped(&game, islands[0]));
    assert!(!tapped(&game, islands[1]));
    assert!(tapped(&game, islands[2]), "the untargeted land stayed down");
}

/// The Candelabra itself taps to pay, which is what stops it from untapping
/// the same land over and over for free.
#[test]
fn it_taps_itself_to_pay() {
    let (mut game, candelabra, _islands) = candelabra(3, 2);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == candelabra))
        .expect("on offer");
    game.apply(PlayerId::One, action).expect("legal");
    drain_pending(&mut game);

    assert!(tapped(&game, candelabra));
}

/// "Can be used on an untapped land." Nothing in the ability asks its
/// targets to be tapped; untapping one that already is does nothing and is
/// still a legal way to spend it.
#[test]
fn an_untapped_land_is_a_legal_target() {
    let (mut game, candelabra, islands) = candelabra(2, 1);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == islands[0])
    {
        permanent.tapped = false;
    }

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, x: 1, targets, .. }
                if *source == candelabra
                    && targets
                        .iter()
                        .any(|slot| slot.targets() == [Target::Permanent(islands[0])]))
        })
        .expect("a land that is already up is a land");
    game.apply(PlayerId::One, action).expect("one mana pays it");
    drain_pending(&mut game);

    assert!(!tapped(&game, islands[0]), "it is still up");
    assert!(
        tapped(&game, islands[1]),
        "and the tapped one was not chosen"
    );
}

/// "You may untap your opponent's lands if desired." The ability names
/// lands and not whose.
#[test]
fn it_will_untap_a_land_they_control() {
    let (mut game, candelabra, _islands) = candelabra(1, 1);
    let mut theirs = creature(10_200, cards::MOUNTAIN, PlayerId::Two);
    theirs.tapped = true;
    let theirs_id = theirs.card.id;
    game.battlefield.push(theirs);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, x: 1, targets, .. }
                if *source == candelabra
                    && targets
                        .iter()
                        .any(|slot| slot.targets() == [Target::Permanent(theirs_id)]))
        })
        .expect("their Mountain is a land like any other");
    game.apply(PlayerId::One, action).expect("one mana pays it");
    drain_pending(&mut game);

    assert!(!tapped(&game, theirs_id), "and it comes up for them");
}

/// "This is not a mana ability. It is a normal ability and it will resolve
/// along with other spells and abilities on the stack." So the lands are
/// still down while it waits there.
#[test]
fn it_waits_on_the_stack_like_anything_else() {
    let (mut game, candelabra, islands) = candelabra(1, 1);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, x: 1, targets, .. }
                if *source == candelabra
                    && targets
                        .iter()
                        .any(|slot| slot.targets() == [Target::Permanent(islands[0])]))
        })
        .expect("one mana for the one land is on offer");
    game.apply(PlayerId::One, action).expect("one mana pays it");

    assert_eq!(game.stack.len(), 1, "it is on the stack rather than done");
    assert!(
        tapped(&game, islands[0]),
        "and the land is down until it resolves",
    );

    drain_pending(&mut game);

    assert!(!tapped(&game, islands[0]), "which is when it comes up");
}
