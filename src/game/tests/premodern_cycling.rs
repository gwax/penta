//! Cycling: an activated ability that lives only in hand.
//!
//! The discard is a cost, so the card is already in the graveyard when the
//! draw resolves. These tests check that the ability is offered from hand and
//! nowhere else, that it costs what it prints, and that the card's ordinary
//! face is untouched by it.

use super::*;

fn ready() -> Game {
    let mut game = ready_game();
    game.turn = 5;
    game.turns_started[PlayerId::One.index()] = 5;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].library.clear();
    for index in 0..4 {
        game.players[PlayerId::One.index()].library.push(card(
            30_000 + index,
            cards::GRIZZLY_BEARS,
            PlayerId::One,
        ));
    }
    game
}

/// The cycling activation for `source`, if the player is offered one.
fn cycle_action(game: &Game, source: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One).into_iter().find(
        |action| matches!(action, Action::ActivateAbility { source: id, .. } if *id == source),
    )
}

fn resolve(game: &mut Game) {
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

/// Cycling a land: one white pays for it, the card ends up in the graveyard
/// as a cost rather than an effect, and a card is drawn.
#[test]
fn cycling_discards_the_card_and_draws_one() {
    let mut game = ready();
    let steppe = card(20_000, cards::SECLUDED_STEPPE, PlayerId::One);
    let steppe_id = steppe.id;
    game.players[PlayerId::One.index()].hand.push(steppe);
    game.players[PlayerId::One.index()].mana_pool.white = 1;
    let before = game.players[PlayerId::One.index()].library.len();

    let action = cycle_action(&game, steppe_id).expect("cycling is offered from hand");
    game.apply(PlayerId::One, action).expect("it is activated");

    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SECLUDED_STEPPE),
        "the discard is a cost, so it happens on activation",
    );
    resolve(&mut game);
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        before - 1,
        "and the draw is what went on the stack",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "the drawn card is in hand",
    );
}

/// Without the mana there is no activation at all -- the discard alone does
/// not pay for it.
#[test]
fn cycling_is_not_offered_without_the_mana() {
    let mut game = ready();
    let steppe = card(20_000, cards::SECLUDED_STEPPE, PlayerId::One);
    let steppe_id = steppe.id;
    game.players[PlayerId::One.index()].hand.push(steppe);
    // A single blue: enough mana, entirely the wrong color.
    game.players[PlayerId::One.index()].mana_pool.blue = 1;

    assert!(
        cycle_action(&game, steppe_id).is_none(),
        "{{W}} is not payable with blue",
    );
}

/// The ability names the hand as its only source zone, so the land on the
/// battlefield does not offer it.
#[test]
fn a_cycling_permanent_does_not_cycle_from_the_battlefield() {
    let mut game = ready();
    let steppe = creature(10_000, cards::SECLUDED_STEPPE, PlayerId::One);
    let steppe_id = steppe.card.id;
    game.battlefield.push(steppe);
    game.players[PlayerId::One.index()].mana_pool.white = 4;

    let cycles = game.legal_actions(PlayerId::One).into_iter().any(
        |action| matches!(action, Action::ActivateAbility { source, .. } if source == steppe_id),
    );
    assert!(
        !cycles,
        "a land on the battlefield cannot discard itself to draw",
    );
}

/// Cycling a spell costs what the spell's cycling clause prints, not what the
/// spell costs: {3} for a six-mana sweeper.
#[test]
fn cycling_costs_the_printed_cycling_cost() {
    let mut game = ready();
    let vengeance = card(20_000, cards::AKROMAS_VENGEANCE, PlayerId::One);
    let vengeance_id = vengeance.id;
    game.players[PlayerId::One.index()].hand.push(vengeance);
    game.players[PlayerId::One.index()].mana_pool.colorless = 2;

    assert!(
        cycle_action(&game, vengeance_id).is_none(),
        "two generic does not pay {{3}}",
    );

    game.players[PlayerId::One.index()].mana_pool.colorless = 3;
    let action = cycle_action(&game, vengeance_id).expect("three generic does");
    game.apply(PlayerId::One, action).expect("it is activated");
    resolve(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.colorless,
        0,
        "all three were spent",
    );
    assert!(
        game.battlefield.is_empty(),
        "cycling is not casting -- nothing was destroyed",
    );
}

