//! Tinker: a Mox for whatever the deck's best artifact is.

use super::search_and_reveal::stack_library;
use super::*;

/// Tinker in hand with three mana, `mine` on the battlefield and `library`
/// stacked top-first.
fn staged(mine: &[CardDefinitionId], library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    stack_library(
        &mut game,
        &library
            .iter()
            .enumerate()
            .map(|(index, definition)| {
                (
                    64_000 + u32::try_from(index).expect("a handful of cards"),
                    *definition,
                )
            })
            .collect::<Vec<_>>(),
    );
    for definition in mine {
        game.put_onto_battlefield(PlayerId::One, *definition)
            .expect("cataloged");
    }
    let tinker = game
        .build_zone(PlayerId::One, &[cards::TINKER])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let tinker_id = tinker.id;
    game.players[0].hand.push(tinker);
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 3);
    (game, tinker_id)
}

/// Every artifact the spell would take as its sacrifice.
fn payments(game: &Game, tinker: GameObjectId) -> Vec<GameObjectId> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell {
                card, sacrifices, ..
            } if card == tinker => sacrifices.first().copied(),
            _ => None,
        })
        .collect()
}

fn on_battlefield(game: &Game, definition: CardDefinitionId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.definition == definition)
}

/// The whole card: give up a Mox, go and get the Lotus.
#[test]
fn a_mox_buys_the_best_artifact_in_the_deck() {
    let (mut game, tinker) = staged(&[cards::MOX_SAPPHIRE], &[cards::BLACK_LOTUS]);
    let mox = game.battlefield[0].card.id;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, sacrifices, .. }
                if *card == tinker && sacrifices.contains(&mox))
        })
        .expect("the Mox pays for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    assert!(
        !on_battlefield(&game, cards::MOX_SAPPHIRE),
        "the sacrifice is a cost, paid on casting",
    );
    drain_pending(&mut game);

    assert!(
        on_battlefield(&game, cards::BLACK_LOTUS),
        "and the Lotus arrived from the library",
    );
    assert!(game.players[0].library.is_empty(), "which is now empty");
}

/// An artifact, not any permanent: a Bear will not pay for it.
#[test]
fn only_an_artifact_pays_for_it() {
    let (game, tinker) = staged(&[cards::GRIZZLY_BEARS], &[cards::BLACK_LOTUS]);

    assert!(
        payments(&game, tinker).is_empty(),
        "there is nothing it can sacrifice",
    );
}

/// With nothing at all on the battlefield the additional cost cannot be
/// paid, so the spell is not castable however much mana is up.
#[test]
fn an_empty_board_cannot_pay_it() {
    let (game, tinker) = staged(&[], &[cards::BLACK_LOTUS]);

    assert!(payments(&game, tinker).is_empty());
}

/// The search finds artifacts and nothing else, and it is a search rather
/// than a reveal: what it takes goes straight onto the battlefield.
#[test]
fn the_search_offers_only_artifacts() {
    let (mut game, tinker) = staged(
        &[cards::MOX_SAPPHIRE],
        &[cards::LIGHTNING_BOLT, cards::SOL_RING, cards::FOREST],
    );
    let mox = game.battlefield[0].card.id;
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, sacrifices, .. }
                if *card == tinker && sacrifices.contains(&mox))
        })
        .expect("the Mox pays for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);

    let decision = game.observe(PlayerId::One).decision.expect("a search");
    let offered = decision
        .options
        .iter()
        .filter_map(|option| {
            option
                .card
                .and_then(|(_, characteristics)| characteristics.card_definition())
        })
        .collect::<Vec<_>>();

    assert_eq!(offered, vec![cards::SOL_RING], "the one artifact in there");
}

/// A library with no artifact in it: the search finds nothing and the spell
/// has still eaten the artifact that paid for it.
#[test]
fn an_empty_search_still_costs_the_sacrifice() {
    let (mut game, tinker) = staged(&[cards::MOX_SAPPHIRE], &[cards::FOREST]);
    let mox = game.battlefield[0].card.id;
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, sacrifices, .. }
                if *card == tinker && sacrifices.contains(&mox))
        })
        .expect("the Mox pays for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert!(
        !on_battlefield(&game, cards::MOX_SAPPHIRE),
        "the Mox is gone either way",
    );
    assert!(game.battlefield.is_empty(), "and nothing came back for it");
}
