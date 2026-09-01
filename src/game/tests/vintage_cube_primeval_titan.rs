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

/// Two lands at once, one of them a fastland. "If one of these lands enters
/// at the same time as one or more other lands, it doesn't take those lands
/// into consideration" -- but what the Titan says wins either way: it puts
/// them onto the battlefield tapped, so the Marsh is tapped on an empty
/// board, where its own clause would have let it in untapped.
#[test]
fn the_titan_taps_a_fastland_that_would_have_come_in_untapped() {
    let (mut game, _titan) = staged(&[
        (58_300, cards::BLOOMING_MARSH),
        (58_301, cards::FOREST),
        (58_302, cards::MOUNTAIN),
    ]);

    let search = accept_the_search(&mut game);
    let wanted = [cards::BLOOMING_MARSH, cards::FOREST]
        .into_iter()
        .map(|definition| {
            search
                .options
                .iter()
                .find(|option| {
                    option
                        .card
                        .and_then(|(_, characteristics)| characteristics.card_definition())
                        == Some(definition)
                })
                .unwrap_or_else(|| panic!("{definition:?} is on offer"))
                .id
        })
        .collect::<Vec<_>>();
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: search.id,
            options: wanted,
        },
    )
    .expect("taking both is legal");
    drain_pending(&mut game);

    let marsh = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::BLOOMING_MARSH)
        .expect("the Marsh arrived");
    assert!(
        marsh.tapped,
        "the Titan said tapped, and no land count argues with that",
    );
}

/// "Whenever this creature enters *or attacks*": the second half of the
/// clause, which is why the Titan is worth leaving alive. Attacking finds
/// two more lands, and they arrive tapped like the first two.
#[test]
fn attacking_searches_again() {
    let (mut game, titan) = staged(&[
        (105_000, cards::FOREST),
        (105_001, cards::ISLAND),
        (105_002, cards::MOUNTAIN),
    ]);
    // Its own arrival is answered first, so the attack is the only trigger
    // this test is counting.
    let search = accept_the_search(&mut game);
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: search.id,
            options: Vec::new(),
        },
    )
    .expect("finding nothing is an answer");
    drain_pending(&mut game);
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id == titan),
        "nothing came in on the way down",
    );

    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == titan)
        .expect("it is there")
        .entered_controller_turn = 0;
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.priority = PlayerId::One;
    game.declare_attacker(titan, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();

    let search = accept_the_search(&mut game);
    let two = search
        .options
        .iter()
        .map(|option| option.id)
        .take(2)
        .collect::<Vec<_>>();
    assert_eq!(two.len(), 2, "every land in the library is on offer");
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: search.id,
            options: two,
        },
    )
    .expect("two of them is what it allows");
    drain_pending(&mut game);

    let arrived = game
        .battlefield
        .iter()
        .filter(|permanent| permanent.card.id != titan)
        .collect::<Vec<_>>();
    assert_eq!(arrived.len(), 2, "the attack found two lands");
    assert!(
        arrived.iter().all(|permanent| permanent.tapped),
        "and they arrive tapped, attacking or arriving",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        1,
        "leaving what it did not take",
    );
}
