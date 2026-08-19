//! The cards the Premodern Replenish list needed.

use super::*;

/// Replenish empties the graveyard of enchantments and leaves everything
/// else in it, including an enchantment that belongs to the opponent.
#[test]
fn replenish_returns_every_enchantment_you_own() {
    let mut game = ready_game();
    game.players[PlayerId::One.index()]
        .graveyard
        .push(card(10_010, cards::ENGINEERED_PLAGUE, PlayerId::One));
    game.players[PlayerId::One.index()]
        .graveyard
        .push(card(10_011, cards::SEAL_OF_CLEANSING, PlayerId::One));
    game.players[PlayerId::One.index()]
        .graveyard
        .push(card(10_012, cards::LIGHTNING_BOLT, PlayerId::One));
    game.players[PlayerId::Two.index()]
        .graveyard
        .push(card(10_013, cards::ENGINEERED_PLAGUE, PlayerId::Two));

    let replenish = card(10_000, cards::REPLENISH, PlayerId::One);
    let replenish_id = replenish.id;
    game.players[PlayerId::One.index()].hand.push(replenish);
    let pool = &mut game.players[PlayerId::One.index()].mana_pool;
    pool.white = 1;
    pool.colorless = 3;
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(replenish_id, Vec::new(), Vec::new(), 0),
    )
    .expect("four mana casts it");
    drain_pending(&mut game);

    assert_eq!(
        game.battlefield.len(),
        2,
        "both of your enchantments came back",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].graveyard.len(),
        2,
        "the Bolt stayed, and the Replenish joined it",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].graveyard.len(),
        1,
        "the opponent's enchantment is not yours to return",
    );
}

/// Frantic Search draws, discards, and hands the mana back: three lands
/// untap, which is what makes it free.
#[test]
fn frantic_search_untaps_three_of_the_lands_that_paid_for_it() {
    let mut game = ready_game();
    for index in 0..4 {
        let mut land = creature(10_010 + index, cards::ISLAND, PlayerId::One);
        land.tapped = true;
        game.battlefield.push(land);
    }
    for index in 0..3 {
        game.players[PlayerId::One.index()].library.push(card(
            10_030 + index,
            cards::COUNTERSPELL,
            PlayerId::One,
        ));
    }

    let search = card(10_000, cards::FRANTIC_SEARCH, PlayerId::One);
    let search_id = search.id;
    game.players[PlayerId::One.index()].hand.push(search);
    let pool = &mut game.players[PlayerId::One.index()].mana_pool;
    pool.blue = 1;
    pool.colorless = 2;
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(search_id, Vec::new(), Vec::new(), 0),
    )
    .expect("three mana casts it");

    // Two cards drawn and two discarded -- a hand of exactly two has no
    // choice to make -- and then the lands are chosen.
    pass_until_decision(&mut game);
    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("which lands to untap");
    let options = decision
        .options
        .iter()
        .take(3)
        .map(|option| option.id)
        .collect();
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options,
        },
    )
    .expect("three is the printed maximum");
    drain_pending(&mut game);

    let untapped = game
        .battlefield
        .iter()
        .filter(|permanent| !permanent.tapped)
        .count();
    assert_eq!(untapped, 3, "three of the four Islands came back up");
}

/// Attunement pays with itself and gets itself back: the enchantment leaves
/// the battlefield for its owner's hand, and the graveyard takes the four.
#[test]
fn attunement_returns_itself_and_fills_the_graveyard() {
    let mut game = ready_game();
    game.battlefield
        .push(creature(10_000, cards::ATTUNEMENT, PlayerId::One));
    for index in 0..3 {
        game.players[PlayerId::One.index()].hand.push(card(
            10_010 + index,
            cards::COUNTERSPELL,
            PlayerId::One,
        ));
    }
    for index in 0..5 {
        game.players[PlayerId::One.index()].library.push(card(
            10_020 + index,
            cards::ISLAND,
            PlayerId::One,
        ));
    }
    game.priority = PlayerId::One;

    let source = game.battlefield[0].card.id;
    let activate = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source: id, .. } if *id == source))
        .expect("the cost is the enchantment itself, so it is always payable");
    game.apply(PlayerId::One, activate).unwrap();
    pass_until_decision(&mut game);
    // Three drawn onto a hand of three, then four of those six discarded.
    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("which four cards go");
    // The Attunement is in hand by now and could be discarded like anything
    // else; keeping it is what shows it came back.
    let options: Vec<u32> = decision
        .options
        .iter()
        .filter(|option| option.label != "Attunement")
        .take(4)
        .map(|option| option.id)
        .collect();
    assert_eq!(options.len(), 4, "four cards other than the Attunement");
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options,
        },
    )
    .expect("four is what the card asks for");
    drain_pending(&mut game);

    let player = &game.players[PlayerId::One.index()];
    assert_eq!(player.graveyard.len(), 4, "four cards fed the graveyard");
    assert!(
        game.battlefield.is_empty(),
        "the enchantment left the battlefield to pay",
    );
    assert!(
        player
            .hand
            .iter()
            .any(|card| card.definition == cards::ATTUNEMENT),
        "and it is back in hand to be cast again",
    );
}
