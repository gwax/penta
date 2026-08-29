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

/// "The effect ... is cumulative with similar effects": an Exploration
/// beside the Explorer is a third land drop, not a wasted one.
#[test]
fn an_exploration_beside_it_is_a_third_drop() {
    let (mut game, held) = staged(
        &[cards::FOREST, cards::FOREST, cards::FOREST, cards::FOREST],
        &[],
    );
    game.put_onto_battlefield(PlayerId::One, cards::EXPLORATION)
        .expect("cataloged");
    drain_pending(&mut game);
    game.players[0].lands_played_this_turn = 0;
    game.priority = PlayerId::One;

    assert!(play_land(&mut game, held[0]), "the first");
    assert!(play_land(&mut game, held[1]), "the second");
    assert!(play_land(&mut game, held[2]), "and the third");
    assert!(
        !play_land(&mut game, held[3]),
        "three is where the two allowances stop",
    );
}

/// "It doesn't allow you to activate abilities (such as cycling) of land
/// cards in your graveyard", and it "doesn't change the times when you can
/// play those land cards" either: the permission says where, not what or
/// when.
#[test]
fn the_graveyard_permission_is_only_for_playing_them() {
    let (mut game, _held) = staged(&[], &[cards::RAFFINES_TOWER]);
    let tower = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == cards::RAFFINES_TOWER)
        .expect("it is in the graveyard")
        .id;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::PlayLand { card, .. } if *card == tower)),
        "the land itself may be played out of the graveyard",
    );
    assert!(
        !game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == tower)
        ),
        "but its cycling stays where the card is",
    );

    game.active_player = PlayerId::Two;
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::PlayLand { card, .. } if *card == tower)),
        "and a land is played on your own turn, graveyard or not",
    );
}
