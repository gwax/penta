//! Icetill Explorer: three clauses that feed each other -- the extra land
//! drop wants lands, the mill finds them, and the graveyard is where the
//! mill puts them.

use super::*;

/// The Explorer on the battlefield under Player One, with `hand` in hand,
/// `graveyard` in the graveyard, and a stocked library.
fn staged(hand: &[CardDefinitionId], graveyard: &[CardDefinitionId]) -> (Game, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[0].library.clear();
    for index in 0..8 {
        game.players[0]
            .library
            .push(card(122_000 + index, cards::ISLAND, PlayerId::One));
    }
    for (index, definition) in graveyard.iter().enumerate() {
        let id = 122_100 + u32::try_from(index).expect("a short graveyard");
        game.players[0]
            .graveyard
            .push(card(id, *definition, PlayerId::One));
    }
    game.put_onto_battlefield(PlayerId::One, cards::ICETILL_EXPLORER)
        .expect("cataloged");
    drain_pending(&mut game);
    let mut held = Vec::new();
    for (index, definition) in hand.iter().enumerate() {
        let id = 122_200 + u32::try_from(index).expect("a short hand");
        let card = card(id, *definition, PlayerId::One);
        held.push(card.id);
        game.players[0].hand.push(card);
    }
    game.players[0].lands_played_this_turn = 0;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    drain_pending(game);
}

fn play_land(game: &mut Game, card: GameObjectId) -> bool {
    let Some(action) = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card: played, .. } if *played == card))
    else {
        return false;
    };
    game.apply(PlayerId::One, action).expect("it is played");
    settle(game);
    true
}

/// Two land drops a turn, and no third.
#[test]
fn the_second_land_drop_is_allowed_and_a_third_is_not() {
    let (mut game, held) = staged(&[cards::FOREST, cards::FOREST, cards::FOREST], &[]);

    assert!(play_land(&mut game, held[0]), "the ordinary land drop");
    assert!(play_land(&mut game, held[1]), "and the additional one");
    assert!(!play_land(&mut game, held[2]), "but not a third");
}

/// Each land arriving mills a card.
#[test]
fn every_land_mills() {
    let (mut game, held) = staged(&[cards::FOREST, cards::FOREST], &[]);
    let library = game.players[0].library.len();

    play_land(&mut game, held[0]);
    assert_eq!(game.players[0].library.len(), library - 1, "one milled");

    play_land(&mut game, held[1]);
    assert_eq!(game.players[0].library.len(), library - 2, "and another");
    assert_eq!(game.players[0].graveyard.len(), 2, "both in the graveyard");
}

/// A land from the graveyard is a legal play, and the trigger sees it.
#[test]
fn a_land_may_be_played_out_of_the_graveyard() {
    let (mut game, _) = staged(&[], &[cards::MOUNTAIN]);
    let buried = game.players[0].graveyard[0].id;
    let library = game.players[0].library.len();

    assert!(
        play_land(&mut game, buried),
        "the graveyard is a legal zone"
    );

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::MOUNTAIN),
        "the land is on the battlefield",
    );
    assert_eq!(
        game.players[0].library.len(),
        library - 1,
        "and it milled on the way in like any other land",
    );
}

/// "Lands from your graveyard": a spell in it is still not castable.
#[test]
fn a_spell_in_the_graveyard_stays_there() {
    let (mut game, _) = staged(&[], &[cards::LIGHTNING_BOLT]);
    let buried = game.players[0].graveyard[0].id;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 2);

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == buried)),
        "the permission is about lands",
    );
}

/// Their land is not a land you control.
#[test]
fn their_land_mills_nothing() {
    let (mut game, _) = staged(&[], &[]);
    let library = game.players[0].library.len();

    game.put_onto_battlefield(PlayerId::Two, cards::FOREST)
        .expect("cataloged");
    settle(&mut game);

    assert_eq!(game.players[0].library.len(), library, "theirs is theirs");
}

/// A land put onto the battlefield is still a land entering, so it mills.
#[test]
fn a_land_put_onto_the_battlefield_mills_too() {
    let (mut game, _) = staged(&[], &[]);
    let library = game.players[0].library.len();

    game.put_onto_battlefield(PlayerId::One, cards::FOREST)
        .expect("cataloged");
    settle(&mut game);

    assert_eq!(
        game.players[0].library.len(),
        library - 1,
        "landfall is about entering rather than about playing",
    );
}
