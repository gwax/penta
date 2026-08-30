//! Phlage, Titan of Fire's Fury: a Lightning Helix that sacrifices itself
//! until the graveyard is deep enough to escape.

use super::*;

/// Player One with a Phlage in hand and `fodder` other cards in the
/// graveyard, ready to pay for either way of casting it.
fn staged(fodder: usize) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].graveyard.clear();
    for _ in 0..fodder {
        let card = game
            .build_zone(PlayerId::One, &[cards::MOUNTAIN])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].graveyard.push(card);
    }
    let phlage = game
        .build_zone(PlayerId::One, &[cards::PHLAGE_TITAN_OF_FIRES_FURY])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let phlage_id = phlage.id;
    game.players[0].hand.push(phlage);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 3);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 3);
    (game, phlage_id)
}

/// Moves the Phlage from hand to graveyard, standing in for it having died
/// or been discarded, and hands back its graveyard identity.
fn bury(game: &mut Game, phlage: GameObjectId) -> GameObjectId {
    let card = remove_card(&mut game.players[0].hand, phlage).expect("it is in hand");
    let (card, _zone_change) = game.zone_change_card(card);
    let id = card.id;
    game.players[0].graveyard.push(card);
    id
}

fn casts_of(game: &Game, card: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| matches!(action, Action::CastSpell { card: id, .. } if *id == card))
        .collect()
}

/// Answers the target choice with the opponent and resolves everything.
fn settle(game: &mut Game) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            // Two triggers waiting at once are ordered rather than picked
            // between, and that decision wants every option.
            let options = if decision.minimum > 1 {
                decision.options.iter().map(|option| option.id).collect()
            } else {
                decision
                    .options
                    .iter()
                    .find(|option| option.label == "your opponent")
                    .or_else(|| decision.options.first())
                    .map(|option| vec![option.id])
                    .unwrap_or_default()
            };
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the decision accepts what it offered");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            return;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            return;
        }
    }
}

fn on_battlefield(game: &Game) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.definition == cards::PHLAGE_TITAN_OF_FIRES_FURY)
}

/// Cast for its printed cost it helixes and then sacrifices itself.
#[test]
fn cast_from_hand_it_helixes_and_dies() {
    let (mut game, phlage) = staged(0);
    game.priority = PlayerId::One;
    let cast = casts_of(&game, phlage)
        .into_iter()
        .next()
        .expect("three mana buys it from hand");
    game.apply(PlayerId::One, cast).expect("it is castable");
    settle(&mut game);
    game.check_state_based_actions();

    assert_eq!(game.players[1].life, 17, "three damage");
    assert_eq!(game.players[0].life, 23, "and three life");
    assert!(!on_battlefield(&game), "it sacrificed itself");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::PHLAGE_TITAN_OF_FIRES_FURY),
        "and lies in the graveyard, ready to escape later",
    );
}

/// Four cards is not five, so there is no way to escape it yet.
#[test]
fn four_cards_are_not_enough_to_escape() {
    let (mut game, phlage) = staged(4);
    let buried = bury(&mut game, phlage);
    game.priority = PlayerId::One;

    assert!(
        casts_of(&game, buried).is_empty(),
        "escape wants five other cards and the graveyard has four",
    );
}

/// With five to exile it escapes, stays on the battlefield, and helixes.
#[test]
fn escaping_keeps_it_around() {
    let (mut game, phlage) = staged(5);
    let buried = bury(&mut game, phlage);
    game.priority = PlayerId::One;
    let cast = casts_of(&game, buried)
        .into_iter()
        .next()
        .expect("five cards and four mana is an escape");
    game.apply(PlayerId::One, cast).expect("it escapes");
    settle(&mut game);
    game.check_state_based_actions();

    assert!(on_battlefield(&game), "an escaped Phlage stays");
    assert_eq!(game.players[1].life, 17);
    assert_eq!(game.players[0].life, 23);
    assert!(
        game.players[0].graveyard.is_empty(),
        "the five it exiled are gone from the graveyard",
    );
    assert_eq!(game.players[0].exile.len(), 5);
}

