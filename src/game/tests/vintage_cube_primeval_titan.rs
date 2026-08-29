//! Primeval Titan: six mana for two lands now and two more every attack,
//! and "up to two" is a ceiling rather than a quota.

use super::*;

/// The Titan arriving with `library` under Player One, top card last.
fn staged(library: &[(u32, CardDefinitionId)]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for (id, definition) in library {
        game.players[0]
            .library
            .push(card(*id, *definition, PlayerId::One));
    }
    let titan = game
        .put_onto_battlefield(PlayerId::One, cards::PRIMEVAL_TITAN)
        .expect("cataloged");
    (game, titan)
}

/// Passes priority until somebody is asked something.
fn ask(game: &mut Game) -> DecisionObservation {
    loop {
        if let Some(decision) = game.observe(PlayerId::One).decision {
            return decision;
        }
        let player = game.priority;
        game.apply(player, Action::PassPriority)
            .expect("the trigger is on the stack");
    }
}

/// Accepts the optional search and returns the search decision itself.
fn accept_the_search(game: &mut Game) -> DecisionObservation {
    let offer = ask(game);
    let accept = offer
        .options
        .last()
        .expect("the optional search offers accepting it")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: offer.id,
            options: vec![accept],
        },
    )
    .expect("the search is accepted");
    ask(game)
}

/// "Up to two": one is a legal answer, and the rest of the library stays
/// where it was.
#[test]
fn one_land_is_enough_for_up_to_two() {
    let (mut game, _titan) = staged(&[
        (58_200, cards::TAIGA),
        (58_201, cards::FOREST),
        (58_202, cards::MOUNTAIN),
    ]);

    let search = accept_the_search(&mut game);
    let forest = search
        .options
        .iter()
        .find(|option| {
            option
                .card
                .and_then(|(_, characteristics)| characteristics.card_definition())
                == Some(cards::FOREST)
        })
        .expect("the Forest is on offer")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: search.id,
            options: vec![forest],
        },
    )
    .expect("taking one of the two is legal");
    drain_pending(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition != cards::PRIMEVAL_TITAN)
            .count(),
        1,
        "one land came out, not two",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::FOREST && permanent.tapped),
        "and it arrived tapped like any other",
    );
    assert_eq!(
        game.players[0].library.len(),
        2,
        "the other two are still in the library",
    );
}

/// Declining the search leaves the library alone entirely.
#[test]
fn the_search_may_be_declined() {
    let (mut game, _titan) = staged(&[(58_300, cards::TAIGA), (58_301, cards::FOREST)]);

    let offer = ask(&mut game);
    let decline = offer
        .options
        .first()
        .expect("declining is the other answer")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: offer.id,
            options: vec![decline],
        },
    )
    .expect("\"you may\" means it can be turned down");
    drain_pending(&mut game);

    assert_eq!(game.players[0].library.len(), 2, "nothing left the library");
    assert_eq!(
        game.battlefield.len(),
        1,
        "and nothing but the Titan is on the battlefield",
    );
}

/// Trample is printed on it, which is what makes six power worth blocking
/// with everything rather than chump-blocking.
#[test]
fn it_tramples() {
    let (game, titan) = staged(&[]);

    assert!(
        game.permanent_has_executable_keyword(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == titan)
                .expect("it is on the battlefield"),
            KeywordAbility::Trample,
        ),
        "a 6/6 trampler",
    );
}
