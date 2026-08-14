//! Additional costs that spend an object.
//!
//! Distinct from a target: the object is chosen and spent as the spell is
//! cast, and never checked again. What these check is that the spell is not
//! offered with nothing to spend, that casting it really exiles the chosen
//! card, and that the choice is per-object rather than a single blanket
//! option.

use super::*;
use crate::ImplementationStatus;

/// Makeshift Mauler in hand, mana to cast it, and `fodder` creature cards in
/// the graveyard.
fn mauler_board(fodder: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    let mauler = card(10_000, cards::MAKESHIFT_MAULER, PlayerId::One);
    let mauler_id = mauler.id;
    game.players[PlayerId::One.index()].hand.push(mauler);
    game.players[PlayerId::One.index()].mana_pool.blue = 1;
    game.players[PlayerId::One.index()].mana_pool.colorless = 3;
    for index in 0..fodder {
        game.players[PlayerId::One.index()].graveyard.push(card(
            20_000 + u32::try_from(index).expect("small"),
            cards::SEDGE_TROLL,
            PlayerId::One,
        ));
    }
    (game, mauler_id)
}

fn cast_actions(game: &Game, card: GameObjectId) -> Vec<Vec<GameObjectId>> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell {
                card: actual,
                sacrifices,
                ..
            } if actual == card => Some(sacrifices),
            _ => None,
        })
        .collect()
}

#[test]
fn the_spell_is_not_castable_without_something_to_spend() {
    let (game, mauler) = mauler_board(0);
    assert!(
        cast_actions(&game, mauler).is_empty(),
        "an empty graveyard leaves no way to pay"
    );
}

/// One action per payable object, so the player picks which card leaves.
#[test]
fn each_payable_object_is_its_own_choice() {
    let (game, mauler) = mauler_board(2);
    let choices = cast_actions(&game, mauler);
    assert_eq!(choices.len(), 2, "two creature cards, two ways to pay");
    assert_ne!(
        choices[0], choices[1],
        "and they name different cards rather than repeating"
    );
}

#[test]
fn casting_exiles_the_chosen_card() {
    let (mut game, mauler) = mauler_board(1);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == mauler))
        .expect("it can be cast");
    game.apply(PlayerId::One, action)
        .expect("the spell is cast");
    drain_pending(&mut game);

    assert!(
        game.players[PlayerId::One.index()].graveyard.is_empty(),
        "the creature card left the graveyard"
    );
    assert_eq!(
        game.players[PlayerId::One.index()].exile.len(),
        1,
        "and went to exile rather than anywhere else"
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::MAKESHIFT_MAULER),
        "and the creature arrived"
    );
}

/// The predicate narrows what may be spent: a noncreature card in the
/// graveyard is not payment.
#[test]
fn only_matching_cards_can_be_spent() {
    let (mut game, mauler) = mauler_board(0);
    game.players[PlayerId::One.index()].graveyard.push(card(
        20_000,
        cards::LIGHTNING_BOLT,
        PlayerId::One,
    ));

    assert!(
        cast_actions(&game, mauler).is_empty(),
        "an instant is not a creature card"
    );
}

#[test]
fn every_additional_cost_identity_reports_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [
        cards::MAKESHIFT_MAULER,
        cards::STITCHED_DRAKE,
        cards::HEADLESS_SKAAB,
        cards::RELENTLESS_SKAABS,
    ] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
