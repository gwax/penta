//! Mishra's Workshop: three mana that only artifacts may spend.

use super::*;

/// The Workshop tapped for its three, with `hand` in Player One's hand.
fn staged(hand: &[CardDefinitionId]) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let workshop = game
        .put_onto_battlefield(PlayerId::One, cards::MISHRA_S_WORKSHOP)
        .expect("cataloged");
    drain_pending(&mut game);
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
    let ability = mana_ability_for(&game, workshop, ManaColor::Colorless);
    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: workshop,
            ability,
            color: ManaColor::Colorless,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("it taps for three");
    game
}

fn castable(game: &Game, definition: CardDefinitionId) -> bool {
    let Some(card) = game.players[0]
        .hand
        .iter()
        .find(|card| card.definition == definition)
    else {
        return false;
    };
    game.legal_actions(PlayerId::One)
        .iter()
        .any(|action| matches!(action, Action::CastSpell { card: id, .. } if *id == card.id))
}

/// What the restriction is for: three mana that pays for an artifact.
#[test]
fn its_mana_casts_an_artifact_spell() {
    let mut game = staged(&[cards::HOWLING_MINE]);
    assert_eq!(game.players[0].mana_pool.colorless, 3);

    assert!(
        castable(&game, cards::HOWLING_MINE),
        "two of the three pay it"
    );

    let card = game.players[0].hand[0].id;
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card: id, .. } if *id == card))
        .expect("it is castable");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::HOWLING_MINE),
        "and the artifact resolved",
    );
    assert_eq!(
        game.players[0].mana_pool.colorless, 1,
        "with one restricted mana left over",
    );
}

/// And what it is not for. A red mana pays the Ogre's pip, but the two
/// generic behind it can only come from the Workshop, and the Workshop's
/// mana does not cast Ogres.
#[test]
fn its_mana_will_not_pay_for_a_creature_spell() {
    let mut game = staged(&[cards::GRAY_OGRE]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);

    assert!(
        !castable(&game, cards::GRAY_OGRE),
        "three restricted mana and a red is not three mana and a red",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    assert!(
        castable(&game, cards::GRAY_OGRE),
        "two mana that may go anywhere is what it was missing",
    );
}

/// An artifact creature spell is an artifact spell, and three is exactly
/// what a Dragon Engine wants.
#[test]
fn its_mana_casts_an_artifact_creature_too() {
    let mut game = staged(&[cards::DRAGON_ENGINE]);
    assert!(
        castable(&game, cards::DRAGON_ENGINE),
        "an artifact creature is still an artifact spell",
    );

    let card = game.players[0].hand[0].id;
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card: id, .. } if *id == card))
        .expect("it is castable");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::DRAGON_ENGINE),
        "and it resolved on Workshop mana alone",
    );
    assert_eq!(
        game.players[0].mana_pool.colorless, 0,
        "all three of them went into it",
    );
}

/// "Spend this mana only to cast artifact spells" is about casting. An
/// artifact already on the battlefield activating an ability is not
/// casting anything, so the Workshop's mana cannot pay for it.
#[test]
fn its_mana_will_not_pay_an_artifacts_activated_ability() {
    let mut game = staged(&[]);
    let icy = creature(88_000, cards::ICY_MANIPULATOR, PlayerId::One);
    let icy_id = icy.card.id;
    game.battlefield.push(icy);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == icy_id)
    {
        permanent.entered_controller_turn = 0;
    }
    assert_eq!(game.players[0].mana_pool.colorless, 3);

    let offered = |game: &Game| {
        game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == icy_id),
        )
    };
    assert!(
        !offered(&game),
        "three mana that may only be cast with is no mana at all here",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    assert!(
        offered(&game),
        "one mana with no strings attached is what it wanted",
    );
}
