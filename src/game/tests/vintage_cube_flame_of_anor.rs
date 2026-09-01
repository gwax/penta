//! Flame of Anor: one spell that is two when a Wizard is watching.

use super::*;

/// "If you control a Wizard as you cast this spell, you may choose two
/// instead" is read where the spell is offered, so without a Wizard the
/// two-mode selections are not legal actions at all.
#[test]
fn flame_of_anor_offers_a_second_mode_only_beside_a_wizard() {
    for (wizard, expected) in [(false, 3), (true, 6)] {
        let mut game = ready_game();
        game.battlefield.clear();
        if wizard {
            game.battlefield
                .push(creature(76_000, cards::VIVI_ORNITIER, PlayerId::One));
        }
        let flame = card(76_001, cards::FLAME_OF_ANOR, PlayerId::One);
        let flame_id = flame.id;
        game.players[0].hand.push(flame);
        game.battlefield
            .push(creature(76_002, cards::GRIZZLY_BEARS, PlayerId::Two));
        game.battlefield
            .push(creature(76_003, cards::BLACK_LOTUS, PlayerId::Two));
        game.players[0].mana_pool.blue = 1;
        game.players[0].mana_pool.red = 1;
        game.players[0].mana_pool.colorless = 1;

        // One selection can be offered several times over, once per legal
        // target, so the modes are what this counts rather than the actions.
        let mut selections = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .filter_map(|action| match action {
                Action::CastSpell { card, choices, .. } if card == flame_id => {
                    Some(choices.modes().to_vec())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        selections.sort_unstable();
        selections.dedup();

        assert_eq!(
            selections.iter().filter(|modes| modes.len() == 1).count(),
            3,
            "all three modes are always on offer",
        );
        assert_eq!(
            selections.len(),
            expected,
            "with{} a Wizard on the battlefield",
            if wizard { "" } else { "out" },
        );
    }
}

/// Both chosen modes resolve, each against its own target: the draw and the
/// damage are one spell, not two.
#[test]
fn flame_of_anor_resolves_both_chosen_modes() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.battlefield
        .push(creature(76_010, cards::VIVI_ORNITIER, PlayerId::One));
    let bears = creature(76_011, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let flame = card(76_012, cards::FLAME_OF_ANOR, PlayerId::One);
    let flame_id = flame.id;
    game.players[0].hand.push(flame);
    game.players[0].mana_pool.blue = 1;
    game.players[0].mana_pool.red = 1;
    game.players[0].mana_pool.colorless = 1;
    let before = game.players[0].hand.len();

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } if *card == flame_id => {
                choices.modes() == [ModeId(0), ModeId(2)]
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(bears_id))
            }
            _ => false,
        })
        .expect("drawing and burning is a legal pair beside a Wizard");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(&mut game);

    // One card left the hand and two were drawn.
    assert_eq!(game.players[0].hand.len(), before + 1);
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "five damage kills a 2/2",
    );
}

/// "Once you cast Flame of Anor and choose two modes, it doesn't matter what
/// happens to the Wizard you control in response." The Wizard is read as the
/// spell is cast, and the spell keeps what it chose.
#[test]
fn flame_of_anor_keeps_both_modes_when_the_wizard_dies() {
    let mut game = ready_game();
    game.battlefield.clear();
    let vivi = creature(76_020, cards::VIVI_ORNITIER, PlayerId::One);
    let vivi_id = vivi.card.id;
    game.battlefield.push(vivi);
    let bears = creature(76_021, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let flame = card(76_022, cards::FLAME_OF_ANOR, PlayerId::One);
    let flame_id = flame.id;
    game.players[0].hand.push(flame);
    game.players[0].mana_pool.blue = 1;
    game.players[0].mana_pool.red = 1;
    game.players[0].mana_pool.colorless = 1;
    let before = game.players[0].hand.len();

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } if *card == flame_id => {
                choices.modes() == [ModeId(0), ModeId(2)]
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(bears_id))
            }
            _ => false,
        })
        .expect("drawing and burning is a legal pair beside a Wizard");
    game.apply(PlayerId::One, action).expect("it is cast");

    // The Wizard that bought the second mode is answered in response.
    game.move_permanents_to_graveyard(&[vivi_id]);
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].hand.len(),
        before + 1,
        "the draw happened even with the Wizard gone",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears_id),
        "and so did the five damage",
    );
}

