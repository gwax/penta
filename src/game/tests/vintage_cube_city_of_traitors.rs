//! City of Traitors: two mana from one land, for as long as you are willing
//! to stop playing lands.

use super::*;

/// The City on the battlefield under Player One, with `hand` in hand and a
/// land drop still available.
fn staged(hand: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let city = game
        .put_onto_battlefield(PlayerId::One, cards::CITY_OF_TRAITORS)
        .expect("cataloged");
    drain_pending(&mut game);
    let mut held = Vec::new();
    for definition in hand {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        held.push(card.id);
        game.players[0].hand.push(card);
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.players[0].lands_played_this_turn = 0;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, city, held)
}

fn alive(game: &Game, id: GameObjectId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.id == id)
}

/// Plays `card` as a land and settles what follows.
fn play_land(game: &mut Game, card: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card: played, .. } if *played == card))
        .expect("the land drop is available");
    game.apply(PlayerId::One, action).expect("it is played");
    drain_pending(game);
}

/// It taps for two colourless.
#[test]
fn it_taps_for_two() {
    let (mut game, city, _) = staged(&[]);

    let add = Action::ActivateManaAbility {
        source: city,
        ability: mana_ability_for(&game, city, ManaColor::Colorless),
        color: ManaColor::Colorless,
        counters_removed: None,
        cost_object: None,
        combination: None,
        triggered_mana: None,
    };
    game.apply(PlayerId::One, add).expect("it taps");

    assert_eq!(game.players[0].mana_pool.colorless, 2);
}

/// Playing another land sacrifices it.
#[test]
fn the_next_land_drop_kills_it() {
    let (mut game, city, held) = staged(&[cards::FOREST]);

    play_land(&mut game, held[0]);

    assert!(!alive(&game, city), "the City is sacrificed");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::CITY_OF_TRAITORS),
        "and it is in the graveyard",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::FOREST),
        "while the land that killed it stays",
    );
}

/// It is the playing that kills it, not the entering: a land an effect puts
/// onto the battlefield leaves the City alone.
#[test]
fn a_land_put_onto_the_battlefield_leaves_it_alone() {
    let (mut game, city, _) = staged(&[]);

    game.put_onto_battlefield(PlayerId::One, cards::FOREST)
        .expect("cataloged");
    drain_pending(&mut game);

    assert!(
        alive(&game, city),
        "nothing was played, so nothing happened"
    );
}

/// "Another": the City arriving does not sacrifice itself.
#[test]
fn playing_the_city_itself_does_nothing() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let card = game
        .build_zone(PlayerId::One, &[cards::CITY_OF_TRAITORS])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[0].hand.push(card);
    game.players[0].lands_played_this_turn = 0;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    play_land(&mut game, held);

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == cards::CITY_OF_TRAITORS)
            .count(),
        1,
        "it is not another land",
    );
}

/// Their land drop is not yours.
#[test]
fn their_land_drop_leaves_it_alone() {
    let (mut game, city, _) = staged(&[]);
    let theirs = game
        .build_zone(PlayerId::Two, &[cards::FOREST])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = theirs.id;
    game.players[1].hand.push(theirs);
    game.players[1].lands_played_this_turn = 0;
    game.active_player = PlayerId::Two;
    game.priority = PlayerId::Two;

    let action = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == held))
        .expect("their land drop is available");
    game.apply(PlayerId::Two, action).expect("it is played");
    drain_pending(&mut game);

    assert!(alive(&game, city), "\"you play\" is not \"they play\"");
}

/// The sacrifice is a triggered ability rather than part of playing the
/// land, so there is a window under it: the City is still on the battlefield
/// while the trigger waits, and the mana it makes outlives it.
#[test]
fn its_mana_can_be_taken_while_the_sacrifice_waits() {
    let (mut game, city, held) = staged(&[cards::FOREST]);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == held[0]))
        .expect("the land drop is available");
    game.apply(PlayerId::One, action).expect("it is played");
    for _ in 0..4 {
        if !game.stack.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    assert_eq!(game.stack.len(), 1, "the sacrifice is on the stack");
    assert!(
        alive(&game, city),
        "and the City is still standing under it"
    );
    let add = Action::ActivateManaAbility {
        source: city,
        ability: mana_ability_for(&game, city, ManaColor::Colorless),
        color: ManaColor::Colorless,
        counters_removed: None,
        cost_object: None,
        combination: None,
        triggered_mana: None,
    };
    game.apply(PlayerId::One, add).expect("it taps in response");
    drain_pending(&mut game);

    assert!(!alive(&game, city), "the trigger still takes it");
    assert_eq!(
        game.players[0].mana_pool.colorless, 2,
        "and the two it made are still in the pool",
    );
}

/// "Another land" reads both ways at once: a second City is another land to
/// the first, and its own trigger still excludes itself. So the new one
/// stands on the old one's grave.
#[test]
fn a_second_city_replaces_the_first() {
    let (mut game, city, held) = staged(&[cards::CITY_OF_TRAITORS]);

    play_land(&mut game, held[0]);

    assert!(!alive(&game, city), "the first is another land's casualty");
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == cards::CITY_OF_TRAITORS)
            .count(),
        1,
        "and the second did not sacrifice itself",
    );
}

/// The trigger asks whether a land was played, not where it was played
/// from. A Crucible turns the graveyard into a land drop, and a land drop is
/// what the City cannot survive.
#[test]
fn a_land_played_from_the_graveyard_kills_it_too() {
    let (mut game, city, _) = staged(&[]);
    game.put_onto_battlefield(PlayerId::One, cards::CRUCIBLE_OF_WORLDS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.players[0].graveyard.clear();
    let buried = card(88_000, cards::FOREST, PlayerId::One);
    let buried_id = buried.id;
    game.players[0].graveyard.push(buried);
    game.players[0].lands_played_this_turn = 0;
    game.priority = PlayerId::One;

    play_land(&mut game, buried_id);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::FOREST),
        "the Forest came out of the graveyard",
    );
    assert!(
        !alive(&game, city),
        "and playing it is playing a land, wherever it came from",
    );
}
