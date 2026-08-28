//! Nethergoyf: one mana for whatever the graveyard has made of it, and the
//! graveyard pays a second time to buy it back.

use super::*;

/// Player One with a Nethergoyf in hand, `graveyard` behind it, and four
/// mana up for either way of casting it.
fn staged(graveyard: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[0].exile.clear();
    for definition in graveyard {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].graveyard.push(card);
    }
    let goyf = game
        .build_zone(PlayerId::One, &[cards::NETHERGOYF])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = goyf.id;
    game.players[0].hand.push(goyf);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, id)
}

/// Moves the Goyf from hand to graveyard, standing in for it having died,
/// and hands back its graveyard identity.
fn bury(game: &mut Game, goyf: GameObjectId) -> GameObjectId {
    let card = remove_card(&mut game.players[0].hand, goyf).expect("it is in hand");
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

fn settle(game: &mut Game) {
    for _ in 0..24 {
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

fn body(game: &Game) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::NETHERGOYF)
}

/// A card type is a point of power, and the toughness is that plus one.
#[test]
fn it_is_as_big_as_the_card_types_in_your_graveyard() {
    let (mut game, goyf) = staged(&[
        cards::MOUNTAIN,
        cards::LIGHTNING_BOLT,
        cards::SERRA_ANGEL,
        cards::FOREST,
    ]);
    let cast = casts_of(&game, goyf)
        .into_iter()
        .next()
        .expect("one black casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    let goyf = body(&game).expect("it resolved");
    assert_eq!(game.power(goyf), Some(3), "land, instant, creature");
    assert_eq!(game.toughness(goyf), Some(4), "and one more toughness");
}

/// An empty graveyard leaves the printed body, which is the plus one on its
/// own.
#[test]
fn an_empty_graveyard_leaves_a_zero_one() {
    let (mut game, goyf) = staged(&[]);
    let cast = casts_of(&game, goyf)
        .into_iter()
        .next()
        .expect("one black casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    let goyf = body(&game).expect("it resolved");
    assert_eq!(game.power(goyf), Some(0));
    assert_eq!(game.toughness(goyf), Some(1));
}

/// Escape wants four card types among the cards it exiles, and three is not
/// four.
#[test]
fn three_card_types_are_not_enough_to_escape() {
    let (mut game, goyf) = staged(&[cards::MOUNTAIN, cards::LIGHTNING_BOLT, cards::SERRA_ANGEL]);
    let buried = bury(&mut game, goyf);

    assert!(
        casts_of(&game, buried).is_empty(),
        "three types in the graveyard cannot pay for four",
    );
}

/// A fourth type opens it, and escaping exiles what it counted.
#[test]
fn four_card_types_escape_and_are_exiled() {
    let (mut game, goyf) = staged(&[
        cards::MOUNTAIN,
        cards::LIGHTNING_BOLT,
        cards::SERRA_ANGEL,
        cards::SOL_RING,
    ]);
    let buried = bury(&mut game, goyf);

    let cast = casts_of(&game, buried)
        .into_iter()
        .next()
        .expect("land, instant, creature, artifact is four");
    game.apply(PlayerId::One, cast).expect("it escapes");
    settle(&mut game);

    assert!(body(&game).is_some(), "an escaped Goyf stays");
    assert_eq!(
        game.players[0].exile.len(),
        4,
        "and the four it counted are exiled",
    );
    assert!(
        game.players[0].graveyard.is_empty(),
        "so the graveyard it was reading is empty now",
    );
}

/// The cost counts types rather than cards: a Dryad Arbor is a land and a
/// creature at once, so three cards can pay a four-type cost.
#[test]
fn one_card_can_pay_for_two_of_the_types() {
    let (mut game, goyf) = staged(&[cards::DRYAD_ARBOR, cards::LIGHTNING_BOLT, cards::SOL_RING]);
    let buried = bury(&mut game, goyf);

    let cast = casts_of(&game, buried)
        .into_iter()
        .next()
        .expect("the Arbor is worth two of the four");
    game.apply(PlayerId::One, cast).expect("it escapes");
    settle(&mut game);

    assert!(body(&game).is_some(), "it escaped on three cards");
    assert_eq!(game.players[0].exile.len(), 3);
}

/// "The ability that defines its power and toughness works in all zones."
/// Corpse Lunge reads the power of the card it exiled, and what it reads is
/// the Goyf counting a graveyard it is no longer in: five types with it
/// there, four once it has left.
#[test]
fn its_power_is_read_wherever_the_card_is() {
    let (mut game, goyf) = staged(&[
        cards::LIGHTNING_BOLT,
        cards::PONDER,
        cards::SOL_RING,
        cards::OATH_OF_DRUIDS,
    ]);
    bury(&mut game, goyf);
    let wall = game
        .put_onto_battlefield(PlayerId::Two, cards::LIVING_WALL)
        .expect("cataloged");
    drain_pending(&mut game);

    let lunge = game
        .build_zone(PlayerId::One, &[cards::CORPSE_LUNGE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let lunge_id = lunge.id;
    game.players[0].hand.push(lunge);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 3);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == lunge_id))
        .expect("the Goyf is the creature card it exiles");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    let damage = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == wall)
        .expect("a 0/6 survives it")
        .damage;
    assert_eq!(
        damage, 4,
        "four types left in the graveyard once the Goyf itself was exiled out of it",
    );
}