/// "If you control a Wizard": a Wizard of theirs buys you nothing, so the
/// second mode stays off the table.
#[test]
fn their_wizard_does_not_unlock_the_second_mode() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.battlefield
        .push(creature(76_030, cards::VIVI_ORNITIER, PlayerId::Two));
    let flame = card(76_031, cards::FLAME_OF_ANOR, PlayerId::One);
    let flame_id = flame.id;
    game.players[0].hand.push(flame);
    game.battlefield
        .push(creature(76_032, cards::GRIZZLY_BEARS, PlayerId::Two));
    game.battlefield
        .push(creature(76_033, cards::BLACK_LOTUS, PlayerId::Two));
    game.players[0].mana_pool.blue = 1;
    game.players[0].mana_pool.red = 1;
    game.players[0].mana_pool.colorless = 1;

    let pairs = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == flame_id && choices.modes().len() == 2
            }
            _ => false,
        })
        .count();

    assert_eq!(pairs, 0, "the Wizard is theirs and the clause is yours");
}

/// The artifact mode, and what happens to a pair when one half loses its
/// target: CR 608.2b, the spell does as much as it can. The Bears die in
/// response and the Lotus is destroyed regardless.
#[test]
fn a_pair_that_loses_one_target_still_does_the_other() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.battlefield
        .push(creature(76_040, cards::VIVI_ORNITIER, PlayerId::One));
    let bears = creature(76_041, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let lotus = creature(76_042, cards::BLACK_LOTUS, PlayerId::Two);
    let lotus_id = lotus.card.id;
    game.battlefield.push(lotus);
    let flame = card(76_043, cards::FLAME_OF_ANOR, PlayerId::One);
    let flame_id = flame.id;
    game.players[0].hand.push(flame);
    game.players[0].mana_pool.blue = 1;
    game.players[0].mana_pool.red = 1;
    game.players[0].mana_pool.colorless = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } if *card == flame_id => {
                choices.modes() == [ModeId(1), ModeId(2)]
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(bears_id))
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(lotus_id))
            }
            _ => false,
        })
        .expect("shattering and burning is a legal pair beside a Wizard");
    game.apply(PlayerId::One, action).expect("it is cast");

    // The creature half loses its target underneath the spell.
    game.move_permanents_to_graveyard(&[bears_id]);
    game.check_state_based_actions();
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == lotus_id),
        "the artifact half went through on its own legal target",
    );
    assert!(
        game.players[PlayerId::Two.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::BLACK_LOTUS),
    );
}

/// "Target player" is any player: the draw half can be pointed across the
/// table, which is how the pair is aimed when you want the burn and they want
/// nothing.
#[test]
fn the_draw_half_may_be_aimed_at_the_opponent() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.battlefield
        .push(creature(76_050, cards::VIVI_ORNITIER, PlayerId::One));
    let flame = card(76_051, cards::FLAME_OF_ANOR, PlayerId::One);
    let flame_id = flame.id;
    game.players[0].hand.push(flame);
    game.players[0].mana_pool.blue = 1;
    game.players[0].mana_pool.red = 1;
    game.players[0].mana_pool.colorless = 1;
    let mine = game.players[0].hand.len();
    let theirs = game.players[1].hand.len();

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } if *card == flame_id => {
                choices.modes() == [ModeId(0)]
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Player(PlayerId::Two))
            }
            _ => false,
        })
        .expect("the draw half takes either player");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(
        game.players[1].hand.len(),
        theirs + 2,
        "the player it targeted drew the two",
    );
    assert_eq!(
        game.players[0].hand.len(),
        mine - 1,
        "and the caster only spent the spell",
    );
}

/// CR 608.2b the other way: when a pair loses *both* of its targets the spell
/// is countered on resolution and does nothing at all.
#[test]
fn a_pair_that_loses_both_targets_does_nothing() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.battlefield
        .push(creature(76_060, cards::VIVI_ORNITIER, PlayerId::One));
    let bears = creature(76_061, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let lotus = creature(76_062, cards::BLACK_LOTUS, PlayerId::Two);
    let lotus_id = lotus.card.id;
    game.battlefield.push(lotus);
    let flame = card(76_063, cards::FLAME_OF_ANOR, PlayerId::One);
    let flame_id = flame.id;
    game.players[0].hand.push(flame);
    game.players[0].mana_pool.blue = 1;
    game.players[0].mana_pool.red = 1;
    game.players[0].mana_pool.colorless = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } if *card == flame_id => {
                choices.modes() == [ModeId(1), ModeId(2)]
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(bears_id))
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(lotus_id))
            }
            _ => false,
        })
        .expect("shattering and burning is a legal pair beside a Wizard");
    game.apply(PlayerId::One, action).expect("it is cast");

    // Both halves lose their target underneath the spell.
    game.move_permanents_to_graveyard(&[bears_id, lotus_id]);
    game.check_state_based_actions();
    drain_pending(&mut game);

    assert!(game.stack.is_empty(), "the spell left the stack");
    assert!(
        game.events.iter().any(|event| matches!(
            event,
            GameEvent::SpellFizzled { definition, .. } if *definition == cards::FLAME_OF_ANOR
        )),
        "it was countered on resolution rather than resolving for nothing",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::FLAME_OF_ANOR),
        "and went to its owner's graveyard",
    );
}
