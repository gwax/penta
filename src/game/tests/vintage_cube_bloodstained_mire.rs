//! Bloodstained Mire: a Swamp or a Mountain, and one life for the privilege.
//!
//! What a fetchland costs, what happens when it finds nothing, and what it
//! does to the land drop are covered across the fetchland family, and the
//! Onslaught cycle's per-member types in `tutors_and_fetch_lands`. That
//! check hands each fetch a dual carrying both of its named types at once,
//! which a search reading "and" would pass just as happily. What is here is
//! the "or", one basic at a time.

use super::*;

/// The Mire on the battlefield with `library` to search, and the crack
/// already activated and waiting on its search decision.
fn cracked(library: &[CardDefinitionId]) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    let mire = game
        .put_onto_battlefield(PlayerId::One, cards::BLOODSTAINED_MIRE)
        .expect("cataloged");
    drain_pending(&mut game);
    game.players[PlayerId::One.index()].library.clear();
    for (index, definition) in library.iter().enumerate() {
        game.players[PlayerId::One.index()].library.push(card(
            98_000 + u32::try_from(index).expect("a short library"),
            *definition,
            PlayerId::One,
        ));
    }
    game.priority = PlayerId::One;

    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == mire))
        .expect("a tap, a life and itself pays for the search");
    game.apply(PlayerId::One, crack).expect("it activates");
    pass_priority_pair(&mut game);
    game
}

/// What the pending search is offering.
fn offered(game: &Game) -> Vec<CardDefinitionId> {
    let mut found = game
        .observe(PlayerId::One)
        .decision
        .expect("the search asks which land to take")
        .options
        .iter()
        .filter_map(|option| {
            option
                .card
                .and_then(|(_, characteristics)| characteristics.card_definition())
        })
        .collect::<Vec<_>>();
    found.sort_unstable();
    found
}

/// "A Swamp or Mountain card": either basic on its own answers the search,
/// and a library holding both offers both.
#[test]
fn either_named_type_is_enough_on_its_own() {
    assert_eq!(
        offered(&cracked(&[cards::SWAMP])),
        vec![cards::SWAMP],
        "a Swamp alone",
    );
    assert_eq!(
        offered(&cracked(&[cards::MOUNTAIN])),
        vec![cards::MOUNTAIN],
        "and a Mountain alone",
    );

    let mut both = vec![cards::SWAMP, cards::MOUNTAIN];
    both.sort_unstable();
    assert_eq!(
        offered(&cracked(&[cards::SWAMP, cards::MOUNTAIN])),
        both,
        "and both when both are there",
    );
}

/// The other side of the "or": a basic of neither named type is no answer,
/// however many of them the library holds.
#[test]
fn a_basic_of_neither_type_is_never_offered() {
    let game = cracked(&[cards::ISLAND, cards::FOREST, cards::PLAINS, cards::SWAMP]);

    assert_eq!(
        offered(&game),
        vec![cards::SWAMP],
        "the one Swamp among four basics, and nothing beside it",
    );
}
