//! Sol Ring: one mana in, two mana out, on the turn it arrives.

use super::*;

/// Player One holding a Sol Ring with `mana` colourless available.
fn staged(mana: u16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let ring = card(110_000, cards::SOL_RING, PlayerId::One);
    let ring_id = ring.id;
    game.players[PlayerId::One.index()].hand.push(ring);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, mana);
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, ring_id)
}

/// Casts it and returns the permanent it became.
fn resolve_it(game: &mut Game, ring: GameObjectId) -> GameObjectId {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == ring))
        .expect("one mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(game);
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SOL_RING)
        .expect("it resolved")
        .card
        .id
}

/// The whole card: one mana buys it and it makes two, with no turn to wait
/// -- an artifact is not summoning sick.
#[test]
fn it_pays_for_itself_the_turn_it_arrives() {
    let (mut game, ring) = staged(1);
    let permanent = resolve_it(&mut game, ring);
    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.total(),
        0,
        "the one mana it cost is spent",
    );

    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: permanent,
            ability: mana_ability_for(&game, permanent, ManaColor::Colorless),
            color: ManaColor::Colorless,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("it taps for colourless on the turn it arrived");

    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.colorless,
        2,
        "two colourless from one tap",
    );
    assert!(
        game.battlefield
            .iter()
            .find(|found| found.card.id == permanent)
            .is_some_and(|found| found.tapped),
        "and the Ring is tapped for it",
    );
}

/// What it makes is colourless: it pays a generic cost and never a coloured
/// pip, however much of it there is.
#[test]
fn its_mana_is_colorless_and_pays_no_pips() {
    let (mut game, ring) = staged(1);
    let permanent = resolve_it(&mut game, ring);
    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: permanent,
            ability: mana_ability_for(&game, permanent, ManaColor::Colorless),
            color: ManaColor::Colorless,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("it taps for two");

    let counter = card(110_100, cards::COUNTERSPELL, PlayerId::One);
    let counter_id = counter.id;
    game.players[PlayerId::One.index()].hand.push(counter);
    let key = card(110_101, cards::MANIFOLD_KEY, PlayerId::One);
    let key_id = key.id;
    game.players[PlayerId::One.index()].hand.push(key);

    let castable = |game: &Game, id: GameObjectId| {
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == id))
    };
    assert!(
        castable(&game, key_id),
        "a one-mana artifact is what colourless pays for",
    );

    // A Counterspell needs something to answer, which is also what closes
    // the sorcery-speed window the artifact was using.
    game.stack.push(crate::game::tests::spell(
        110_102,
        cards::GRIZZLY_BEARS,
        PlayerId::Two,
        0,
    ));
    assert!(
        !castable(&game, counter_id),
        "and two colourless does not pay two blue pips",
    );
    assert!(
        !castable(&game, key_id),
        "with the artifact closed out by the stack, as sorcery speed asks",
    );
}
