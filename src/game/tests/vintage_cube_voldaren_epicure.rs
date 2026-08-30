//! Voldaren Epicure: one mana for a body, a point of damage, and a card the
//! Blood turns a dead draw into later.

use super::*;

/// Player One holding the Epicure with a mana up and a stocked library.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..4 {
        game.players[0]
            .library
            .push(card(112_000 + index, cards::ISLAND, PlayerId::One));
    }
    let card = game
        .build_zone(PlayerId::One, &[cards::VOLDAREN_EPICURE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[0].hand.push(card);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.players[0].life = 20;
    game.players[1].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held)
}

fn cast(game: &mut Game, held: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held))
        .expect("one red mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(game);
}

fn bloods(game: &Game) -> Vec<GameObjectId> {
    game.battlefield
        .iter()
        .filter(|permanent| game.effective_subtypes(permanent).contains(&"Blood"))
        .map(|permanent| permanent.card.id)
        .collect()
}

/// It arrives with a point of damage and a Blood token.
#[test]
fn it_pings_and_leaves_blood() {
    let (mut game, held) = staged();

    cast(&mut game, held);

    assert_eq!(game.players[1].life, 19, "one damage to the opponent");
    assert_eq!(game.players[0].life, 20, "and none to you");
    assert_eq!(bloods(&game).len(), 1, "and one Blood token");
    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::VOLDAREN_EPICURE)
        .expect("the Vampire is there");
    assert_eq!(game.power(body), Some(1));
    assert_eq!(game.toughness(body), Some(1));
}

/// The Blood cashes a card in hand for a fresh one, and goes with it.
#[test]
fn the_blood_turns_a_card_over() {
    let (mut game, held) = staged();
    cast(&mut game, held);
    let blood = bloods(&game)[0];
    game.players[0]
        .hand
        .push(card(112_500, cards::MOUNTAIN, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }

    let activate = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == blood))
        .expect("a mana, the tap, a card, and the token itself");
    game.apply(PlayerId::One, activate).expect("it activates");
    drain_pending(&mut game);

    assert!(bloods(&game).is_empty(), "the token sacrificed itself");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::MOUNTAIN),
        "the discarded card is in the graveyard",
    );
    assert_eq!(
        game.players[0].hand.len(),
        1,
        "and a fresh card replaced it"
    );
    assert_eq!(game.players[0].library.len(), 3);
}

/// The Blood is an artifact and not a creature.
#[test]
fn the_blood_is_an_artifact() {
    let (mut game, held) = staged();
    cast(&mut game, held);
    let blood = bloods(&game)[0];

    let token = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == blood)
        .expect("the token is there");
    let types = game.permanent_types(token).expect("it has card types");
    assert!(types.contains(CardType::Artifact));
    assert!(!types.contains(CardType::Creature));
}

/// Summoning sickness belongs to creatures: the Blood is an artifact, so the
/// turn it arrives is as good as any for cashing it in.
#[test]
fn the_blood_may_be_cashed_the_turn_it_arrives() {
    let (mut game, held) = staged();
    cast(&mut game, held);
    let blood = bloods(&game)[0];
    game.players[0]
        .hand
        .push(card(112_600, cards::MOUNTAIN, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    assert!(
        game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == blood)
        ),
        "a token that just arrived taps like any other artifact",
    );
}

/// "Discard a card" is part of the cost, so an empty hand is a Blood that
/// cannot be spent however much mana is available.
#[test]
fn an_empty_hand_cannot_pay_for_the_draw() {
    let (mut game, held) = staged();
    cast(&mut game, held);
    let blood = bloods(&game)[0];
    game.players[0].hand.clear();
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == blood)
        ),
        "there is nothing to discard for it",
    );
}
