//! Concealed Courtyard: the white-black fastland, and what its clause counts.
//!
//! The cycle's boundary -- untapped through two other lands and tapped from
//! three -- both its colours, and a fastland put onto the battlefield by an
//! effect are pinned in `vintage_cube_lands` and
//! `vintage_cube_primeval_titan`. What is here is the word the clause turns
//! on: it counts *lands*, and it counts them by type rather than by what
//! they are otherwise doing.

use super::*;

/// Player One with `board` down, then a Courtyard played onto it; reports
/// whether the Courtyard arrived tapped.
fn courtyard_arrives_tapped(board: &[CardDefinitionId]) -> bool {
    let mut game = ready_game();
    game.battlefield.clear();
    for definition in board {
        game.put_onto_battlefield(PlayerId::One, *definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    let courtyard = game
        .put_onto_battlefield(PlayerId::One, cards::CONCEALED_COURTYARD)
        .expect("cataloged");
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == courtyard)
        .expect("it entered")
        .tapped
}

/// "Two or fewer other *lands*": a board of artifacts and creatures is not a
/// board of lands, however crowded it is.
#[test]
fn nonlands_are_not_counted_however_many_there_are() {
    assert!(
        !courtyard_arrives_tapped(&[
            cards::BLACK_LOTUS,
            cards::MOX_SAPPHIRE,
            cards::MOX_RUBY,
            cards::GRIZZLY_BEARS,
            cards::SERRA_ANGEL,
        ]),
        "five permanents and no lands leaves it untapped",
    );
    assert!(
        courtyard_arrives_tapped(&[
            cards::BLACK_LOTUS,
            cards::FOREST,
            cards::ISLAND,
            cards::SWAMP,
        ]),
        "and the three lands among them are what tap it",
    );
}

/// A land that is also a creature is still a land: three Dryad Arbors are
/// three other lands, and the Courtyard behind them comes in tapped.
#[test]
fn a_land_creature_counts_as_a_land() {
    assert!(
        !courtyard_arrives_tapped(&[cards::DRYAD_ARBOR, cards::DRYAD_ARBOR]),
        "two of them is two other lands",
    );
    assert!(
        courtyard_arrives_tapped(&[cards::DRYAD_ARBOR, cards::DRYAD_ARBOR, cards::DRYAD_ARBOR,]),
        "and a third is a third, creature or not",
    );
}
