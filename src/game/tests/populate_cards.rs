//! Two cards whose audit lines said populate was unavailable.
//!
//! It was not: populate has its own procedure and its own tests already.
//! What is worth pinning here is what each card wraps around it -- a
//! self-counting body on one, and an ordering on the other, since the
//! destruction happens before the copy is chosen.

use super::*;
use crate::ImplementationStatus;

fn ready() -> Game {
    let mut game = ready_game();
    game.turn = 5;
    game.turns_started[PlayerId::One.index()] = 5;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game
}

fn stats(game: &Game, id: GameObjectId) -> (Option<i16>, Option<i16>) {
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("still there");
    (game.power(permanent), game.toughness(permanent))
}

fn tokens(game: &Game, definition: CardDefinitionId) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.card.definition == definition)
        .count()
}

/// Alone, the Temple counts only itself.
#[test]
fn the_temple_is_a_one_one_on_an_empty_board() {
    let mut game = ready();
    let temple = creature(10_000, cards::WAYFARING_TEMPLE, PlayerId::One);
    let temple_id = temple.card.id;
    game.battlefield.push(temple);

    assert_eq!(stats(&game, temple_id), (Some(1), Some(1)));
}

/// The count is live and covers only creatures its controller has.
#[test]
fn the_temple_grows_with_your_own_creatures() {
    let mut game = ready();
    let temple = creature(10_000, cards::WAYFARING_TEMPLE, PlayerId::One);
    let temple_id = temple.card.id;
    game.battlefield.push(temple);
    for index in 0..2 {
        game.battlefield.push(creature(
            10_100 + index,
            cards::GRIZZLY_BEARS,
            PlayerId::One,
        ));
    }
    game.battlefield
        .push(creature(10_200, cards::GRIZZLY_BEARS, PlayerId::Two));

    assert_eq!(
        stats(&game, temple_id),
        (Some(3), Some(3)),
        "itself and two others; theirs does not count",
    );

    game.battlefield
        .retain(|permanent| permanent.card.id != CardInstanceId(10_100));
    assert_eq!(stats(&game, temple_id), (Some(2), Some(2)));
}

/// Sundering Growth destroys and then populates, in that order.
#[test]
fn sundering_growth_destroys_then_copies_a_token() {
    let mut game = ready();
    let mox = creature(10_000, cards::MOX_JET, PlayerId::Two);
    let mox_id = mox.card.id;
    game.battlefield.push(mox);
    game.battlefield.push(creature(
        10_100,
        cards::ZOMBIE_TOKEN_2_2_BLACK,
        PlayerId::One,
    ));

    let spell = card(20_000, cards::SUNDERING_GROWTH, PlayerId::One);
    let spell_id = spell.id;
    game.players[PlayerId::One.index()].hand.push(spell);
    game.players[PlayerId::One.index()].mana_pool.green = 2;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell_id))
        .expect("two green pays a hybrid cost");
    game.apply(PlayerId::One, action)
        .expect("the cast is legal");
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == mox_id),
        "the artifact went",
    );
    assert_eq!(
        tokens(&game, cards::ZOMBIE_TOKEN_2_2_BLACK),
        2,
        "and the token was copied",
    );
}

/// A board with no creature token is not a failure: the destruction still
/// happens and populate simply does nothing.
#[test]
fn sundering_growth_still_destroys_with_no_token_to_copy() {
    let mut game = ready();
    let mox = creature(10_000, cards::MOX_JET, PlayerId::Two);
    let mox_id = mox.card.id;
    game.battlefield.push(mox);

    let spell = card(20_000, cards::SUNDERING_GROWTH, PlayerId::One);
    let spell_id = spell.id;
    game.players[PlayerId::One.index()].hand.push(spell);
    game.players[PlayerId::One.index()].mana_pool.white = 2;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell_id))
        .expect("two white pays it too");
    game.apply(PlayerId::One, action)
        .expect("the cast is legal");
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == mox_id),
    );
    assert_eq!(tokens(&game, cards::ZOMBIE_TOKEN_2_2_BLACK), 0);
}

#[test]
fn both_cards_report_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [cards::WAYFARING_TEMPLE, cards::SUNDERING_GROWTH] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
