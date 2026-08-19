//! Lands cataloged for the Vintage Cube pool.
//!
//! A triome is three basic land types, a tapped entry, and cycling. Nothing
//! in it is new, so what these check is the composition: that the subtypes
//! really do produce all three colors without a printed mana clause, that the
//! land arrives tapped, and that the cycling ability is the one the card
//! prints rather than the one its subtypes might suggest.

use super::*;

/// Every triome, with the three colors its subtypes grant.
const TRIOMES: [(CardDefinitionId, [ManaColor; 3]); 10] = [
    (
        cards::INDATHA_TRIOME,
        [ManaColor::White, ManaColor::Black, ManaColor::Green],
    ),
    (
        cards::KETRIA_TRIOME,
        [ManaColor::Green, ManaColor::Blue, ManaColor::Red],
    ),
    (
        cards::RAUGRIN_TRIOME,
        [ManaColor::Blue, ManaColor::Red, ManaColor::White],
    ),
    (
        cards::SAVAI_TRIOME,
        [ManaColor::Red, ManaColor::White, ManaColor::Black],
    ),
    (
        cards::ZAGOTH_TRIOME,
        [ManaColor::Black, ManaColor::Green, ManaColor::Blue],
    ),
    (
        cards::JETMIRS_GARDEN,
        [ManaColor::Red, ManaColor::Green, ManaColor::White],
    ),
    (
        cards::RAFFINES_TOWER,
        [ManaColor::White, ManaColor::Blue, ManaColor::Black],
    ),
    (
        cards::SPARAS_HEADQUARTERS,
        [ManaColor::Green, ManaColor::White, ManaColor::Blue],
    ),
    (
        cards::XANDERS_LOUNGE,
        [ManaColor::Blue, ManaColor::Black, ManaColor::Red],
    ),
    (
        cards::ZIATORAS_PROVING_GROUND,
        [ManaColor::Black, ManaColor::Red, ManaColor::Green],
    ),
];

#[test]
fn every_triome_enters_tapped_and_taps_for_each_of_its_three_colors() {
    for (definition, colors) in TRIOMES {
        let mut game = ready_game();
        let land = game
            .put_onto_battlefield(PlayerId::One, definition)
            .expect("cataloged");
        assert!(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == land)
                .expect("the triome entered")
                .tapped,
            "{definition:?} enters tapped",
        );

        for color in colors {
            let mut game = ready_game();
            let land = game
                .put_onto_battlefield(PlayerId::One, definition)
                .expect("cataloged");
            game.battlefield
                .iter_mut()
                .find(|permanent| permanent.card.id == land)
                .expect("the triome entered")
                .tapped = false;
            game.apply(
                PlayerId::One,
                Action::ActivateManaAbility {
                    source: land,
                    ability: mana_ability_for(&game, land, color),
                    color,
                    counters_removed: None,
                    cost_object: None,
                },
            )
            .unwrap_or_else(|error| panic!("{definition:?} makes {color:?}: {error}"));
            assert_eq!(
                game.players[PlayerId::One.index()].mana_pool.amount(color),
                1,
                "{definition:?} taps for {color:?}",
            );
        }
    }
}

#[test]
fn a_triome_cycles_from_hand_for_three_generic() {
    let mut game = ready_game();
    game.players[PlayerId::One.index()].hand.clear();
    let triome = card(41_000, cards::RAFFINES_TOWER, PlayerId::One);
    let triome_id = triome.id;
    game.players[PlayerId::One.index()].hand.push(triome);

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == triome_id)
        ),
        "cycling is not offered before the three mana is available",
    );

    game.players[PlayerId::One.index()].mana_pool.colorless = 3;
    let library_before = game.players[PlayerId::One.index()].library.len();
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == triome_id),
        )
        .expect("cycling is offered from hand");
    game.apply(PlayerId::One, action).expect("it is activated");

    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::RAFFINES_TOWER),
        "the discard is a cost",
    );
    pass_priority_pair(&mut game);
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        library_before - 1,
        "and the draw is what resolved",
    );
}
