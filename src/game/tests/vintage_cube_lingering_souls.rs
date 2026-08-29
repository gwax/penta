//! Lingering Souls: four fliers from one card, half of them out of the
//! graveyard, which is why the card is a discard rather than a loss.

use super::*;

/// Player One with the Souls in `zone`, the mana for both halves, and an
/// empty board.
fn staged(in_graveyard: bool) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    let souls = card(127_000, cards::LINGERING_SOULS, PlayerId::One);
    let souls_id = souls.id;
    if in_graveyard {
        game.players[0].graveyard.push(souls);
    } else {
        game.players[0].hand.push(souls);
    }
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, souls_id)
}

fn cast(game: &mut Game, souls: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == souls))
        .expect("the mana is there");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(game);
}

fn spirits(game: &Game) -> Vec<&Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| {
            is_token_with(
                permanent,
                token_with_flying(tokens::creature(&["Spirit"], &[ManaColor::White], 1, 1)),
            )
        })
        .collect()
}

/// Two 1/1 white Spirits with flying, from hand.
#[test]
fn it_makes_two_flying_spirits() {
    let (mut game, souls) = staged(false);

    cast(&mut game, souls);

    let made = spirits(&game);
    assert_eq!(made.len(), 2, "two of them");
    assert_eq!(game.power(made[0]), Some(1));
    assert!(game.has_flying(made[0]), "and they fly");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::LINGERING_SOULS),
        "the card is in the graveyard, where the other half waits",
    );
}

/// Flashback {1}{B}: two more out of the graveyard, and the card is exiled
/// rather than buried again.
#[test]
fn flashback_makes_two_more_and_exiles_the_card() {
    let (mut game, souls) = staged(true);

    cast(&mut game, souls);

    assert_eq!(spirits(&game).len(), 2, "the second pair");
    assert!(
        game.players[0].graveyard.is_empty(),
        "the card did not go back to the graveyard",
    );
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::LINGERING_SOULS),
        "it was exiled, which is what ends the card",
    );
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == souls)),
        "and there is no third casting",
    );
}

/// "You must still follow any timing restrictions ... you can cast a sorcery
/// using flashback only when you could normally cast a sorcery."
#[test]
fn flashback_still_waits_for_a_main_phase() {
    let (mut game, souls) = staged(true);
    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == souls)),
        "your main phase with an empty stack",
    );

    game.step = Step::DeclareBlockers;
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == souls)),
        "and nowhere else: flashback changes the zone, not the timing",
    );
}
