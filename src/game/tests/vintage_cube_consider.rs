//! Consider: one blue mana that looks at the top card, decides whether the
//! graveyard is a better place for it, and then draws.

use super::*;

/// Player One holding Consider with one blue up and `library` stacked, the
/// last entry on top.
fn staged(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[0].library.clear();
    for (index, definition) in library.iter().enumerate() {
        game.players[0].library.push(card(
            98_000 + u32::try_from(index).expect("few cards"),
            *definition,
            PlayerId::One,
        ));
    }
    let consider = card(98_500, cards::CONSIDER, PlayerId::One);
    let consider_id = consider.id;
    game.players[0].hand.push(consider);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    (game, consider_id)
}

/// Casts it and answers the surveil, binning the card when `bin` is set.
fn cast_surveilling(game: &mut Game, consider: GameObjectId, bin: bool) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == consider))
        .expect("one blue casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..12 {
        let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        else {
            if game.stack.is_empty() && game.pending_triggers.is_empty() {
                break;
            }
            if game.apply(game.priority, Action::PassPriority).is_err() {
                break;
            }
            continue;
        };
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
    }
    drain_pending(game);
}

fn hand_definitions(game: &Game) -> Vec<CardDefinitionId> {
    game.players[0]
        .hand
        .iter()
        .map(|card| card.definition)
        .collect()
}

/// "You perform the actions stated on a card in sequence." The surveil comes
/// first, so binning the top card means the draw finds the one underneath.
#[test]
fn binning_the_top_card_draws_the_one_beneath_it() {
    // Bottom first: the Bolt is on top and the Bears under it.
    let (mut game, consider) = staged(&[cards::GRIZZLY_BEARS, cards::LIGHTNING_BOLT]);

    cast_surveilling(&mut game, consider, true);

    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .filter(|card| card.definition == cards::LIGHTNING_BOLT)
            .count(),
        1,
        "the Bolt was surveilled away",
    );
    assert_eq!(
        hand_definitions(&game),
        vec![cards::GRIZZLY_BEARS],
        "and the draw found what was under it",
    );
    assert!(game.players[0].library.is_empty(), "two cards, both spent");
}

/// The other answer leaves the card on top, and the draw takes that same
/// card: one mana to look before you take it.
#[test]
fn leaving_it_on_top_draws_that_card() {
    let (mut game, consider) = staged(&[cards::GRIZZLY_BEARS, cards::LIGHTNING_BOLT]);

    cast_surveilling(&mut game, consider, false);

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .all(|card| card.definition != cards::LIGHTNING_BOLT),
        "nothing was binned",
    );
    assert_eq!(
        hand_definitions(&game),
        vec![cards::LIGHTNING_BOLT],
        "the card looked at is the card drawn",
    );
    assert_eq!(
        game.players[0].library.len(),
        1,
        "and the Bears is still where it was",
    );
}

/// Surveilling the last card away leaves nothing to draw, and drawing from
/// an empty library is how the game ends.
#[test]
fn binning_the_last_card_leaves_nothing_to_draw() {
    let (mut game, consider) = staged(&[cards::LIGHTNING_BOLT]);

    cast_surveilling(&mut game, consider, true);
    game.check_state_based_actions();

    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "the only card went to the graveyard",
    );
    assert_eq!(
        game.result(),
        Some(GameResult::Winner {
            winner: PlayerId::Two,
            reason: WinReason::OpponentTriedToDrawFromEmptyLibrary,
        }),
        "and the draw that followed had nothing to take",
    );
}

/// It is an instant: one blue on their turn, in their end step, is as good
/// as one on yours.
#[test]
fn it_may_be_cast_on_their_turn() {
    let (mut game, consider) = staged(&[cards::GRIZZLY_BEARS, cards::LIGHTNING_BOLT]);
    game.active_player = PlayerId::Two;
    game.step = Step::End;
    game.priority = PlayerId::One;

    cast_surveilling(&mut game, consider, true);

    assert_eq!(
        hand_definitions(&game),
        vec![cards::GRIZZLY_BEARS],
        "the card was drawn on their end step",
    );
}
