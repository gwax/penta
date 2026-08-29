//! Expedition Map: three mana over two turns for any land in the deck.

use super::*;

/// Player One with a Map on the battlefield and `library` in their library.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].library.clear();
    game.players[0].hand.clear();
    for definition in library {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].library.push(card);
    }
    let map = game
        .put_onto_battlefield(PlayerId::One, cards::EXPEDITION_MAP)
        .expect("cataloged");
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == map)
    {
        permanent.entered_controller_turn = 0;
    }
    drain_pending(&mut game);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.priority = PlayerId::One;
    (game, map)
}

/// Cracks the Map, taking whatever the search offers.
fn crack(game: &mut Game, map: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == map))
        .expect("two mana and a tap cracks it");
    game.apply(PlayerId::One, action).expect("it activates");

    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .take(decision.maximum.max(1))
                .map(|option| option.id)
                .collect();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the search accepts what it offered");
            continue;
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

/// It finds a land and puts it in hand, sacrificing itself to do it.
#[test]
fn it_finds_a_land_and_goes_away() {
    let (mut game, map) = staged(&[cards::MOUNTAIN]);
    crack(&mut game, map);

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::MOUNTAIN),
        "the land is in hand",
    );
    assert!(
        game.battlefield.is_empty(),
        "and the Map sacrificed itself as a cost",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::EXPEDITION_MAP),
    );
}

/// A nonland card is not what it is looking for.
#[test]
fn it_will_not_find_a_spell() {
    let (mut game, map) = staged(&[cards::LIGHTNING_BOLT]);
    crack(&mut game, map);

    assert!(
        game.players[0].hand.is_empty(),
        "an instant is not a land card",
    );
    assert_eq!(game.players[0].library.len(), 1, "and it stayed put");
    assert!(game.battlefield.is_empty(), "the Map is spent either way");
}

/// A nonbasic land is a land card, which is the whole point of the card.
#[test]
fn it_finds_a_nonbasic_land() {
    let (mut game, map) = staged(&[cards::CELESTIAL_COLONNADE]);
    crack(&mut game, map);

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::CELESTIAL_COLONNADE),
    );
}

/// Without the mana there is nothing to activate.
#[test]
fn it_needs_the_two_mana() {
    let (mut game, map) = staged(&[cards::MOUNTAIN]);
    game.players[0].mana_pool = ManaPool::default();

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .all(|action| !matches!(
                action,
                Action::ActivateAbility { source, .. } if *source == map
            )),
        "the ability costs two on top of the tap",
    );
}

/// "Reveal it": what the Map finds is shown, which is the difference between
/// it and a tutor that keeps its answer to itself.
#[test]
fn it_shows_what_it_found() {
    let (mut game, map) = staged(&[cards::WASTELAND]);
    let found = game.players[0].library[0].id;

    crack(&mut game, map);

    assert!(
        game.events.iter().any(|event| matches!(
            event,
            GameEvent::CardRevealed {
                player: PlayerId::One,
                card,
                definition,
            } if *card == found && *definition == cards::WASTELAND
        )),
        "the land it took was revealed on the way to hand",
    );
}

/// The tap is an artifact's tap: the Map cracks on the turn it arrives.
#[test]
fn it_cracks_the_turn_it_lands() {
    let (mut game, map) = staged(&[cards::WASTELAND]);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == map)
    {
        permanent.entered_controller_turn = game.turns_started[0];
    }

    crack(&mut game, map);

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::WASTELAND),
        "nothing about it waits a turn",
    );
}

/// The mana, the tap and the Map are costs: a library with no land in it
/// spends all three for nothing.
#[test]
fn a_landless_library_still_costs_the_map() {
    let (mut game, map) = staged(&[cards::LIGHTNING_BOLT]);

    crack(&mut game, map);

    assert!(game.players[0].hand.is_empty(), "there was nothing to find");
    assert_eq!(
        game.players[0].library.len(),
        1,
        "and the Bolt stayed where it was",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::EXPEDITION_MAP),
        "the Map is spent all the same",
    );
    assert_eq!(game.players[0].mana_pool.total(), 0, "and so is the mana");
}
