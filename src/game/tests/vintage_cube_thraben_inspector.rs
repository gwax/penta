//! Thraben Inspector: one mana for a body and a card, with the card
//! deferred until two mana is spare.

use super::*;

/// Player One holding the Inspector, with a library to draw from.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for index in 0..4 {
        game.players[0]
            .library
            .push(card(105_000 + index, cards::GRIZZLY_BEARS, PlayerId::One));
    }
    let inspector = game
        .build_zone(PlayerId::One, &[cards::THRABEN_INSPECTOR])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = inspector.id;
    game.players[0].hand.push(inspector);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held)
}

fn clues(game: &Game) -> Vec<GameObjectId> {
    game.battlefield
        .iter()
        .filter(|permanent| game.effective_subtypes(permanent).contains(&"Clue"))
        .map(|permanent| permanent.card.id)
        .collect()
}

/// Casts the Inspector for its one white mana.
fn cast(game: &mut Game, held: GameObjectId) {
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == held))
        .expect("one white mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(game);
}

/// One mana buys a 1/2 and a Clue.
#[test]
fn it_arrives_with_a_clue() {
    let (mut game, held) = staged();

    cast(&mut game, held);

    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::THRABEN_INSPECTOR)
        .expect("the Soldier is there");
    assert_eq!(game.power(body), Some(1));
    assert_eq!(game.toughness(body), Some(2));
    assert_eq!(clues(&game).len(), 1, "and investigating made one Clue");
    assert_eq!(
        game.players[0].hand.len(),
        0,
        "the card is not drawn yet -- the Clue is what holds it",
    );
}

/// The Clue is an artifact, not a creature: it is a permanent that sits
/// there until it is cashed in.
#[test]
fn the_clue_is_an_artifact() {
    let (mut game, held) = staged();
    cast(&mut game, held);

    let clue = game
        .battlefield
        .iter()
        .find(|permanent| game.effective_subtypes(permanent).contains(&"Clue"))
        .expect("the Clue is there");
    let types = game.permanent_types(clue).expect("it has card types");
    assert!(types.contains(CardType::Artifact));
    assert!(!types.contains(CardType::Creature));
    assert_eq!(game.power(clue), None, "an artifact has no power");
}

/// Two mana and the Clue itself buys the card it was holding.
#[test]
fn cashing_the_clue_in_draws_a_card() {
    let (mut game, held) = staged();
    cast(&mut game, held);
    let clue = clues(&game)[0];
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);

    let cash_in = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == clue))
        .expect("two mana and the Clue itself buys a card");
    game.apply(PlayerId::One, cash_in).expect("it activates");
    drain_pending(&mut game);

    assert_eq!(game.players[0].hand.len(), 1, "the card is drawn");
    assert_eq!(game.players[0].library.len(), 3);
    assert!(clues(&game).is_empty(), "and the Clue sacrificed itself");
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::THRABEN_INSPECTOR),
        "the body stays behind either way",
    );
}

/// Without the two mana the Clue cannot be cashed in.
#[test]
fn the_clue_costs_two_to_cash_in() {
    let (mut game, held) = staged();
    cast(&mut game, held);
    let clue = clues(&game)[0];
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == clue)
        ),
        "one mana is not two",
    );
}
