//! Crucible of Worlds: land drops out of the graveyard.

use super::*;

/// A main phase with a Mountain and a Lightning Bolt in the graveyard.
fn staged(with_crucible: bool) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[0]
        .graveyard
        .push(card(87_000, cards::MOUNTAIN, PlayerId::One));
    game.players[0]
        .graveyard
        .push(card(87_001, cards::LIGHTNING_BOLT, PlayerId::One));
    if with_crucible {
        game.put_onto_battlefield(PlayerId::One, cards::CRUCIBLE_OF_WORLDS)
            .expect("cataloged");
        drain_pending(&mut game);
    }
    game
}

fn land_plays(game: &Game) -> Vec<GameObjectId> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::PlayLand { card, .. } => Some(card),
            _ => None,
        })
        .collect()
}

/// Without it a land in the graveyard is not a land drop.
#[test]
fn a_graveyard_land_is_not_playable_on_its_own() {
    let game = staged(false);

    assert!(land_plays(&game).is_empty(), "nothing to play");
}

/// With it the land is offered, and nothing else in the graveyard is.
#[test]
fn the_crucible_offers_the_land_and_only_the_land() {
    let game = staged(true);

    assert_eq!(
        land_plays(&game),
        vec![GameObjectId(87_000)],
        "the Mountain, and not the Bolt",
    );
    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::CastSpell { card, .. } if *card == GameObjectId(87_001))
        }),
        "the permission names lands, so the Bolt is still stuck",
    );
}

/// Playing it moves it out of the graveyard and onto the battlefield, and
/// spends the turn's land drop.
#[test]
fn playing_it_takes_the_land_drop() {
    let mut game = staged(true);
    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { .. }))
        .expect("the Mountain is offered");

    game.apply(PlayerId::One, play).expect("it is played");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::MOUNTAIN),
        "the land is on the battlefield",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .all(|card| card.definition != cards::MOUNTAIN),
        "and out of the graveyard",
    );
    assert!(
        land_plays(&game).is_empty(),
        "one land drop a turn, wherever it came from",
    );
}

/// The permission is the Crucible's; losing it closes the graveyard again.
#[test]
fn losing_the_crucible_closes_the_graveyard() {
    let mut game = staged(true);
    let crucible = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::CRUCIBLE_OF_WORLDS)
        .map(|permanent| permanent.card.id)
        .expect("it is there");

    game.move_permanents_to_graveyard(&[crucible]);
    drain_pending(&mut game);

    assert!(land_plays(&game).is_empty(), "the permission went with it");
}

/// It is your own graveyard: their lands stay theirs.
#[test]
fn it_does_not_reach_the_opponents_graveyard() {
    let mut game = staged(true);
    game.players[1].graveyard.clear();
    game.players[1]
        .graveyard
        .push(card(87_010, cards::FOREST, PlayerId::Two));

    assert_eq!(
        land_plays(&game),
        vec![GameObjectId(87_000)],
        "only your own Mountain",
    );
}

/// "Crucible of Worlds doesn't allow you to activate abilities (such as
/// cycling) of land cards in your graveyard." The permission is to play a
/// land, and cycling is neither playing nor a land.
#[test]
fn it_opens_the_land_drop_and_not_the_cycling() {
    let mut game = staged(true);
    let steppe = card(87_100, cards::SECLUDED_STEPPE, PlayerId::One);
    let steppe_id = steppe.id;
    game.players[0].graveyard.push(steppe);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 2);

    assert!(
        land_plays(&game).contains(&steppe_id),
        "the Steppe is a land, so the Crucible offers it as a land drop",
    );
    assert!(
        !game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == steppe_id)
        ),
        "but its cycling stays where it was printed, which is the hand",
    );
}

/// "It doesn't change the times when you can play those land cards." One a
/// turn, your own main phase, empty stack -- the same three gates an
/// ordinary land drop waits for.
#[test]
fn it_does_not_change_when_a_land_may_be_played() {
    let mut game = staged(true);
    let mountain = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == cards::MOUNTAIN)
        .expect("it is buried")
        .id;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    assert!(
        land_plays(&game).contains(&mountain),
        "a main phase is the window"
    );

    game.step = Step::Upkeep;
    assert!(!land_plays(&game).contains(&mountain), "an upkeep is not");

    game.step = Step::PrecombatMain;
    game.active_player = PlayerId::Two;
    assert!(
        !land_plays(&game).contains(&mountain),
        "and neither is their turn",
    );

    game.active_player = PlayerId::One;
    game.players[0].lands_played_this_turn = 1;
    assert!(
        !land_plays(&game).contains(&mountain),
        "nor a turn whose land has already been played",
    );
}

/// What the card is actually for: a fetchland is a land drop that puts
/// itself in the graveyard, so with a Crucible out it is the same land drop
/// again every turn. Play it from the graveyard, crack it, and it is back
/// where it started -- offered again once the turn's drop comes round.
#[test]
fn a_fetchland_replayed_from_the_graveyard_sacrifices_itself_back_into_it() {
    let mut game = staged(true);
    game.players[0].graveyard.clear();
    game.players[0]
        .graveyard
        .push(card(87_200, cards::WINDSWEPT_HEATH, PlayerId::One));
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(87_201, cards::FOREST, PlayerId::One));

    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == GameObjectId(87_200)))
        .expect("the Crucible offers the Heath");
    game.apply(PlayerId::One, play).expect("it is played");
    drain_pending(&mut game);

    let heath = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::WINDSWEPT_HEATH)
        .map(|permanent| permanent.card.id)
        .expect("it arrived as a new permanent");
    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == heath))
        .expect("a fetchland on the battlefield cracks like any other");
    game.apply(PlayerId::One, crack).expect("it activates");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::FOREST),
        "the Forest it went looking for",
    );
    let returned = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == cards::WINDSWEPT_HEATH)
        .map(|card| card.id)
        .expect("the Heath sacrificed itself back into the graveyard");
    assert!(
        land_plays(&game).is_empty(),
        "this turn's drop was spent playing it",
    );

    game.players[0].lands_played_this_turn = 0;
    assert_eq!(
        land_plays(&game),
        vec![returned],
        "and next turn's drop is the same Heath again, new object though it is",
    );
}
