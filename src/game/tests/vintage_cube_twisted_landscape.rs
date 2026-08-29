//! Twisted Landscape: the Jund member of the cycle, which finds its own
//! three basics and cycles for its own three colours.

use super::*;

/// Player one with the Landscape on the battlefield since last turn and
/// `library` stacked underneath.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for definition in library {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].library.push(card);
    }
    let landscape = game
        .put_onto_battlefield(PlayerId::One, cards::TWISTED_LANDSCAPE)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [1, 1];
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, landscape)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if game.observe(PlayerId::One).decision.is_some() {
            return;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

/// It taps for one colourless.
#[test]
fn it_taps_for_colorless() {
    let (mut game, landscape) = staged(&[]);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateManaAbility { source, .. } if *source == landscape)
        })
        .expect("the mana ability is offered");
    game.apply(PlayerId::One, action).expect("it taps");

    assert_eq!(game.players[0].mana_pool.colorless, 1);
}

/// It finds a Swamp, a Mountain, or a Forest, and nothing else.
#[test]
fn it_fetches_one_of_its_three_basics() {
    let (mut game, landscape) = staged(&[cards::PLAINS, cards::SWAMP]);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, .. } if *source == landscape)
        })
        .expect("the fetch is offered");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(&mut game);

    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("the search asks what to find");
    let offered: Vec<CardDefinitionId> = decision
        .options
        .iter()
        .filter_map(|option| {
            option
                .card
                .and_then(|(_, characteristics)| characteristics.card_definition())
        })
        .collect();
    assert!(offered.contains(&cards::SWAMP), "{offered:?}");
    assert!(
        !offered.contains(&cards::PLAINS),
        "a Plains is not one of the three: {offered:?}",
    );

    let swamp = decision
        .options
        .iter()
        .find(|option| {
            option.card.is_some_and(|(_, characteristics)| {
                characteristics.card_definition() == Some(cards::SWAMP)
            })
        })
        .expect("the Swamp is offered")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![swamp],
        },
    )
    .expect("finding it is legal");
    settle(&mut game);

    assert!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.definition == cards::SWAMP)
            .expect("the Swamp arrived")
            .tapped,
        "tapped, which is what the cycle charges",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == landscape),
        "and the land it came from is gone",
    );
}

/// Cycling costs three colours, which is the point: it is a land you cash
/// in when the hand needs a spell instead.
#[test]
fn it_cycles_for_three_colors() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    let drawn = game
        .build_zone(PlayerId::One, &[cards::GIANT_GROWTH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    game.players[0].library.push(drawn);
    let landscape = game
        .build_zone(PlayerId::One, &[cards::TWISTED_LANDSCAPE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let landscape_id = landscape.id;
    game.players[0].hand.push(landscape);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == landscape_id)
        ),
        "with no mana there is nothing to cycle with",
    );

    for color in [ManaColor::Black, ManaColor::Red, ManaColor::Green] {
        game.add_unrestricted_mana(PlayerId::One, color, 1);
    }
    let cycle = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, .. } if *source == landscape_id)
        })
        .expect("three colours buys the cycle");
    game.apply(PlayerId::One, cycle).expect("it activates");
    settle(&mut game);

    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::GIANT_GROWTH],
    );
}

/// "A basic Swamp, Mountain, or Forest card": a Bayou is a Swamp and a
/// Forest and still not one of them, because the supertype is what the
/// clause asks for. And the fetch is not a land drop, so the basic it
/// finds does not stop you playing one from hand.
#[test]
fn a_dual_with_the_right_types_is_not_basic_enough() {
    let (mut game, landscape) = staged(&[cards::BAYOU, cards::FOREST]);
    let held = card(97_800, cards::MOUNTAIN, PlayerId::One);
    let held_id = held.id;
    game.players[0].hand.push(held);
    game.players[0].lands_played_this_turn = 0;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, .. } if *source == landscape)
        })
        .expect("the fetch is offered");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(&mut game);

    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("the search asks what to find");
    let offered: Vec<CardDefinitionId> = decision
        .options
        .iter()
        .filter_map(|option| {
            option
                .card
                .and_then(|(_, characteristics)| characteristics.card_definition())
        })
        .collect();
    assert_eq!(
        offered,
        vec![cards::FOREST],
        "the Forest is basic and the Bayou only has the types",
    );

    let forest = decision
        .options
        .iter()
        .find(|option| {
            option.card.is_some_and(|(_, characteristics)| {
                characteristics.card_definition() == Some(cards::FOREST)
            })
        })
        .expect("the Forest is offered")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![forest],
        },
    )
    .expect("finding it is legal");
    settle(&mut game);

    assert_eq!(
        game.players[0].lands_played_this_turn, 0,
        "putting a land onto the battlefield is not playing one",
    );
    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::PlayLand { card, .. } if *card == held_id)),
        "so the Mountain in hand is still a land drop waiting",
    );
}
