//! Consider: one mana to see two cards deep and choose which of them the
//! deck is better off having in the graveyard.

use super::*;

/// Consider in hand with one blue up and `library` stacked, top card first.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[0].library.clear();
    for (index, definition) in library.iter().rev().enumerate() {
        game.players[0].library.push(card(
            99_000 + u32::try_from(index).expect("a small library"),
            *definition,
            PlayerId::One,
        ));
    }
    let consider = game
        .build_zone(PlayerId::One, &[cards::CONSIDER])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = consider.id;
    game.players[0].hand.push(consider);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.turns_started = [2, 1];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, id)
}

/// Casts it and answers the surveil, binning the card when `bin` is true.
fn cast_and_surveil(game: &mut Game, consider: GameObjectId, bin: bool) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == consider))
        .expect("one blue casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = if bin {
                decision
                    .options
                    .first()
                    .map(|option| vec![option.id])
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("surveil accepts either answer");
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

fn in_hand(game: &Game, definition: CardDefinitionId) -> bool {
    game.players[0]
        .hand
        .iter()
        .any(|card| card.definition == definition)
}

/// Binning the top card draws the one underneath it.
#[test]
fn binning_the_top_card_draws_the_next_one() {
    let (mut game, consider) = staged(&[cards::MOUNTAIN, cards::LIGHTNING_BOLT]);

    cast_and_surveil(&mut game, consider, true);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::MOUNTAIN),
        "the Mountain went to the graveyard",
    );
    assert!(in_hand(&game, cards::LIGHTNING_BOLT), "and the Bolt drawn");
    assert!(game.players[0].library.is_empty(), "both left the library");
}

/// Keeping it draws the card you just looked at.
#[test]
fn keeping_the_top_card_draws_it() {
    let (mut game, consider) = staged(&[cards::LIGHTNING_BOLT, cards::MOUNTAIN]);

    cast_and_surveil(&mut game, consider, false);

    assert!(in_hand(&game, cards::LIGHTNING_BOLT), "the Bolt is drawn");
    assert!(
        !game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "nothing was binned",
    );
    assert_eq!(
        game.players[0].library.len(),
        1,
        "the Mountain is still there"
    );
}

/// The spell itself goes to the graveyard after it resolves.
#[test]
fn the_spell_goes_to_the_graveyard() {
    let (mut game, consider) = staged(&[cards::MOUNTAIN, cards::LIGHTNING_BOLT]);

    cast_and_surveil(&mut game, consider, true);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::CONSIDER),
        "it is in the graveyard",
    );
    assert!(game.stack.is_empty(), "and off the stack");
}

/// An empty library has nothing to look at, and the draw is what kills you
/// rather than the look.
#[test]
fn an_empty_library_still_resolves() {
    let (mut game, consider) = staged(&[]);

    cast_and_surveil(&mut game, consider, false);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::CONSIDER),
        "the spell resolved",
    );
}
