//! Keen-Eyed Curator: he answers a graveyard a card at a time, and turns
//! into a 7/7 trampler for having done it four kinds of times.

use super::*;

/// The Curator on the battlefield under Player One, with `theirs` in Player
/// Two's graveyard and mana to spare.
fn staged(theirs: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].graveyard.clear();
    let mut buried = Vec::new();
    for (index, definition) in theirs.iter().enumerate() {
        let id = 120_000 + u32::try_from(index).expect("a short graveyard");
        let card = card(id, *definition, PlayerId::Two);
        buried.push(card.id);
        game.players[1].graveyard.push(card);
    }
    let curator = game
        .put_onto_battlefield(PlayerId::One, cards::KEEN_EYED_CURATOR)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 8);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, curator, buried)
}

fn stats(game: &Game, id: GameObjectId) -> (Option<i16>, Option<i16>) {
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("he is on the battlefield");
    (game.power(permanent), game.toughness(permanent))
}

/// Exiles `card` with the Curator.
fn curate(game: &mut Game, curator: GameObjectId, card: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source, targets, ..
            } => {
                *source == curator
                    && targets
                        .iter()
                        .any(|selection| selection.targets().contains(&Target::Card(card)))
            }
            _ => false,
        })
        .expect("he can point at that card");
    game.apply(PlayerId::One, action).expect("it activates");
    drain_pending(game);
}

/// One mana takes a card out of their graveyard for good.
#[test]
fn he_exiles_a_card_from_their_graveyard() {
    let (mut game, curator, buried) = staged(&[cards::GRIZZLY_BEARS]);

    curate(&mut game, curator, buried[0]);

    assert!(game.players[1].graveyard.is_empty(), "their card is gone");
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "and it is in exile",
    );
    assert_eq!(stats(&game, curator), (Some(3), Some(3)), "one type so far");
}

/// Three card types is not four.
#[test]
fn three_card_types_leave_him_a_three_three() {
    let (mut game, curator, buried) = staged(&[
        cards::GRIZZLY_BEARS,
        cards::LIGHTNING_BOLT,
        cards::HOWLING_MINE,
    ]);

    for card in &buried {
        curate(&mut game, curator, *card);
    }

    assert_eq!(stats(&game, curator), (Some(3), Some(3)));
    assert!(
        !game.permanent_has_executable_keyword(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == curator)
                .expect("he is there"),
            KeywordAbility::Trample,
        ),
        "and no trample either",
    );
}

/// The fourth type turns him into a 7/7 trampler.
#[test]
fn four_card_types_make_him_a_seven_seven() {
    let (mut game, curator, buried) = staged(&[
        cards::GRIZZLY_BEARS,
        cards::LIGHTNING_BOLT,
        cards::HOWLING_MINE,
        cards::FOREST,
    ]);

    for card in &buried {
        curate(&mut game, curator, *card);
    }

    assert_eq!(stats(&game, curator), (Some(7), Some(7)));
    assert!(
        game.permanent_has_executable_keyword(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == curator)
                .expect("he is there"),
            KeywordAbility::Trample,
        ),
        "and trample with it",
    );
}

/// Four cards of one type is one type: the count is of kinds, not cards.
#[test]
fn four_creatures_are_still_one_type() {
    let (mut game, curator, buried) = staged(&[
        cards::GRIZZLY_BEARS,
        cards::SAVANNAH_LIONS,
        cards::GRAVE_TITAN,
        cards::GIANT_SPIDER,
    ]);

    for card in &buried {
        curate(&mut game, curator, *card);
    }

    assert_eq!(stats(&game, curator), (Some(3), Some(3)));
}

/// "As long as": a card leaving the pile takes the bonus with it.
#[test]
fn the_bonus_leaves_with_the_pile() {
    let (mut game, curator, buried) = staged(&[
        cards::GRIZZLY_BEARS,
        cards::LIGHTNING_BOLT,
        cards::HOWLING_MINE,
        cards::FOREST,
    ]);
    for card in &buried {
        curate(&mut game, curator, *card);
    }
    assert_eq!(stats(&game, curator), (Some(7), Some(7)));

    // The land goes back to its owner's graveyard, which is one card type
    // fewer among what he still holds.
    let land = game.players[1]
        .exile
        .iter()
        .position(|card| card.definition == cards::FOREST)
        .expect("the Forest is in exile");
    let card = game.players[1].exile.remove(land);
    game.players[1].graveyard.push(card);

    assert_eq!(stats(&game, curator), (Some(3), Some(3)), "back to a 3/3");
}

/// Your own graveyard is a graveyard too.
#[test]
fn he_may_take_from_your_own_graveyard() {
    let (mut game, curator, _) = staged(&[]);
    game.players[0]
        .graveyard
        .push(card(120_500, cards::LIGHTNING_BOLT, PlayerId::One));
    let mine = game.players[0].graveyard[0].id;

    curate(&mut game, curator, mine);

    assert!(game.players[0].graveyard.is_empty(), "yours is a graveyard");
}
