//! Verdant Catacombs: the black-green fetch, and what "a Swamp or Forest
//! card" reaches for.

use super::*;

/// The Catacombs on the battlefield with `library` beneath it, and `theirs`
/// in the other player's library.
fn staged(library: &[CardDefinitionId], theirs: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].library.clear();
    game.players[1].library.clear();
    let fetch = game
        .put_onto_battlefield(PlayerId::One, cards::VERDANT_CATACOMBS)
        .expect("cataloged");
    for (index, definition) in library.iter().enumerate() {
        game.players[0].library.push(card(
            81_000 + u32::try_from(index).expect("a handful"),
            *definition,
            PlayerId::One,
        ));
    }
    for (index, definition) in theirs.iter().enumerate() {
        game.players[1].library.push(card(
            81_100 + u32::try_from(index).expect("a handful"),
            *definition,
            PlayerId::Two,
        ));
    }
    drain_pending(&mut game);
    (game, fetch)
}

/// Cracks it and reports what the search offers, by definition.
fn offered(game: &mut Game, fetch: GameObjectId) -> Vec<CardDefinitionId> {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == fetch))
        .expect("a tap, a life, and the land itself");
    game.apply(PlayerId::One, action).expect("it activates");
    pass_priority_pair(game);
    game.observe(PlayerId::One)
        .decision
        .expect("the search asks")
        .options
        .iter()
        .filter_map(|option| {
            option
                .card
                .and_then(|(_, characteristics)| characteristics.card_definition())
        })
        .collect()
}

/// "A Swamp or Forest card" is a disjunction: each type answers it on its
/// own, and a land carrying neither answers it not at all. The cycle's other
/// test only ever offers a Bayou, which is both at once.
#[test]
fn either_named_type_alone_is_enough() {
    let (mut game, fetch) = staged(&[cards::ISLAND, cards::SWAMP, cards::FOREST], &[]);

    let mut found = offered(&mut game, fetch);
    found.sort_unstable();
    let mut expected = vec![cards::SWAMP, cards::FOREST];
    expected.sort_unstable();

    assert_eq!(found, expected, "the Island is neither of them");
}

/// "Search your library": theirs is a different library, however good the
/// land sitting in it would be. With nothing of yours to find, the fetch
/// still pays for itself and their Bayou stays where it is.
#[test]
fn it_searches_only_your_own_library() {
    let (mut game, fetch) = staged(&[cards::ISLAND], &[cards::BAYOU]);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == fetch))
        .expect("a tap, a life, and the land itself");
    game.apply(PlayerId::One, action).expect("it activates");
    pass_priority_pair(&mut game);
    if let Some(decision) = game.observe(PlayerId::One).decision {
        assert!(
            decision.options.is_empty(),
            "nothing of yours answers the search",
        );
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: decision.id,
                options: Vec::new(),
            },
        )
        .expect("taking nothing is allowed");
    }
    drain_pending(&mut game);

    assert!(
        game.battlefield.is_empty(),
        "the fetch sacrificed itself and found nothing to replace it",
    );
    assert_eq!(game.players[0].life, 19, "the life was paid all the same");
    assert_eq!(
        game.players[1]
            .library
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::BAYOU],
        "and their Bayou is still sitting where it was",
    );
}
