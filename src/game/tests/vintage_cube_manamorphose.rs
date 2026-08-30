//! Manamorphose: two mana of whatever colours you needed, and the card back.

use super::*;

/// Player One holding a Manamorphose with exactly the mana to cast it.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let morph = game
        .build_zone(PlayerId::One, &[cards::MANAMORPHOSE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = morph.id;
    game.players[0].hand.push(morph);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.priority = PlayerId::One;
    (game, id)
}

/// Casts it and answers each colour question with `colors` in turn.
fn cast(game: &mut Game, morph: GameObjectId, colors: &[ManaColor]) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == morph))
        .expect("a red and a generic pays for it");
    game.apply(PlayerId::One, cast).expect("it is castable");

    let mut asked = 0;
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let wanted = colors.get(asked).copied().unwrap_or(ManaColor::Blue);
            let label = format!("{wanted:?}");
            let options = decision
                .options
                .iter()
                .find(|option| option.label.eq_ignore_ascii_case(&label))
                .or_else(|| decision.options.first())
                .map(|option| vec![option.id])
                .unwrap_or_default();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the decision accepts what it offered");
            asked += 1;
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            return;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            return;
        }
    }
}

/// Two mana of one colour is one of the combinations.
#[test]
fn both_mana_may_be_the_same_colour() {
    let (mut game, morph) = staged();
    cast(&mut game, morph, &[ManaColor::Blue, ManaColor::Blue]);

    assert_eq!(game.players[0].mana_pool.blue, 2);
    assert_eq!(game.players[0].mana_pool.red, 0, "the red paid for it");
}

/// And so is two different colours, which is what "any combination" adds
/// over a choice of one colour for the pair.
#[test]
fn the_two_mana_may_be_different_colours() {
    let (mut game, morph) = staged();
    cast(&mut game, morph, &[ManaColor::White, ManaColor::Green]);

    assert_eq!(game.players[0].mana_pool.white, 1);
    assert_eq!(game.players[0].mana_pool.green, 1);
}

/// It replaces itself, which is the whole reason a deck plays it.
#[test]
fn it_draws_a_card() {
    let (mut game, morph) = staged();
    let before = game.players[0].hand.len();
    cast(&mut game, morph, &[ManaColor::Blue, ManaColor::Blue]);

    assert_eq!(
        game.players[0].hand.len(),
        before - 1 + 1,
        "the Manamorphose left hand and a card came back",
    );
}

/// Its hybrid half may be paid with either colour.
#[test]
fn the_hybrid_half_takes_either_colour() {
    for color in [ManaColor::Red, ManaColor::Green] {
        let mut game = ready_game();
        game.battlefield.clear();
        let morph = game
            .build_zone(PlayerId::One, &[cards::MANAMORPHOSE])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        let id = morph.id;
        game.players[0].hand.push(morph);
        game.add_unrestricted_mana(PlayerId::One, color, 1);
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
        game.priority = PlayerId::One;

        assert!(
            game.legal_actions(PlayerId::One)
                .iter()
                .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == id)),
            "{color:?} pays the hybrid half",
        );
    }
}

/// "You choose which color or colors of mana to add before you draw a card."
/// While the question is still on the table nothing has been drawn.
#[test]
fn the_colours_are_chosen_before_the_card_is_drawn() {
    let (mut game, morph) = staged();
    let hand = game.players[0].hand.len();
    let library = game.players[0].library.len();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == morph))
        .expect("a red and a generic pays for it");
    game.apply(PlayerId::One, cast).expect("it is castable");
    for _ in 0..8 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }

    assert!(
        !game.pending_decisions.is_empty(),
        "it asks which colours to add",
    );
    assert_eq!(
        game.players[0].hand.len(),
        hand - 1,
        "and the only card that has left the hand is the Manamorphose itself",
    );
    assert_eq!(
        game.players[0].library.len(),
        library,
        "the draw waits until the colours are settled",
    );
}

/// "In any combination of colors": the five colours and nothing else, so
/// there is no way to ask it for colourless.
#[test]
fn it_offers_colours_and_no_colourless() {
    let (mut game, morph) = staged();
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == morph))
        .expect("a red and a generic pays for it");
    game.apply(PlayerId::One, cast).expect("it is castable");
    for _ in 0..8 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }

    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("it asks which colour to add");
    let labels = decision
        .options
        .iter()
        .map(|option| option.label.clone())
        .collect::<Vec<_>>();
    assert_eq!(labels.len(), 5, "five colours on offer: {labels:?}");
    assert!(
        !labels
            .iter()
            .any(|label| label.eq_ignore_ascii_case("colorless")),
        "and colourless is not one of them: {labels:?}",
    );
}
