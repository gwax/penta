//! Daze: a counter that costs an Island rather than mana, and asks for one
//! mana rather than an answer.
//!
//! Which lands pay the alternative cost, that the printed cost is still
//! there without one, and that a free Daze is still a two-mana spell are
//! pinned in `premodern_free_spells`. What is here is the other half of the
//! card: what the Island costs and what the tax buys.

use super::*;

/// Player Two casting a Serra Angel with `spare` mana left over, Player One
/// holding a Daze behind an Island.
fn staged(spare: u16) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    game.players[1].graveyard.clear();
    game.put_onto_battlefield(PlayerId::One, cards::ISLAND)
        .expect("cataloged");
    let daze = game
        .build_zone(PlayerId::One, &[cards::DAZE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let daze_id = daze.id;
    game.players[0].hand.push(daze);
    let angel = game
        .build_zone(PlayerId::Two, &[cards::SERRA_ANGEL])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let angel_id = angel.id;
    game.players[1].hand.push(angel);
    drain_pending(&mut game);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::White, 5);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Colorless, spare);

    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == angel_id))
        .expect("five mana casts the Angel");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    for _ in 0..4 {
        if game.priority == PlayerId::One {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    (game, daze_id, angel_id)
}

/// Casts the Daze for its Island, then answers the tax.
fn daze_them(game: &mut Game, daze: GameObjectId, pay: bool) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == daze && choices.costs().alternative().is_some())
        })
        .expect("an Island pays for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::ISLAND),
        "the Island went back to hand as the Daze was announced",
    );
    assert!(
        game.battlefield.is_empty(),
        "and left the battlefield to do it",
    );

    for _ in 0..8 {
        let Some(decision) = game.observe(PlayerId::Two).decision else {
            let priority = game.priority;
            if game.apply(priority, Action::PassPriority).is_err() {
                break;
            }
            continue;
        };
        let wanted = if pay { "Pay the cost" } else { "Decline" };
        let option = decision
            .options
            .iter()
            .find(|option| option.label == wanted)
            .unwrap_or(&decision.options[0])
            .id;
        game.apply(
            PlayerId::Two,
            Action::ChooseDecision {
                decision: decision.id,
                options: vec![option],
            },
        )
        .expect("the answer is legal");
    }
    drain_pending(game);
}

/// One mana keeps the spell, and the Island is spent either way.
#[test]
fn paying_one_keeps_the_spell() {
    let (mut game, daze, _angel) = staged(1);

    daze_them(&mut game, daze, true);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::SERRA_ANGEL),
        "the Angel resolved",
    );
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::ISLAND),
        "and the Island is in hand, bought for nothing",
    );
}

/// A player tapped out has nothing to pay it with, which is the whole card:
/// one Island for their turn.
#[test]
fn a_tapped_out_player_loses_the_spell() {
    let (mut game, daze, _angel) = staged(0);

    daze_them(&mut game, daze, false);

    assert!(game.battlefield.is_empty(), "the Angel never arrived");
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "it was countered into their graveyard",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::DAZE),
        "and the Daze itself is spent",
    );
}
