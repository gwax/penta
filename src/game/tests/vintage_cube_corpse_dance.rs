//! Corpse Dance: the top creature in your graveyard, hasty, until the end
//! step takes it away.
//!
//! Which cost was paid and where the card goes afterwards is covered with
//! the other spells. What this adds is the end of the deal: the exile it
//! promises, the condition on that exile, and which creature it is that
//! comes back.

use super::*;

/// Player One holding a Corpse Dance with mana for either price and
/// `graveyard` behind them, bottom-first.
fn staged(graveyard: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].graveyard.clear();
    for (index, definition) in graveyard.iter().enumerate() {
        game.players[PlayerId::One.index()].graveyard.push(card(
            97_000 + u32::try_from(index).expect("a small graveyard"),
            *definition,
            PlayerId::One,
        ));
    }
    let dance = card(97_500, cards::CORPSE_DANCE, PlayerId::One);
    let dance_id = dance.id;
    game.players[PlayerId::One.index()].hand.push(dance);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 4);
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, dance_id)
}

/// Casts it for its printed cost and lets it resolve.
fn dance(game: &mut Game, dance: GameObjectId) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == dance && choices.costs().additional().is_empty()
            }
            _ => false,
        })
        .expect("three mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_until_decision(game);
    drain_pending(game);
}

/// Runs Player One's end step.
fn end_step(game: &mut Game) {
    game.active_player = PlayerId::One;
    game.step = Step::PostcombatMain;
    game.priority = PlayerId::One;
    for _ in 0..16 {
        game.advance_step();
        drain_pending(game);
        if game.step == Step::End || game.step == Step::Cleanup {
            break;
        }
    }
    drain_pending(game);
    game.check_state_based_actions();
}

/// "Exile it at the beginning of the next end step": the loan is for one
/// turn and the card does not come back to the graveyard.
#[test]
fn the_end_step_exiles_what_it_lent_you() {
    let (mut game, spell) = staged(&[cards::GRAVE_TITAN]);

    dance(&mut game, spell);
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::GRAVE_TITAN),
        "the Titan is back",
    );

    end_step(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::GRAVE_TITAN),
        "and the end step took it",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .exile
            .iter()
            .any(|card| card.definition == cards::GRAVE_TITAN),
        "to exile rather than back to the graveyard",
    );
}

/// "Corpse Dance only exiles the card if the card is still on the
/// battlefield at the beginning of the end step." Answered first, it stays
/// in the graveyard where the answer put it.
#[test]
fn a_creature_that_died_first_is_not_exiled() {
    let (mut game, spell) = staged(&[cards::GRIZZLY_BEARS]);
    dance(&mut game, spell);
    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS)
        .expect("the Bears are back")
        .card
        .id;

    game.move_permanents_to_graveyard(&[bears]);
    game.check_state_based_actions();
    drain_pending(&mut game);

    end_step(&mut game);

    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "the delayed exile found nothing on the battlefield to take",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .exile
            .iter()
            .all(|card| card.definition != cards::GRIZZLY_BEARS),
        "so nothing was exiled",
    );
}

/// "The top creature card": the topmost creature, with everything above it
/// passed over and left where it lies.
#[test]
fn it_takes_the_topmost_creature_and_leaves_the_rest() {
    let (mut game, spell) = staged(&[
        cards::GRAVE_TITAN,
        cards::SERRA_ANGEL,
        cards::LIGHTNING_BOLT,
    ]);

    dance(&mut game, spell);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::SERRA_ANGEL),
        "the Angel is the creature nearest the top",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::GRAVE_TITAN),
        "the Titan is beneath it and stays there",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "and the Bolt above it was passed over rather than moved",
    );
}
