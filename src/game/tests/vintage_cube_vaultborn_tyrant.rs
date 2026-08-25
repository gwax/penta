//! Vaultborn Tyrant: seven mana that draws a card as it lands, and hands the
//! same body back when it dies.

use super::*;

fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..8 {
        game.players[0]
            .library
            .push(card(93_000 + index, cards::ISLAND, PlayerId::One));
    }
    let tyrant = game
        .put_onto_battlefield(PlayerId::One, cards::VAULTBORN_TYRANT)
        .expect("cataloged");
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    drain_pending(&mut game);
    (game, tyrant)
}

fn tokens(game: &Game) -> Vec<&Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == ObjectKind::Token)
        .collect()
}

fn kill(game: &mut Game, permanent: GameObjectId) {
    game.destroy_permanent(permanent);
    drain_pending(game);
    game.check_state_based_actions();
}

/// Its own arrival triggers it: a 6/6 is a creature with power 4 or greater.
#[test]
fn it_pays_for_itself_on_the_way_in() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..8 {
        game.players[0]
            .library
            .push(card(93_100 + index, cards::ISLAND, PlayerId::One));
    }
    let life = game.players[0].life;

    game.put_onto_battlefield(PlayerId::One, cards::VAULTBORN_TYRANT)
        .expect("cataloged");
    drain_pending(&mut game);

    assert_eq!(game.players[0].life, life + 3, "three life");
    assert_eq!(game.players[0].hand.len(), 1, "and a card");
}

/// Another big creature does it again; a small one does not, and neither
/// does one the other player plays.
#[test]
fn only_your_own_big_creatures_trigger_it() {
    let (mut game, _tyrant) = staged();
    let life = game.players[0].life;
    let hand = game.players[0].hand.len();

    game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    assert_eq!(game.players[0].life, life, "a 2/2 is not big enough");

    game.put_onto_battlefield(PlayerId::Two, cards::SHIVAN_DRAGON)
        .expect("cataloged");
    drain_pending(&mut game);
    assert_eq!(game.players[0].life, life, "and theirs is not yours");

    game.put_onto_battlefield(PlayerId::One, cards::SHIVAN_DRAGON)
        .expect("cataloged");
    drain_pending(&mut game);

    assert_eq!(game.players[0].life, life + 3);
    assert_eq!(game.players[0].hand.len(), hand + 1);
}

/// Killing it gives the body back as an artifact copy.
#[test]
fn dying_leaves_an_artifact_copy_of_itself() {
    let (mut game, tyrant) = staged();

    kill(&mut game, tyrant);

    let copies = tokens(&game);
    assert_eq!(copies.len(), 1, "one copy");
    let copy = copies[0];
    assert_eq!(game.power(copy), Some(6), "the same body");
    assert_eq!(game.toughness(copy), Some(6));
    assert!(
        game.permanent_types(copy)
            .expect("the copy has types")
            .contains(CardType::Artifact),
        "an artifact in addition to its other types",
    );
    assert!(
        game.permanent_types(copy)
            .expect("the copy has types")
            .contains(CardType::Creature),
        "and still a creature",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::VAULTBORN_TYRANT),
        "the card itself is in the graveyard",
    );
}

/// The copy is itself a creature with power 4 or greater entering, so it
/// pays its controller as it arrives.
#[test]
fn the_copy_pays_on_its_way_in_too() {
    let (mut game, tyrant) = staged();
    let life = game.players[0].life;
    let hand = game.players[0].hand.len();

    kill(&mut game, tyrant);

    assert_eq!(game.players[0].life, life + 3, "the copy triggers it");
    assert_eq!(game.players[0].hand.len(), hand + 1);
}

/// "If it's not a token": the copy dying leaves nothing behind, so the
/// Tyrant comes back once rather than forever.
#[test]
fn the_copy_does_not_copy_itself() {
    let (mut game, tyrant) = staged();
    kill(&mut game, tyrant);
    let copy = tokens(&game)[0].card.id;

    kill(&mut game, copy);

    assert!(tokens(&game).is_empty(), "a token leaves nothing behind");
}
