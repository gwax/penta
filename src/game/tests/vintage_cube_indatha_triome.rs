//! Indatha Triome: Plains, Swamp and Forest on one nonbasic land, which is
//! four of the five Zendikar fetches finding it and one of them not.

use super::*;

/// Player One with `fetch` out and an Indatha Triome alone in the library.
fn staged(fetch: CardDefinitionId) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(63_100, cards::INDATHA_TRIOME, PlayerId::One));
    let source = game
        .put_onto_battlefield(PlayerId::One, fetch)
        .expect("cataloged");
    (game, source)
}

/// Cracks the fetch and hands back the search question, if one is asked at
/// all: a search with nothing to find resolves without asking.
fn crack(game: &mut Game, source: GameObjectId) -> Option<DecisionObservation> {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
        })
        .expect("the fetch ability is offered");
    game.apply(PlayerId::One, action).expect("it is cracked");
    pass_priority_pair(game);
    game.observe(PlayerId::One).decision
}

/// A fetch reads land types, not the basic supertype: every fetch that names
/// one of the Triome's three finds it, and Scalding Tarn, which names
/// neither an Island nor a Mountain it has, does not.
#[test]
fn four_of_the_five_fetches_find_it() {
    for fetch in [
        cards::ARID_MESA,
        cards::MARSH_FLATS,
        cards::MISTY_RAINFOREST,
        cards::VERDANT_CATACOMBS,
    ] {
        let (mut game, source) = staged(fetch);
        let offered = crack(&mut game, source)
            .unwrap_or_else(|| panic!("{fetch:?} asks which land to take"))
            .options
            .iter()
            .filter_map(|option| {
                option
                    .card
                    .and_then(|(_, characteristics)| characteristics.card_definition())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            offered,
            vec![cards::INDATHA_TRIOME],
            "{fetch:?} names a type the Triome carries",
        );
    }

    let (mut game, source) = staged(cards::SCALDING_TARN);
    assert!(
        crack(&mut game, source).is_none(),
        "an Island or a Mountain is what the Tarn wants, and it has neither",
    );
    assert_eq!(
        game.players[0].library.len(),
        1,
        "so the Triome stays where it was, and the life was spent for nothing",
    );
}

/// The fetch says to put the land onto the battlefield, and the Triome says
/// it enters tapped. The Triome's own clause is the one that speaks about
/// how it enters, so the life is paid and the land still comes down tapped.
#[test]
fn the_fetched_triome_still_enters_tapped() {
    let (mut game, source) = staged(cards::MARSH_FLATS);
    let life = game.players[0].life;

    let search = crack(&mut game, source).expect("the search is asking");
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: search.id,
            options: vec![search.options[0].id],
        },
    )
    .expect("the Triome is taken");
    drain_pending(&mut game);

    let triome = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::INDATHA_TRIOME)
        .expect("it arrived");
    assert!(triome.tapped, "tapped, for all that the fetch cost a life");
    assert_eq!(game.players[0].life, life - 1);
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::MARSH_FLATS),
        "and the fetch sacrificed itself to do it",
    );
}
