//! Mox Sapphire: the blue member of the five, and the one place worth
//! testing them from is the hand -- a spell that costs nothing, is not a
//! land, and taps the moment it lands.

use super::*;

/// Player One holding `hand`, with an empty pool and the land drop unused.
fn staged(hand: &[CardDefinitionId]) -> (Game, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let mut held = Vec::new();
    for (index, definition) in hand.iter().enumerate() {
        let card = card(
            97_500 + u32::try_from(index).expect("a few cards"),
            *definition,
            PlayerId::One,
        );
        held.push(card.id);
        game.players[0].hand.push(card);
    }
    game.players[0].lands_played_this_turn = 0;
    game.turns_started = [1, 0];
    game.turn = 1;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.empty_mana_pools();
    (game, held)
}

fn cast(game: &mut Game, spell: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
        .expect("a free spell is castable with nothing available");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(game);
}

fn tap_for(game: &mut Game, definition: CardDefinitionId, color: ManaColor) {
    let mox = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == definition)
        .expect("it is on the battlefield")
        .card
        .id;
    let action = Action::ActivateManaAbility {
        source: mox,
        ability: mana_ability_for(game, mox, color),
        color,
        counters_removed: None,
        cost_object: None,
        combination: None,
    };
    game.apply(PlayerId::One, action).expect("it taps");
}

/// Nothing is what it costs: it comes down off an empty pool and makes blue
/// the same turn, because an artifact has no summoning sickness.
#[test]
fn it_costs_nothing_and_pays_at_once() {
    let (mut game, held) = staged(&[cards::MOX_SAPPHIRE]);
    assert_eq!(game.players[0].mana_pool.total(), 0, "nothing to spend");

    cast(&mut game, held[0]);
    tap_for(&mut game, cards::MOX_SAPPHIRE, ManaColor::Blue);

    assert_eq!(game.players[0].mana_pool.blue, 1);
    assert_eq!(game.players[0].mana_pool.total(), 1);
}

/// A Mox is a spell rather than a land, so a hand of them all comes down on
/// the same turn and the land drop is still waiting afterwards.
#[test]
fn a_hand_of_moxen_all_comes_down_at_once() {
    let (mut game, held) = staged(&[cards::MOX_SAPPHIRE, cards::MOX_JET, cards::ISLAND]);

    cast(&mut game, held[0]);
    cast(&mut game, held[1]);

    assert_eq!(
        game.battlefield.len(),
        2,
        "both of them, on the first turn, for nothing",
    );
    assert_eq!(
        game.players[0].lands_played_this_turn, 0,
        "and neither one was a land drop",
    );
    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::PlayLand { card, .. } if *card == held[2])),
        "so the Island in hand is still playable",
    );

    tap_for(&mut game, cards::MOX_SAPPHIRE, ManaColor::Blue);
    tap_for(&mut game, cards::MOX_JET, ManaColor::Black);
    assert_eq!(game.players[0].mana_pool.blue, 1);
    assert_eq!(game.players[0].mana_pool.black, 1);
}
