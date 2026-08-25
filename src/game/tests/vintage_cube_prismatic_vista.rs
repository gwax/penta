//! Prismatic Vista: a fetchland for every basic at once, and for no dual at
//! all.

use super::*;

/// The Vista on the battlefield with `library` in the library.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for (index, definition) in library.iter().enumerate() {
        let id = 66_000 + u32::try_from(index).expect("a handful of cards");
        game.players[0]
            .library
            .push(card(id, *definition, PlayerId::One));
    }
    let vista = game
        .put_onto_battlefield(PlayerId::One, cards::PRISMATIC_VISTA)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].life = 20;
    (game, vista)
}

/// Cracks the Vista, taking `wanted` from the search when it is offered.
fn crack(game: &mut Game, vista: GameObjectId, wanted: Option<CardDefinitionId>) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == vista))
        .expect("the Vista can be cracked");
    game.apply(PlayerId::One, action).expect("it activates");
    // The ability has to resolve before the search asks anything.
    pass_priority_pair(game);
    for _ in 0..8 {
        let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        else {
            break;
        };
        let options = decision
            .options
            .iter()
            .filter(|option| {
                wanted.is_none_or(|wanted| {
                    option.card.is_some_and(|(_, characteristics)| {
                        characteristics.card_definition() == Some(wanted)
                    })
                })
            })
            .map(|option| option.id)
            .take(1)
            .collect::<Vec<_>>();
        game.apply(
            decision.player,
            Action::ChooseDecision {
                decision: decision.id,
                options,
            },
        )
        .expect("the choice is legal");
    }
    drain_pending(game);
    game.check_state_based_actions();
}

/// What the search offered, by definition.
fn offered(game: &Game) -> Vec<CardDefinitionId> {
    game.pending_decisions
        .first()
        .into_iter()
        .flat_map(|pending| pending.observation.options.iter())
        .filter_map(|option| {
            option
                .card
                .and_then(|(_, characteristics)| characteristics.card_definition())
        })
        .collect()
}

fn on_battlefield(game: &Game, definition: CardDefinitionId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.definition == definition)
}

/// A life and the land itself, for any basic you like -- untapped.
#[test]
fn it_finds_any_basic_untapped() {
    let (mut game, vista) = staged(&[cards::MOUNTAIN, cards::ISLAND]);

    crack(&mut game, vista, Some(cards::ISLAND));

    assert_eq!(game.players[0].life, 19, "one life");
    assert!(!on_battlefield(&game, cards::PRISMATIC_VISTA), "and itself");
    let island = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::ISLAND)
        .expect("the Island arrived");
    assert!(!island.tapped, "a fetchland's land arrives untapped");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::PRISMATIC_VISTA),
        "the Vista sacrificed itself to do it",
    );
}

/// "A basic land card" is the supertype: a dual land with two basic land
/// types printed on it is not one.
#[test]
fn a_dual_is_not_a_basic_land() {
    let (mut game, vista) = staged(&[cards::TUNDRA, cards::PLAINS]);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == vista))
        .expect("the Vista can be cracked");
    game.apply(PlayerId::One, action).expect("it activates");
    pass_priority_pair(&mut game);

    assert_eq!(
        offered(&game),
        vec![cards::PLAINS],
        "the Tundra has both types and neither supertype",
    );
}

/// Every basic, not a colour pair: the five are all on offer at once, which
/// is the whole difference from the cycles it copies.
#[test]
fn every_basic_is_on_offer() {
    let (mut game, vista) = staged(&[
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
    ]);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == vista))
        .expect("the Vista can be cracked");
    game.apply(PlayerId::One, action).expect("it activates");
    pass_priority_pair(&mut game);

    let mut found = offered(&game);
    found.sort_unstable();
    let mut basics = vec![
        cards::PLAINS,
        cards::ISLAND,
        cards::SWAMP,
        cards::MOUNTAIN,
        cards::FOREST,
    ];
    basics.sort_unstable();
    assert_eq!(found, basics);
}

/// A library with no basic in it: the Vista still eats itself and the life.
#[test]
fn an_empty_search_still_costs_a_life() {
    let (mut game, vista) = staged(&[cards::TUNDRA]);

    crack(&mut game, vista, None);

    assert_eq!(game.players[0].life, 19, "the life is a cost");
    assert!(
        !on_battlefield(&game, cards::PRISMATIC_VISTA),
        "and so is the land",
    );
    assert_eq!(game.players[0].library.len(), 1, "the Tundra stayed put");
}

/// At one life it can still be cracked; at zero there is nothing to pay with.
#[test]
fn the_life_has_to_be_there() {
    let (mut game, vista) = staged(&[cards::PLAINS]);
    game.players[0].life = 1;
    assert!(
        game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == vista)
        ),
        "paying down to zero is legal (CR 118.4)",
    );

    game.players[0].life = 0;
    assert!(
        !game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == vista)
        ),
        "and there is no life left to spend",
    );
}
