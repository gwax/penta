//! Lotus Petal: a free artifact that is one mana of any colour, once.
//!
//! That it cracks the turn it lands, for any of the five, is pinned in
//! `premodern_cards`. What is here is the cast from hand that the cube plays
//! it for: nothing spent to play it, and the mana it makes spent on
//! something else the same turn.

use super::*;

/// Player One with a Petal in hand and nothing else.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].mana_pool = ManaPool::default();
    let petal = game
        .build_zone(PlayerId::One, &[cards::LOTUS_PETAL])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = petal.id;
    game.players[0].hand.push(petal);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, id)
}

/// Casts the Petal for nothing and returns the permanent it became.
fn play(game: &mut Game, petal: GameObjectId) -> GameObjectId {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == petal))
        .expect("an artifact costing nothing is castable with an empty pool");
    game.apply(PlayerId::One, action).expect("it is cast");
    pass_priority_pair(game);
    drain_pending(game);
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::LOTUS_PETAL)
        .expect("it arrived")
        .card
        .id
}

fn crack(game: &mut Game, petal: GameObjectId, color: ManaColor) {
    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: petal,
            ability: mana_ability_for(game, petal, color),
            color,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .unwrap_or_else(|error| panic!("it makes {color:?}: {error}"));
}

/// Cast for nothing out of an empty pool, and cracked the same turn: an
/// artifact is not a creature, so nothing about the turn it arrived stops it
/// tapping.
#[test]
fn it_is_free_to_cast_and_ready_the_turn_it_lands() {
    let (mut game, petal) = staged();
    let permanent = play(&mut game, petal);
    assert_eq!(
        game.battlefield
            .iter()
            .find(|candidate| candidate.card.id == permanent)
            .expect("it is there")
            .entered_controller_turn,
        game.turns_started[PlayerId::One.index()],
        "it arrived this turn",
    );

    crack(&mut game, permanent, ManaColor::Blue);

    assert_eq!(game.players[0].mana_pool.blue, 1, "and taps regardless");
}

/// The mana is mana, and the engine offers the cast it pays for: a blue
/// spell is uncastable on an empty board and castable beside a Petal, which
/// is spent on the way.
#[test]
fn the_mana_it_makes_casts_a_spell() {
    let (mut game, petal) = staged();
    let brainstorm = game
        .build_zone(PlayerId::One, &[cards::BRAINSTORM])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let brainstorm_id = brainstorm.id;
    game.players[0].hand.push(brainstorm);
    let cast_of = |game: &Game| {
        game.legal_actions(PlayerId::One).into_iter().find(
            |action| matches!(action, Action::CastSpell { card, .. } if *card == brainstorm_id),
        )
    };
    assert!(
        cast_of(&game).is_none(),
        "with nothing on the battlefield there is no blue to be had",
    );

    play(&mut game, petal);
    let cast = cast_of(&game).expect("the Petal is the blue mana it needs");
    game.apply(PlayerId::One, cast).expect("it is cast");

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::LOTUS_PETAL),
        "the Petal was cracked to pay for it",
    );
    assert!(
        game.stack
            .iter()
            .any(|object| object.card.definition.card_definition() == Some(cards::BRAINSTORM)),
        "and the spell is on the stack",
    );
}
