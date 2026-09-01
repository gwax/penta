//! Razorverge Thicket: the fastland boundary, walked through the land drop
//! rather than put onto the battlefield. The cycle's shared behaviour is in
//! `vintage_cube_lands`; what is here is the play from hand that a deck
//! actually makes, where the count and the tap decide the turn.

use super::*;

/// Player One with `others` Forests out and a Thicket in hand, at a point in
/// the turn where the land drop is available.
fn staged(others: u32) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    for index in 0..others {
        game.battlefield
            .push(creature(64_000 + index, cards::FOREST, PlayerId::One));
    }
    // Their lands are not lands you control, however many they have.
    for index in 0..4 {
        game.battlefield
            .push(creature(64_100 + index, cards::ISLAND, PlayerId::Two));
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    let thicket = game
        .build_zone(PlayerId::One, &[cards::RAZORVERGE_THICKET])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = thicket.id;
    game.players[0].hand.push(thicket);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].lands_played_this_turn = 0;
    (game, id)
}

/// Plays the Thicket and answers the entry, returning the permanent it
/// became.
fn play(game: &mut Game, thicket: GameObjectId) -> GameObjectId {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == thicket))
        .expect("a land drop is available");
    game.apply(PlayerId::One, action).expect("it is played");
    drain_pending(game);
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::RAZORVERGE_THICKET)
        .expect("it arrived")
        .card
        .id
}

fn tapped(game: &Game, land: GameObjectId) -> bool {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == land)
        .expect("it is there")
        .tapped
}

/// "Unless you control two or fewer other lands": the third land you play is
/// still under the line, and it makes mana the turn it lands.
#[test]
fn the_third_land_arrives_ready_to_use() {
    let (mut game, thicket) = staged(2);

    let land = play(&mut game, thicket);

    assert!(!tapped(&game, land), "two others is two or fewer");
    for color in [ManaColor::Green, ManaColor::White] {
        assert!(
            game.legal_actions(PlayerId::One).into_iter().any(|action| {
                matches!(action, Action::ActivateManaAbility { source, color: made, .. }
                    if source == land && made == color)
            }),
            "and it makes {color:?} the turn it is played",
        );
    }
}

/// The fourth is over it, and arrives tapped with nothing to offer.
#[test]
fn the_fourth_land_arrives_tapped() {
    let (mut game, thicket) = staged(3);

    let land = play(&mut game, thicket);

    assert!(tapped(&game, land), "three others is more than two");
    assert!(
        game.legal_actions(PlayerId::One).into_iter().all(
            |action| !matches!(action, Action::ActivateManaAbility { source, .. } if source == land)
        ),
        "a tapped land makes nothing this turn",
    );
    assert_eq!(
        game.players[0].lands_played_this_turn, 1,
        "and it cost the land drop either way",
    );
}

/// "Other lands": the Thicket does not count itself, which is what puts the
/// boundary between the third land and the fourth rather than a step
/// earlier.
#[test]
fn it_does_not_count_itself() {
    let (mut game, first) = staged(2);
    let untapped = play(&mut game, first);
    assert!(!tapped(&game, untapped), "the third land is untapped");

    let second = game
        .build_zone(PlayerId::One, &[cards::RAZORVERGE_THICKET])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let second_id = second.id;
    game.players[0].hand.push(second);
    game.players[0].lands_played_this_turn = 0;
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == second_id))
        .expect("a second land drop");
    game.apply(PlayerId::One, action).expect("it is played");
    drain_pending(&mut game);

    let late = game
        .battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == cards::RAZORVERGE_THICKET)
        .find(|permanent| permanent.card.id != untapped)
        .expect("the second arrived")
        .card
        .id;
    assert!(
        tapped(&game, late),
        "the first Thicket is one of the three others the second counts",
    );
}
