//! Force of Vigor: two artifacts for nothing on their turn, and the green
//! card that pays for it.
//!
//! Whose turn the free cast wants, and that one Force answers an artifact
//! and an enchantment together, is covered with the other spells. What this
//! adds is which card the alternative cost takes, where that card goes, and
//! what "up to two" allows at the other end.

use super::*;

/// Player One holding a Force with `hand` beside it, on Player Two's turn,
/// with `theirs` on the battlefield across the table.
fn staged(
    hand: &[CardDefinitionId],
    theirs: &[CardDefinitionId],
) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let force = card(96_000, cards::FORCE_OF_VIGOR, PlayerId::One);
    let force_id = force.id;
    game.players[PlayerId::One.index()].hand.push(force);
    for (index, definition) in hand.iter().enumerate() {
        game.players[PlayerId::One.index()].hand.push(card(
            96_100 + u32::try_from(index).expect("a small hand"),
            *definition,
            PlayerId::One,
        ));
    }
    let mut ids = Vec::new();
    for definition in theirs {
        ids.push(
            game.put_onto_battlefield(PlayerId::Two, *definition)
                .expect("cataloged"),
        );
    }
    drain_pending(&mut game);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, force_id, ids)
}

/// Every free cast on offer, by the cards it would exile.
fn free_casts(game: &Game, force: GameObjectId) -> Vec<Vec<GameObjectId>> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell {
                card,
                choices,
                sacrifices,
            } if card == force && choices.costs().alternative().is_some() => Some(sacrifices),
            _ => None,
        })
        .collect()
}

/// "Exile a green card from your hand": green and nothing else, and the
/// card is exiled rather than discarded.
#[test]
fn only_a_green_card_pays_for_it_and_it_goes_to_exile() {
    let (game, force, _) = staged(&[cards::LIGHTNING_BOLT], &[cards::BLACK_LOTUS]);
    assert!(
        free_casts(&game, force).is_empty(),
        "a red card is not a green one",
    );

    let (mut game, force, ids) = staged(&[cards::BIRDS_OF_PARADISE], &[cards::BLACK_LOTUS]);
    let offered = free_casts(&game, force);
    assert!(!offered.is_empty(), "and a green card is");

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == force
                    && choices.costs().alternative().is_some()
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(ids[0]))
            }
            _ => false,
        })
        .expect("the free cast can name the Lotus");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert!(
        game.players[PlayerId::One.index()]
            .exile
            .iter()
            .any(|card| card.definition == cards::BIRDS_OF_PARADISE),
        "the Birds were exiled to pay for it",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .all(|card| card.definition != cards::BIRDS_OF_PARADISE),
        "and not discarded",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == ids[0]),
        "the Lotus is destroyed",
    );
}

/// "Up to two": one is a legal number of targets, and the second artifact on
/// the board is left alone.
#[test]
fn it_may_name_only_one() {
    let (mut game, force, ids) = staged(
        &[cards::BIRDS_OF_PARADISE],
        &[cards::BLACK_LOTUS, cards::SOL_RING],
    );

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == force
                    && choices.costs().alternative().is_some()
                    && choices.iter_targets().count() == 1
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(ids[0]))
            }
            _ => false,
        })
        .expect("naming one of the two is an offer it makes");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == ids[0]),
        "the one it named is gone",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == ids[1]),
        "and the one it did not is still there",
    );
}
