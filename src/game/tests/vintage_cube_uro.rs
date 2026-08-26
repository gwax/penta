//! Uro, Titan of Nature's Wrath: a ramp spell that sacrifices itself, and
//! the same card again later as a 6/6 that does it every attack.

use super::*;

/// Player One with an Uro in hand, `fodder` other cards in the graveyard,
/// `hand` further cards in hand, and mana for either way of casting it.
fn staged(fodder: usize, hand: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[0].library.clear();
    for index in 0..6 {
        game.players[0]
            .library
            .push(card(69_000 + index, cards::ISLAND, PlayerId::One));
    }
    for _ in 0..fodder {
        let card = game
            .build_zone(PlayerId::One, &[cards::MOUNTAIN])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].graveyard.push(card);
    }
    let uro = game
        .build_zone(PlayerId::One, &[cards::URO_TITAN_OF_NATURE_S_WRATH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let uro_id = uro.id;
    game.players[0].hand.push(uro);
    for definition in hand {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].hand.push(card);
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].life = 20;
    for color in [ManaColor::Green, ManaColor::Blue] {
        game.add_unrestricted_mana(PlayerId::One, color, 3);
    }
    (game, uro_id)
}

/// Moves the Uro from hand to the graveyard, standing in for it having died,
/// and hands back its graveyard identity.
fn bury(game: &mut Game, uro: GameObjectId) -> GameObjectId {
    let card = remove_card(&mut game.players[0].hand, uro).expect("it is in hand");
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

/// Answers everything, taking `land` from the land choice when it is named.
fn settle(game: &mut Game, land: Option<CardDefinitionId>) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = if decision.minimum > 1 {
                decision.options.iter().map(|option| option.id).collect()
            } else {
                land.into_iter()
                    .flat_map(|land| {
                        decision.options.iter().filter(move |option| {
                            option.card.is_some_and(|(_, characteristics)| {
                                characteristics.card_definition() == Some(land)
                            })
                        })
                    })
                    .map(|option| option.id)
                    .take(1)
                    .collect::<Vec<_>>()
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
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

fn on_battlefield(game: &Game) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.definition == cards::URO_TITAN_OF_NATURE_S_WRATH)
}

/// Cast for its printed cost it grows you and then sacrifices itself.
#[test]
fn cast_from_hand_it_ramps_and_dies() {
    let (mut game, uro) = staged(0, &[cards::FOREST]);
    let cast = casts_of(&game, uro)
        .into_iter()
        .next()
        .expect("three mana buys it from hand");
    game.apply(PlayerId::One, cast).expect("it is castable");
    settle(&mut game, Some(cards::FOREST));

    assert_eq!(game.players[0].life, 23, "three life");
    assert_eq!(game.players[0].hand.len(), 1, "a card drawn, a land played");
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::FOREST),
        "the land came down",
    );
    assert!(!on_battlefield(&game), "and Uro sacrificed itself");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::URO_TITAN_OF_NATURE_S_WRATH),
        "leaving it in the graveyard to escape from later",
    );
}

/// The land is a "may": declining leaves it in hand, and the free land drop
/// is not spent.
#[test]
fn the_land_drop_may_be_declined() {
    let (mut game, uro) = staged(0, &[cards::FOREST]);
    let cast = casts_of(&game, uro)
        .into_iter()
        .next()
        .expect("three mana buys it from hand");
    game.apply(PlayerId::One, cast).expect("it is castable");
    settle(&mut game, None);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::FOREST),
        "the Forest stayed in hand",
    );
    assert_eq!(game.players[0].life, 23, "the rest of the clause happened");
}

/// Escape needs five other cards in the graveyard: with four there is no way
/// to cast it from there.
#[test]
fn escape_needs_five_other_cards() {
    let (mut game, uro) = staged(4, &[]);
    let buried = bury(&mut game, uro);

    assert!(casts_of(&game, buried).is_empty(), "four is not five");

    let extra = game
        .build_zone(PlayerId::One, &[cards::MOUNTAIN])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    game.players[0].graveyard.push(extra);

    assert!(!casts_of(&game, buried).is_empty(), "and five is");
}

/// Escaped, it stays: the sacrifice clause reads how it was cast.
#[test]
fn escaped_it_stays_on_the_battlefield() {
    let (mut game, uro) = staged(5, &[]);
    let buried = bury(&mut game, uro);
    let cast = casts_of(&game, buried)
        .into_iter()
        .next()
        .expect("escape is on offer");
    game.apply(PlayerId::One, cast).expect("it is castable");
    settle(&mut game, None);

    assert!(on_battlefield(&game), "an escaped Uro sacrifices nothing");
    assert_eq!(game.players[0].life, 23, "and it still grows you");
    assert_eq!(
        game.players[0].graveyard.len(),
        0,
        "five cards paid for it, and Uro left the graveyard",
    );
}

/// Attacking fires the same clause again, which is what the 6/6 is for.
#[test]
fn attacking_grows_you_again() {
    let (mut game, uro) = staged(5, &[]);
    let buried = bury(&mut game, uro);
    let cast = casts_of(&game, buried)
        .into_iter()
        .next()
        .expect("escape is on offer");
    game.apply(PlayerId::One, cast).expect("it is castable");
    settle(&mut game, None);
    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::URO_TITAN_OF_NATURE_S_WRATH)
        .expect("it stayed")
        .card
        .id;
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    let life = game.players[0].life;

    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.apply(
        PlayerId::One,
        Action::DeclareAttacker {
            attacker: body,
            defender: AttackDefender::Player(PlayerId::Two),
        },
    )
    .expect("a 6/6 with no summoning sickness may attack");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("the declaration is complete");
    settle(&mut game, None);

    assert_eq!(game.players[0].life, life + 3, "three more life");
}
