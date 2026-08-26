//! Eagles of the North: one mana for the Plains most of the time, and a
//! board-wide charge on the turn the six is actually paid.

use super::*;

/// Player One holding the Eagles, with `board` under them and a library
/// holding a Plains.
fn staged(board: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..3 {
        game.players[0]
            .library
            .push(card(116_000 + index, cards::ISLAND, PlayerId::One));
    }
    game.players[0]
        .library
        .push(card(116_100, cards::PLAINS, PlayerId::One));
    let mut ids = Vec::new();
    for definition in board {
        ids.push(
            game.put_onto_battlefield(PlayerId::One, *definition)
                .expect("cataloged"),
        );
    }
    drain_pending(&mut game);
    let card = game
        .build_zone(PlayerId::One, &[cards::EAGLES_OF_THE_NORTH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[0].hand.push(card);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [7, 7];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held, ids)
}

fn stats(game: &Game, id: GameObjectId) -> (Option<i16>, Option<i16>) {
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield");
    (game.power(permanent), game.toughness(permanent))
}

/// Six mana buys a 3/3 flier and a charge for the whole board.
#[test]
fn casting_it_charges_the_board() {
    let (mut game, held, board) = staged(&[cards::GRIZZLY_BEARS]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 5);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held))
        .expect("six mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    let bears = board[0];
    assert_eq!(
        stats(&game, bears),
        (Some(3), Some(2)),
        "+1/+0 for the bear"
    );
    assert!(
        game.permanent_has_executable_keyword(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == bears)
                .expect("it is there"),
            KeywordAbility::FirstStrike,
        ),
        "and first strike with it",
    );

    let eagles = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::EAGLES_OF_THE_NORTH)
        .expect("the Bird is there");
    assert_eq!(game.power(eagles), Some(4), "they charge themselves too");
    assert!(game.has_flying(eagles));
}

/// The charge is until end of turn.
#[test]
fn the_charge_wears_off() {
    let (mut game, held, board) = staged(&[cards::GRIZZLY_BEARS]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 5);
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held))
        .expect("six mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);
    assert_eq!(stats(&game, board[0]), (Some(3), Some(2)));

    let turn = game.turn;
    for _ in 0..60 {
        if game.turn > turn {
            break;
        }
        game.advance_step();
        drain_pending(&mut game);
    }

    assert_eq!(
        stats(&game, board[0]),
        (Some(2), Some(2)),
        "back to an ordinary bear",
    );
}

/// Their creatures are not yours.
#[test]
fn their_creatures_are_not_charged() {
    let (mut game, held, _) = staged(&[]);
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 5);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held))
        .expect("six mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(stats(&game, theirs), (Some(2), Some(2)), "theirs is theirs");
}

/// One mana and the card itself fetches a Plains instead.
#[test]
fn plainscycling_finds_the_land() {
    let (mut game, held, _) = staged(&[]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    let cycle = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == held))
        .expect("plainscycling is offered from hand");
    game.apply(PlayerId::One, cycle).expect("it activates");
    drain_pending(&mut game);

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::PLAINS),
        "the Plains is in hand",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::EAGLES_OF_THE_NORTH),
        "and the Eagles paid for it",
    );
    assert!(
        !game.players[0]
            .library
            .iter()
            .any(|card| card.definition == cards::PLAINS),
    );
}
