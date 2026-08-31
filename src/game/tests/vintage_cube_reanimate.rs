//! Reanimate: one black mana for anything in any graveyard, and a bill in
//! life for it.
//!
//! What the card charges is read off the card in the graveyard, and it is
//! charged after the creature is already standing there. Both halves of that
//! are what these check.

use super::*;

/// Player One with one black mana and a Reanimate, and `corpse` in their own
/// graveyard for it to name.
fn staged(corpse: CardDefinitionId) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].graveyard.clear();
    let body = card(77_000, corpse, PlayerId::One);
    let body_id = body.id;
    game.players[PlayerId::One.index()].graveyard.push(body);
    let reanimate = card(77_001, cards::REANIMATE, PlayerId::One);
    let reanimate_id = reanimate.id;
    game.players[PlayerId::One.index()].hand.push(reanimate);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.priority = PlayerId::One;
    (game, reanimate_id, body_id)
}

fn cast_at(game: &mut Game, reanimate: GameObjectId, corpse: GameObjectId) {
    game.apply(
        PlayerId::One,
        cast_action(reanimate, vec![Target::Card(corpse)], Vec::new(), 0),
    )
    .expect("a creature card in a graveyard is what it names");
}

/// "If a card in a graveyard has {X} in its mana cost, X is 0." A Walking
/// Ballista is {X}{X}, so reanimating it is free -- and what arrives is the
/// 0/0 that mana value describes, which the game then puts straight back.
#[test]
fn an_x_in_the_graveyard_is_zero_life_and_a_zero_zero_body() {
    let (mut game, reanimate, ballista) = staged(cards::WALKING_BALLISTA);
    let life = game.players[PlayerId::One.index()].life;

    cast_at(&mut game, reanimate, ballista);
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life,
        "{{X}}{{X}} in a graveyard is mana value zero",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::WALKING_BALLISTA),
        "and X counters it never got are counters it does not have",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::WALKING_BALLISTA),
        "a 0/0 goes back where it came from",
    );
}

/// "You lose life after the creature is already on the battlefield", and
/// "if any abilities trigger on the creature entering, those abilities
/// resolve after you lose life." The Titan is standing and the six is paid
/// before its Zombies are anywhere.
#[test]
fn the_life_is_paid_with_the_titan_out_and_its_zombies_still_waiting() {
    let (mut game, reanimate, titan) = staged(cards::GRAVE_TITAN);
    cast_at(&mut game, reanimate, titan);
    pass_priority_pair(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::GRAVE_TITAN),
        "the Titan arrived first",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        14,
        "and six life was paid for a {{4}}{{B}}{{B}} card",
    );
    assert!(
        !game.pending_triggers.is_empty() || !game.stack.is_empty(),
        "its enters trigger is still waiting to resolve",
    );
    assert!(
        !game.battlefield.iter().any(|permanent| is_token_with(
            permanent,
            tokens::creature(&["Zombie"], &[ManaColor::Black], 2, 2)
        )),
        "so there are no Zombies yet",
    );

    drain_pending(&mut game);
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| is_token_with(
                permanent,
                tokens::creature(&["Zombie"], &[ManaColor::Black], 2, 2)
            ))
            .count(),
        2,
        "and two once it does",
    );
}

/// "If losing life results in you losing the game, those abilities won't
/// resolve." Six life is more than five, so the Titan is on the battlefield
/// and its controller is not in the game.
#[test]
fn dying_to_the_cost_leaves_the_enters_trigger_unresolved() {
    let (mut game, reanimate, titan) = staged(cards::GRAVE_TITAN);
    game.players[PlayerId::One.index()].life = 5;

    cast_at(&mut game, reanimate, titan);
    drain_pending(&mut game);

    assert_eq!(
        game.result,
        Some(GameResult::Winner {
            winner: PlayerId::Two,
            reason: WinReason::OpponentLostAllLife,
        }),
        "one life short of the Titan's mana value",
    );
    assert!(
        !game.battlefield.iter().any(|permanent| is_token_with(
            permanent,
            tokens::creature(&["Zombie"], &[ManaColor::Black], 2, 2)
        )),
        "the Zombies never arrived",
    );
}

/// "Target creature card from a graveyard": either graveyard, and the
/// creature arrives under the controller of the spell rather than its
/// owner's.
#[test]
fn it_reaches_across_the_table_and_keeps_what_it_takes() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::Two.index()].graveyard.clear();
    let angel = card(77_100, cards::SERRA_ANGEL, PlayerId::Two);
    let angel_id = angel.id;
    game.players[PlayerId::Two.index()].graveyard.push(angel);
    let reanimate = card(77_101, cards::REANIMATE, PlayerId::One);
    let reanimate_id = reanimate.id;
    game.players[PlayerId::One.index()].hand.push(reanimate);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.priority = PlayerId::One;

    cast_at(&mut game, reanimate_id, angel_id);
    drain_pending(&mut game);

    let angel = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SERRA_ANGEL)
        .expect("the Angel was reanimated");
    assert_eq!(angel.controller, PlayerId::One, "under your control");
    assert_eq!(
        angel.card.owner,
        PlayerId::Two,
        "though it is still their card",
    );
}