/// Attacking fires the same ability the entry did.
#[test]
fn attacking_helixes_again() {
    let (mut game, phlage) = staged(5);
    let buried = bury(&mut game, phlage);
    game.priority = PlayerId::One;
    let cast = casts_of(&game, buried)
        .into_iter()
        .next()
        .expect("five cards and four mana is an escape");
    game.apply(PlayerId::One, cast).expect("it escapes");
    settle(&mut game);

    let attacker = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::PHLAGE_TITAN_OF_FIRES_FURY)
        .expect("it is on the battlefield")
        .card
        .id;
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == attacker)
    {
        permanent.entered_controller_turn = 0;
    }
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("a 6/6 with no summoning sickness may attack");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration is complete");
    settle(&mut game);

    assert_eq!(game.players[1].life, 14, "another three");
    assert_eq!(game.players[0].life, 26);
}

/// "After an escaped spell resolves, it enters the battlefield and will
/// return to its owner's graveyard if it dies later. It can escape again."
/// The second escape costs another five cards, and the body it brings back
/// helixes again on the way in.
#[test]
fn it_escapes_a_second_time_from_the_graveyard_it_died_into() {
    let (mut game, phlage) = staged(10);
    let buried = bury(&mut game, phlage);
    game.priority = PlayerId::One;
    let cast = casts_of(&game, buried)
        .into_iter()
        .next()
        .expect("five of the ten and four mana is an escape");
    game.apply(PlayerId::One, cast).expect("it escapes");
    settle(&mut game);
    game.check_state_based_actions();
    assert!(on_battlefield(&game), "the first escape stuck");
    assert_eq!(game.players[0].graveyard.len(), 5, "five cards are left");

    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::PHLAGE_TITAN_OF_FIRES_FURY)
        .expect("it is there")
        .card
        .id;
    game.move_permanents_to_graveyard(&[body]);
    game.check_state_based_actions();
    let returned = game.players[0]
        .graveyard
        .iter()
        .find(|card| card.definition == cards::PHLAGE_TITAN_OF_FIRES_FURY)
        .expect("it died back into the graveyard")
        .id;

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 3);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 3);
    game.priority = PlayerId::One;
    let again = casts_of(&game, returned)
        .into_iter()
        .next()
        .expect("the other five pay for a second escape");
    game.apply(PlayerId::One, again).expect("it escapes again");
    settle(&mut game);
    game.check_state_based_actions();

    assert!(on_battlefield(&game), "and it stays a second time");
    assert_eq!(
        game.players[1].life, 14,
        "two helixes across the two escapes",
    );
    assert_eq!(game.players[0].exile.len(), 10, "ten cards paid for them");
}

/// "Phlage's first ability causes you to sacrifice it if you didn't cast it,
/// or if it was cast using any permission other than an escape ability."
/// Reanimating it buys one Helix and nothing else.
#[test]
fn put_onto_the_battlefield_it_still_sacrifices_itself() {
    let (mut game, _phlage) = staged(0);
    let mine = game.players[0].life;
    let theirs = game.players[1].life;
    game.put_onto_battlefield(PlayerId::One, cards::PHLAGE_TITAN_OF_FIRES_FURY)
        .expect("cataloged");
    settle(&mut game);

    assert!(
        !on_battlefield(&game),
        "arriving without a cast is not escaping",
    );
    assert_eq!(game.players[1].life, theirs - 3, "the Helix still happened");
    assert_eq!(game.players[0].life, mine + 3, "on both halves");
}

/// "Escape's permission doesn't change when you may cast the spell from your
/// graveyard." It is still a creature spell, so it waits for your own main
/// phase like any other.
#[test]
fn escape_is_still_a_creature_spell() {
    let (mut game, phlage) = staged(5);
    let buried = bury(&mut game, phlage);
    assert!(
        !casts_of(&game, buried).is_empty(),
        "five cards and four mana escape it on your own main phase",
    );

    game.active_player = PlayerId::Two;
    assert!(
        casts_of(&game, buried).is_empty(),
        "permission to cast it from there is not permission to cast it now",
    );
}
