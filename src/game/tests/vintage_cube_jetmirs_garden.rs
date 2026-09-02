//! Jetmir's Garden: three basic land types on one card, and what the cards
//! that count them make of it.
//!
//! The cycle's own behaviour -- entering tapped, its three colours, cycling
//! for three generic, and a fetchland finding it -- is pinned in
//! `vintage_cube_lands`. What is here is the reason a deck plays a triome
//! over a dual: one land answers three questions at once.

use super::*;

/// A land on the battlefield untapped, whatever its entry clause wanted.
fn land_ready(game: &mut Game, definition: CardDefinitionId) -> GameObjectId {
    let land = game
        .put_onto_battlefield(PlayerId::One, definition)
        .expect("cataloged");
    drain_pending(game);
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == land)
    {
        permanent.tapped = false;
    }
    land
}

/// Domain counts basic land types rather than lands: one Garden is three of
/// them, so a Leyline Binding that would cost six costs three.
#[test]
fn one_garden_is_three_types_of_domain() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    // Left tapped as it entered: domain counts its types either way, and an
    // untapped land would add its own mana to what is castable.
    game.put_onto_battlefield(PlayerId::One, cards::JETMIRS_GARDEN)
        .expect("cataloged");
    drain_pending(&mut game);
    let binding = game
        .build_zone(PlayerId::One, &[cards::LEYLINE_BINDING])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let binding_id = binding.id;
    game.players[0].hand.push(binding);
    game.put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let castable = |game: &Game| {
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == binding_id))
    };

    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert!(!castable(&game), "two mana is not three");

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    assert!(
        castable(&game),
        "three basic land types off one card take three off the six",
    );
}

/// The same three types read one at a time: the Garden is a Forest, so
/// Nissa doubles it, and a Flinthoof Boar reads its Mountain.
#[test]
fn its_types_are_read_one_at_a_time_by_whatever_asks() {
    let mut game = ready_game();
    game.battlefield.clear();
    let mut nissa = creature(88_000, cards::NISSA_WHO_SHAKES_THE_WORLD, PlayerId::One);
    nissa.set_counters(CounterKind::Loyalty, 5);
    game.battlefield.push(nissa);
    let boar = game
        .put_onto_battlefield(PlayerId::One, cards::FLINTHOOF_BOAR)
        .expect("cataloged");
    drain_pending(&mut game);
    let size = |game: &Game| {
        let permanent = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == boar)
            .expect("the Boar is out");
        (game.power(permanent), game.toughness(permanent))
    };
    assert_eq!(size(&game), (Some(2), Some(2)), "no Mountain yet");

    let garden = land_ready(&mut game, cards::JETMIRS_GARDEN);
    game.priority = PlayerId::One;

    assert_eq!(
        size(&game),
        (Some(3), Some(3)),
        "the Garden's Mountain turns the Boar on",
    );

    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: garden,
            ability: mana_ability_for(&game, garden, ManaColor::Green),
            color: ManaColor::Green,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("it taps for green");
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].mana_pool.green, 2,
        "and its Forest is a Forest to Nissa",
    );
}