/// Typecycling buys a search rather than a draw, and the search is bounded
/// by the type it names: a library of bears offers nothing.
#[test]
fn plainscycling_fetches_a_plains_and_only_a_plains() {
    let mut game = ready();
    game.players[PlayerId::One.index()]
        .library
        .push(card(30_100, cards::PLAINS, PlayerId::One));
    let dragon = card(20_000, cards::ETERNAL_DRAGON, PlayerId::One);
    let dragon_id = dragon.id;
    game.players[PlayerId::One.index()].hand.push(dragon);
    game.players[PlayerId::One.index()].mana_pool.colorless = 2;

    let action = cycle_action(&game, dragon_id).expect("plainscycling is offered from hand");
    game.apply(PlayerId::One, action).expect("it is activated");
    resolve(&mut game);

    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the search asks which card to take");
    // The library holds four bears and one Plains, and only the Plains is on
    // offer.
    assert_eq!(
        decision.options.len(),
        1,
        "a bear is not a Plains card, however much you would like one",
    );
    game.apply(
        decision.player,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![decision.options[0].id],
        },
    )
    .expect("the search accepts what it offered");
    resolve(&mut game);

    assert!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .any(|card| card.definition == cards::PLAINS),
        "the Plains went to hand, not the battlefield",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::PLAINS),
        "typecycling is not a fetch land",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::ETERNAL_DRAGON),
        "and the Dragon paid for it by being discarded",
    );
}

/// The Dragon buys itself back from the graveyard, but only in your own
/// upkeep -- which is what keeps it from being a main-phase threat every
/// turn.
#[test]
fn the_dragon_returns_itself_only_during_your_upkeep() {
    let mut game = ready();
    let dragon = card(20_000, cards::ETERNAL_DRAGON, PlayerId::One);
    let dragon_id = dragon.id;
    game.players[PlayerId::One.index()].graveyard.push(dragon);
    game.players[PlayerId::One.index()].mana_pool.white = 2;
    game.players[PlayerId::One.index()].mana_pool.colorless = 3;

    assert!(
        cycle_action(&game, dragon_id).is_none(),
        "the main phase is not an upkeep",
    );

    game.step = Step::Upkeep;
    let action = cycle_action(&game, dragon_id).expect("your own upkeep is the window");
    game.apply(PlayerId::One, action).expect("it is activated");
    resolve(&mut game);

    assert!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .any(|card| card.definition == cards::ETERNAL_DRAGON),
        "the Dragon came back to hand",
    );
    assert!(
        game.players[PlayerId::One.index()].graveyard.is_empty(),
        "and left the graveyard behind",
    );
}

/// And casting it the ordinary way still sweeps all three types while
/// leaving lands alone.
#[test]
fn akromas_vengeance_destroys_artifacts_creatures_and_enchantments() {
    let mut game = ready();
    game.battlefield
        .push(creature(10_000, cards::GRIZZLY_BEARS, PlayerId::Two));
    game.battlefield
        .push(creature(10_001, cards::BLACK_VISE, PlayerId::Two));
    game.battlefield
        .push(creature(10_002, cards::SECLUDED_STEPPE, PlayerId::One));

    let vengeance = card(20_000, cards::AKROMAS_VENGEANCE, PlayerId::One);
    let vengeance_id = vengeance.id;
    game.players[PlayerId::One.index()].hand.push(vengeance);
    game.players[PlayerId::One.index()].mana_pool.white = 2;
    game.players[PlayerId::One.index()].mana_pool.colorless = 4;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == vengeance_id))
        .expect("six mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    resolve(&mut game);

    let remaining: Vec<_> = game
        .battlefield
        .iter()
        .map(|permanent| permanent.card.definition)
        .collect();
    assert_eq!(
        remaining,
        vec![cards::SECLUDED_STEPPE],
        "the land is the only thing the sweeper does not name",
    );
}
