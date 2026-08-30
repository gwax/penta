//! Cabal Ritual: Dark Ritual with a late-game mode, and the graveyard says
//! which one you get.

use super::*;

/// Player One holding a Cabal Ritual with one black mana up and `buried`
/// cards in the graveyard.
fn staged(buried: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    for index in 0..buried {
        let id = 71_000 + u32::try_from(index).expect("a handful of cards");
        game.players[0]
            .graveyard
            .push(card(id, cards::MOUNTAIN, PlayerId::One));
    }
    let ritual = game
        .build_zone(PlayerId::One, &[cards::CABAL_RITUAL])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let ritual_id = ritual.id;
    game.players[0].hand.push(ritual);
    game.turns_started = [4, 4];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 2);
    (game, ritual_id)
}

/// Casts it and lets it resolve, returning the black mana left in the pool.
fn cast(game: &mut Game, ritual: GameObjectId) -> u16 {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == ritual))
        .expect("two mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    pass_priority_pair(game);
    drain_pending(game);
    game.players[0].mana_pool.black
}

/// Six cards is not threshold: three black, which is one more than it cost.
#[test]
fn without_threshold_it_adds_three() {
    let (mut game, ritual) = staged(6);

    assert_eq!(cast(&mut game, ritual), 3, "the printed amount");
}

/// Seven is, and then it adds five.
#[test]
fn with_threshold_it_adds_five() {
    let (mut game, ritual) = staged(7);

    assert_eq!(cast(&mut game, ritual), 5, "the threshold amount instead");
}

/// The count is taken as the spell resolves rather than as it is cast, so a
/// card that reaches the graveyard in between counts.
#[test]
fn the_graveyard_is_counted_on_resolution() {
    let (mut game, ritual) = staged(6);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == ritual))
        .expect("two mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");

    // A seventh card arrives while the Ritual is still on the stack.
    game.players[0]
        .graveyard
        .push(card(71_900, cards::MOUNTAIN, PlayerId::One));
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].mana_pool.black, 5,
        "threshold was reached before it resolved",
    );
}

/// "Cards in your graveyard": theirs is a different graveyard, however deep
/// it is.
#[test]
fn only_your_own_graveyard_counts() {
    let (mut game, ritual) = staged(6);
    game.players[1].graveyard.clear();
    for index in 0..10 {
        let id = 71_500 + index;
        game.players[1]
            .graveyard
            .push(card(id, cards::MOUNTAIN, PlayerId::Two));
    }

    assert_eq!(
        cast(&mut game, ritual),
        3,
        "ten of theirs and six of yours is still six",
    );
}

/// The Ritual is on the stack while it resolves, not in the graveyard, so
/// the card that would have been the seventh is itself. Six behind it is
/// three mana, and the seventh card only arrives once the mana is already
/// added.
#[test]
fn it_does_not_count_itself() {
    let (mut game, ritual) = staged(6);

    assert_eq!(cast(&mut game, ritual), 3, "it counted six, not seven");
    assert_eq!(
        game.players[0].graveyard.len(),
        7,
        "and then it became the seventh",
    );
}
