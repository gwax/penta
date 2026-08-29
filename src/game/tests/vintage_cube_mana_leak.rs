//! Mana Leak: two mana that answers anything early and nothing late, since
//! the three it asks for is paid out of whatever is available right then.

use super::*;

/// Player One holding a Leak with the mana for it, and Player Two holding a
/// Lightning Bolt with `lands` untapped Islands to pay with.
fn staged(lands: usize) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    for index in 0..lands {
        let mut island = creature(
            122_000 + u32::try_from(index).expect("a few lands"),
            cards::ISLAND,
            PlayerId::Two,
        );
        island.entered_controller_turn = 0;
        game.battlefield.push(island);
    }
    let leak = card(122_100, cards::MANA_LEAK, PlayerId::One);
    let leak_id = leak.id;
    game.players[0].hand.push(leak);
    let bolt = card(122_101, cards::LIGHTNING_BOLT, PlayerId::Two);
    let bolt_id = bolt.id;
    game.players[1].hand.push(bolt);
    game.players[0].life = 20;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 2);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;
    (game, leak_id, bolt_id)
}

/// Casts the Bolt at Player One, answers it with the Leak, and stops with
/// whatever the Leak is asking.
fn bolt_and_leak(game: &mut Game, leak: GameObjectId, bolt: GameObjectId) {
    game.apply(
        PlayerId::Two,
        cast_action(bolt, vec![Target::Player(PlayerId::One)], Vec::new(), 0),
    )
    .expect("one red casts it");
    let on_stack = game.stack.last().expect("the Bolt is on the stack").id;
    game.apply(PlayerId::Two, Action::PassPriority)
        .expect("they pass");
    game.apply(
        PlayerId::One,
        cast_action(leak, vec![Target::Spell(on_stack)], Vec::new(), 0),
    )
    .expect("two mana answers it");
    pass_priority_pair(game);
}

/// The three is paid as the Leak resolves, out of whatever is available
/// then: an empty pool and three untapped lands is enough.
#[test]
fn the_three_comes_out_of_untapped_lands() {
    let (mut game, leak, bolt) = staged(3);
    bolt_and_leak(&mut game, leak, bolt);

    let decision = game
        .observe(PlayerId::Two)
        .decision
        .expect("the Bolt's controller is asked for three");
    let pay = decision
        .options
        .iter()
        .find(|option| option.label == "Pay the cost")
        .expect("three lands can pay it")
        .id;
    game.apply(
        PlayerId::Two,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![pay],
        },
    )
    .expect("paying is legal");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].life, 17,
        "the Bolt was paid for and resolved"
    );
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.controller == PlayerId::Two && permanent.tapped)
            .count(),
        3,
        "and the three lands are what paid",
    );
}

/// Two lands is not three: with nothing that can pay, there is nothing to
/// ask, and the Bolt is countered.
#[test]
fn two_lands_is_not_three() {
    let (mut game, leak, bolt) = staged(2);
    bolt_and_leak(&mut game, leak, bolt);

    assert!(
        game.observe(PlayerId::Two)
            .decision
            .is_none_or(|decision| !decision
                .options
                .iter()
                .any(|option| option.label == "Pay the cost")),
        "there is no three to be found",
    );
    drain_pending(&mut game);
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert_eq!(game.players[0].life, 20, "the Bolt never resolved");
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "it was countered instead",
    );
}
