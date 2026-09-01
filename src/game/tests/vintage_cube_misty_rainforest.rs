//! Misty Rainforest: the Simic fetch, and the two-type menu it offers.
//!
//! What a fetchland is -- the life and the sacrifice as costs, Stifle, the
//! private search, the shuffle on a failed one, cracking on their turn --
//! is pinned in `tutors_and_fetch_lands`, along with each fetch finding its
//! own enemy pair and not another's. What is here is the half those leave
//! alone: a fetch whose library holds both of its types offers both, and
//! the one named is the one that arrives.

use super::*;

/// A Rainforest on the battlefield with `library` behind it, cracked, and
/// the search decision waiting.
fn cracked(library: &[CardDefinitionId]) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    let source = game
        .put_onto_battlefield(PlayerId::One, cards::MISTY_RAINFOREST)
        .expect("cataloged");
    game.players[0].library.clear();
    for (index, definition) in library.iter().enumerate() {
        game.players[0].library.push(card(
            121_000 + u32::try_from(index).expect("few cards"),
            *definition,
            PlayerId::One,
        ));
    }
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source: actual, .. } if *actual == source)
        })
        .expect("the fetch is offered");
    game.apply(PlayerId::One, crack).expect("it activates");
    pass_priority_pair(&mut game);
    game
}

/// What the pending search is offering.
fn offered(game: &Game) -> Vec<CardDefinitionId> {
    game.observe(PlayerId::One)
        .decision
        .expect("the search asks what to find")
        .options
        .iter()
        .filter_map(|option| match option.card {
            Some((_, ObjectCharacteristics::Card { definition, .. })) => Some(definition),
            _ => None,
        })
        .collect()
}

/// "A Forest or Island card" is two types and a choice between them: a
/// library holding one of each offers both, and the Swamp beside them is
/// neither.
#[test]
fn it_offers_both_of_its_types_and_nothing_else() {
    let game = cracked(&[cards::FOREST, cards::ISLAND, cards::SWAMP]);

    let mut names = offered(&game);
    names.sort_unstable();
    let mut expected = vec![cards::FOREST, cards::ISLAND];
    expected.sort_unstable();
    assert_eq!(names, expected, "both halves of the pair, and only those");
}

/// And the choice is honoured: naming the Island brings the Island, with the
/// Forest left where it was.
#[test]
fn the_land_named_is_the_land_that_arrives() {
    for wanted in [cards::FOREST, cards::ISLAND] {
        let mut game = cracked(&[cards::FOREST, cards::ISLAND, cards::SWAMP]);
        let decision = game
            .observe(PlayerId::One)
            .decision
            .expect("the search asks what to find");
        let option = decision
            .options
            .iter()
            .find(|option| {
                matches!(
                    option.card,
                    Some((_, ObjectCharacteristics::Card { definition, .. }))
                        if definition == wanted
                )
            })
            .unwrap_or_else(|| panic!("{wanted:?} is on the menu"))
            .id;
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: decision.id,
                options: vec![option],
            },
        )
        .expect("taking it is legal");
        drain_pending(&mut game);

        let arrived: Vec<_> = game
            .battlefield
            .iter()
            .map(|permanent| permanent.card.definition)
            .collect();
        assert_eq!(
            arrived,
            vec![ObjectKind::Card(wanted)],
            "the Rainforest sacrificed itself for the one named",
        );
        assert_eq!(
            game.players[0].library.len(),
            2,
            "and the other two stayed in the library",
        );
        assert_eq!(game.players[0].life, 19, "one life, whichever was taken");
    }
}
