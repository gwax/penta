//! Arid Mesa: a Mountain or a Plains, and one life for the privilege.
//!
//! What a fetchland costs, what happens when it finds nothing, and what it
//! does to the land drop are covered across the fetchland family. What this
//! checks is the "or": a card carrying one of the two named types is as good
//! as a dual carrying both, which is the only way the pair is ever really
//! used.

use super::*;

/// The Mesa on the battlefield with `library` to search, and the crack
/// already activated and waiting on its search decision.
fn cracked(library: &[CardDefinitionId]) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    let mesa = game
        .put_onto_battlefield(PlayerId::One, cards::ARID_MESA)
        .expect("cataloged");
    drain_pending(&mut game);
    game.players[PlayerId::One.index()].library.clear();
    for (index, definition) in library.iter().enumerate() {
        game.players[PlayerId::One.index()].library.push(card(
            97_000 + u32::try_from(index).expect("a short library"),
            *definition,
            PlayerId::One,
        ));
    }
    game.priority = PlayerId::One;

    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == mesa))
        .expect("a tap, a life and itself pays for the search");
    game.apply(PlayerId::One, crack).expect("it activates");
    pass_priority_pair(&mut game);
    game
}

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

/// "A Mountain or Plains card": each basic on its own answers the search,
/// and both are offered at once when both are there.
#[test]
fn either_named_type_is_enough_on_its_own() {
    let game = cracked(&[cards::ISLAND, cards::MOUNTAIN, cards::PLAINS]);

    let mut expected = vec![cards::MOUNTAIN, cards::PLAINS];
    expected.sort_unstable();
    assert_eq!(
        offered(&game),
        expected,
        "one type each is enough; the Island has neither",
    );
}

/// And whichever is taken is the one that arrives -- untapped, with the life
/// already paid and the Mesa already in the graveyard.
#[test]
fn whichever_is_named_is_the_one_that_comes() {
    for wanted in [cards::MOUNTAIN, cards::PLAINS] {
        let mut game = cracked(&[cards::MOUNTAIN, cards::PLAINS]);
        assert_eq!(
            game.players[PlayerId::One.index()].life,
            19,
            "the life is paid as a cost, before anything is found",
        );

        let decision = game
            .observe(PlayerId::One)
            .decision
            .expect("the search asks which land to take");
        let option = decision
            .options
            .iter()
            .find(|option| {
                option
                    .card
                    .and_then(|(_, characteristics)| characteristics.card_definition())
                    == Some(wanted)
            })
            .map_or_else(|| panic!("{wanted:?} is on offer"), |option| option.id);
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: decision.id,
                options: vec![option],
            },
        )
        .expect("taking one of the two is legal");
        drain_pending(&mut game);

        let found = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.definition == wanted)
            .unwrap_or_else(|| panic!("{wanted:?} arrived"));
        assert!(!found.tapped, "the Mesa says nothing about tapped");
        assert!(
            !game
                .battlefield
                .iter()
                .any(|permanent| permanent.card.definition == cards::ARID_MESA),
            "and it sacrificed itself to get there",
        );
        assert_eq!(
            game.players[PlayerId::One.index()].library.len(),
            1,
            "the one it did not take is still in the library",
        );
    }
}
