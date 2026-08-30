//! Mystical Tutor: the card it finds goes back on top, face up on the way.
//!
//! `vintage_cube_library` checks what the search offers; what it does with
//! the answer lives here, because that file has no room left in it.

use super::*;

/// Casts the Tutor with a stacked library and takes `wanted` out of it.
fn tutor_for(wanted: CardDefinitionId) -> Game {
    let mut game = ready_game();
    game.players[0].library.clear();
    stack_library(
        &mut game,
        &[
            (50_700, cards::GRIZZLY_BEARS),
            (50_701, cards::LIGHTNING_BOLT),
            (50_702, cards::ANCESTRAL_RECALL),
        ],
    );
    let tutor = card(50_703, cards::MYSTICAL_TUTOR, PlayerId::One);
    let tutor_id = tutor.id;
    game.players[0].hand.push(tutor);
    game.players[0].mana_pool.blue = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == tutor_id))
        .expect("one blue mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    pass_priority_pair(&mut game);

    let decision = game.observe(PlayerId::One).decision.expect("a search");
    let chosen = decision
        .options
        .iter()
        .find(|option| {
            option.card.is_some_and(|(_, characteristics)| {
                characteristics.card_definition() == Some(wanted)
            })
        })
        .expect("the card asked for is on offer")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![chosen],
        },
    )
    .expect("the search is answered");
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    game
}

/// "Shuffle and put that card on top": what it finds stays in the library
/// and is the next draw.
#[test]
fn what_it_finds_ends_up_on_top() {
    let game = tutor_for(cards::ANCESTRAL_RECALL);

    assert_eq!(
        game.players[0].library.last().map(|card| card.definition),
        Some(cards::ANCESTRAL_RECALL),
        "the found card survives the shuffle on top",
    );
    assert_eq!(
        game.players[0].library.len(),
        3,
        "and it never left the library: three cards in, three out",
    );
    assert!(
        game.players[0].hand.is_empty(),
        "the Tutor draws nothing itself",
    );
}

/// "Reveal it": unlike Vampiric Tutor, what it takes is public.
#[test]
fn the_card_it_takes_is_revealed() {
    let game = tutor_for(cards::LIGHTNING_BOLT);

    assert!(
        game.events().iter().any(|event| matches!(
            event,
            GameEvent::CardRevealed {
                player: PlayerId::One,
                definition: cards::LIGHTNING_BOLT,
                ..
            }
        )),
        "the Bolt was shown as it was taken",
    );
}
