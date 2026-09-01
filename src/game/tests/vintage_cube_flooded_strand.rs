//! Flooded Strand: what "search your library" reaches, and what it does not.
//!
//! The cycle's costs, its fail-to-find, and its shuffle are checked where
//! the fetchlands live as a family. What this adds is the zone the search
//! reads and the breadth of what it turns up.

use super::*;

/// A Strand on the battlefield with `library` under it, ready to crack.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].graveyard.clear();
    game.players[PlayerId::One.index()].library.clear();
    for (index, definition) in library.iter().enumerate() {
        game.players[PlayerId::One.index()].library.push(card(
            99_700 + u32::try_from(index).expect("a short library"),
            *definition,
            PlayerId::One,
        ));
    }
    let strand = game
        .put_onto_battlefield(PlayerId::One, cards::FLOODED_STRAND)
        .expect("cataloged");
    drain_pending(&mut game);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, strand)
}

/// Cracks it and returns what the search offered, leaving the decision
/// standing.
fn crack(game: &mut Game, strand: GameObjectId) -> Option<DecisionObservation> {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == strand),
        )
        .expect("a life and a sacrifice");
    game.apply(PlayerId::One, action).expect("it activates");
    pass_priority_pair(game);
    game.pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
}

/// "A Plains or Island card" is two type lines and any number of cards: a
/// basic Island, a dual that is both, and a Tundra are all on the list, and
/// the Badlands beside them is not.
#[test]
fn every_land_of_either_type_is_offered_at_once() {
    let (mut game, strand) = staged(&[
        cards::ISLAND,
        cards::TUNDRA,
        cards::HALLOWED_FOUNTAIN,
        cards::BADLANDS,
        cards::FOREST,
    ]);

    let search = crack(&mut game, strand).expect("it asks what to find");
    let offered = search
        .options
        .iter()
        .filter_map(|option| option.card.and_then(|(_, card)| card.card_definition()))
        .collect::<Vec<_>>();

    for wanted in [cards::ISLAND, cards::TUNDRA, cards::HALLOWED_FOUNTAIN] {
        assert!(
            offered.contains(&wanted),
            "{wanted:?} is a Plains or an Island: {offered:?}",
        );
    }
    assert!(
        !offered.contains(&cards::BADLANDS),
        "a Swamp Mountain is neither",
    );
    assert!(!offered.contains(&cards::FOREST), "and nor is a Forest");
}

/// "Search your library": the Tundra in hand and the one in the graveyard
/// are not in it, so a library of Forests finds nothing -- and the life and
/// the land are spent anyway.
#[test]
fn it_searches_the_library_and_nowhere_else() {
    let (mut game, strand) = staged(&[cards::FOREST, cards::FOREST]);
    game.players[PlayerId::One.index()]
        .hand
        .push(card(99_800, cards::TUNDRA, PlayerId::One));
    game.players[PlayerId::One.index()]
        .graveyard
        .push(card(99_801, cards::TUNDRA, PlayerId::One));
    let life = game.players[PlayerId::One.index()].life;

    if let Some(search) = crack(&mut game, strand) {
        assert!(
            search
                .options
                .iter()
                .filter_map(|option| option.card.and_then(|(_, card)| card.card_definition()))
                .all(|definition| definition != cards::TUNDRA),
            "neither Tundra is in the library the search reads",
        );
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: search.id,
                options: Vec::new(),
            },
        )
        .expect("finding nothing is the only answer");
    }
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::TUNDRA),
        "nothing arrived from hand or graveyard",
    );
    assert_eq!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .filter(|card| card.definition == cards::TUNDRA)
            .count(),
        1,
        "the one in hand stayed there",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life - 1,
        "and the life was paid for the privilege",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == strand),
        "along with the land itself",
    );
}
