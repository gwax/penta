//! Oust: one white mana answers anything, and hands it back two draws later.

use super::*;

/// A game with `library` cards under Player Two's library top and an Oust in
/// Player One's hand.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].library.clear();
    for index in 0..4 {
        game.players[1]
            .library
            .push(card(95_100 + index, cards::MOUNTAIN, PlayerId::Two));
    }
    // The library reads from the back, so this is the card on top.
    game.players[1]
        .library
        .push(card(95_200, cards::ISLAND, PlayerId::Two));
    let oust = game
        .build_zone(PlayerId::One, &[cards::OUST])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let oust_id = oust.id;
    game.players[0].hand.push(oust);
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    (game, oust_id)
}

fn cast_oust(game: &mut Game, oust: GameObjectId, victim: GameObjectId) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == oust
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(victim))
            }
            _ => false,
        })
        .expect("the creature is a legal target");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

/// The library from the top down, by definition.
fn library_from_top(game: &Game, player: PlayerId) -> Vec<CardDefinitionId> {
    game.players[player.index()]
        .library
        .iter()
        .rev()
        .map(|card| card.definition)
        .collect()
}

/// It goes under the top card, so the next draw is the card that was
/// already there.
#[test]
fn the_creature_lands_second_from_the_top() {
    let (mut game, oust) = staged();
    let bears = creature(95_000, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let before = game.players[1].library.len();

    cast_oust(&mut game, oust, bears_id);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "the creature left the battlefield",
    );
    assert_eq!(game.players[1].library.len(), before + 1);
    assert_eq!(
        library_from_top(&game, PlayerId::Two)[..2],
        [cards::ISLAND, cards::GRIZZLY_BEARS],
        "under the card that was on top",
    );
}

/// Three life is the price, and it is paid to the creature's controller.
#[test]
fn its_controller_gains_three_life() {
    let (mut game, oust) = staged();
    let bears = creature(95_000, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let theirs = game.players[1].life;
    let mine = game.players[0].life;

    cast_oust(&mut game, oust, bears_id);

    assert_eq!(game.players[1].life, theirs + 3);
    assert_eq!(game.players[0].life, mine, "and never to you");
}

/// A stolen creature separates the two halves: the card goes to its owner's
/// library, and the player who controlled it is the one paid.
#[test]
fn a_stolen_creature_pays_the_thief_and_returns_to_its_owner() {
    let (mut game, oust) = staged();
    let mut bears = creature(95_000, cards::GRIZZLY_BEARS, PlayerId::Two);
    // Owned by Player Two, controlled by Player One: the thief is the one
    // holding it when the Oust resolves.
    bears.controller = PlayerId::One;
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let theirs = game.players[1].life;
    let mine = game.players[0].life;

    cast_oust(&mut game, oust, bears_id);

    assert_eq!(
        library_from_top(&game, PlayerId::Two)[..2],
        [cards::ISLAND, cards::GRIZZLY_BEARS],
        "the card goes into its owner's library",
    );
    assert_eq!(
        game.players[0].life,
        mine + 3,
        "and the player who controlled it gains the life",
    );
    assert_eq!(game.players[1].life, theirs);
}

/// A shallow library takes it as deep as it goes: with one card left, the
/// creature lands underneath it.
#[test]
fn a_one_card_library_puts_it_on_the_bottom() {
    let (mut game, oust) = staged();
    game.players[1].library.clear();
    game.players[1]
        .library
        .push(card(95_300, cards::ISLAND, PlayerId::Two));
    let bears = creature(95_000, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);

    cast_oust(&mut game, oust, bears_id);

    assert_eq!(
        library_from_top(&game, PlayerId::Two),
        vec![cards::ISLAND, cards::GRIZZLY_BEARS],
        "one card above it is all there was",
    );
}

/// "If the targeted creature's owner has no cards left in their library,
/// that creature is put into that library as the only card there." Second
/// from the top of nothing is the top.
#[test]
fn an_empty_library_takes_it_as_its_only_card() {
    let (mut game, oust) = staged();
    game.players[1].library.clear();
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    cast_oust(&mut game, oust, bears);

    assert_eq!(
        library_from_top(&game, PlayerId::Two),
        vec![cards::GRIZZLY_BEARS],
        "it is the whole library now",
    );
    assert_eq!(game.players[1].life, 23, "and they were paid all the same");
}

/// "If the targeted creature is a token, it will cease to exist after it's
/// put into its owner's library." The library is no fuller for it, and the
/// three life is paid regardless.
#[test]
fn a_token_ceases_to_exist_and_the_life_is_still_paid() {
    let (mut game, oust) = staged();
    game.create_token(
        PlayerId::Two,
        tokens::creature(&["Bear"], &[ManaColor::Green], 2, 2),
    );
    drain_pending(&mut game);
    let token = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Token)
        .expect("the token is out")
        .card
        .id;
    game.priority = PlayerId::One;
    let library_before = game.players[1].library.len();

    cast_oust(&mut game, oust, token);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != token),
        "it left the battlefield",
    );
    assert_eq!(
        game.players[1].library.len(),
        library_before,
        "and a token that leaves ceases to exist rather than joining a library",
    );
    assert_eq!(game.players[1].life, 23, "the three life was paid anyway");
}

/// "If the targeted creature is an illegal target by the time Oust resolves,
/// the spell doesn't resolve. No one gains life."
#[test]
fn a_target_that_is_gone_pays_nobody() {
    let (mut game, oust) = staged();
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    let library_before = game.players[1].library.len();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == oust
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(bears))
            }
            _ => false,
        })
        .expect("the bear is a legal target");
    game.apply(PlayerId::One, cast).expect("it is cast");

    game.move_permanents_to_graveyard(&[bears]);
    game.check_state_based_actions();
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        if game.apply(game.priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();

    assert_eq!(game.players[1].life, 20, "nobody gained anything");
    assert_eq!(
        game.players[1].library.len(),
        library_before,
        "and the library is as it was",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::OUST),
        "the Oust itself went to the graveyard",
    );
}
