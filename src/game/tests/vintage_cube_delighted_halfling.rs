//! Delighted Halfling: what "a legendary spell" covers.
//!
//! The restriction itself, the uncounterable rider it carries, and the
//! ordinary colourless half are checked where the mana sources live as a
//! family. What this adds is the breadth of the word legendary: it is not a
//! clause about creatures.

use super::*;

/// A Halfling out since last turn, with `held` in hand.
fn staged(held: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let halfling = game
        .put_onto_battlefield(PlayerId::One, cards::DELIGHTED_HALFLING)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    let mut ids = Vec::new();
    for (index, definition) in held.iter().enumerate() {
        let card = card(
            103_000 + u32::try_from(index).expect("a small hand"),
            *definition,
            PlayerId::One,
        );
        ids.push(card.id);
        game.players[PlayerId::One.index()].hand.push(card);
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, halfling, ids)
}

/// Taps the Halfling for its restricted colour.
fn tap_for_color(game: &mut Game, halfling: GameObjectId, color: ManaColor) {
    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: halfling,
            ability: mana_ability_for(game, halfling, color),
            color,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("it taps for that colour");
}

fn castable(game: &Game, spell: GameObjectId) -> bool {
    game.legal_actions(PlayerId::One)
        .iter()
        .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
}

/// A legendary artifact is a legendary spell: the Jitte is castable on the
/// Halfling's mana, and a Sol Ring beside it is not.
#[test]
fn legendary_is_not_a_word_about_creatures() {
    let (mut game, halfling, held) = staged(&[cards::UMEZAWAS_JITTE, cards::SMUGGLER_S_COPTER]);
    let (jitte, copter) = (held[0], held[1]);
    tap_for_color(&mut game, halfling, ManaColor::White);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    assert!(
        castable(&game, jitte),
        "two mana, one of them restricted, and the Jitte is legendary",
    );
    assert!(
        !castable(&game, copter),
        "the same two will not cast a Smuggler's Copter, which is not",
    );

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == jitte))
        .expect("it is castable");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::UMEZAWAS_JITTE),
        "and it resolves onto the battlefield",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.total(),
        0,
        "with both mana spent, restricted and ordinary alike",
    );
}

/// Playing a land is not casting a spell, so the restricted mana is no help
/// with one -- legendary or not.
#[test]
fn a_legendary_land_is_no_spell_at_all() {
    let (mut game, halfling, held) = staged(&[cards::GAEAS_CRADLE]);
    let cradle = held[0];
    tap_for_color(&mut game, halfling, ManaColor::Green);
    game.players[PlayerId::One.index()].lands_played_this_turn = 0;

    // The land drop is what plays it; the mana is beside the point.
    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::PlayLand { card, .. } if *card == cradle)),
        "a land is played rather than cast",
    );
    assert!(
        !castable(&game, cradle),
        "and there is no cast of it for the mana to pay for",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.total(),
        1,
        "the restricted mana is still sitting there",
    );
}
