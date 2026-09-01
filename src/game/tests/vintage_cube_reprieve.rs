//! Reprieve: two mana that takes a spell off the stack without countering
//! it, and replaces itself while it does.

use super::*;

/// The spell goes back to its owner's hand rather than the graveyard, and
/// Reprieve replaces itself.
#[test]
fn reprieve_returns_a_spell_and_draws() {
    let mut game = ready_game();
    game.battlefield.clear();
    let bears = card(81_000, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.id;
    game.players[0].hand.push(bears);
    let reprieve = card(81_001, cards::REPRIEVE, PlayerId::Two);
    let reprieve_id = reprieve.id;
    game.players[1].hand.push(reprieve);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::White, 2);
    let held_before = game.players[1].hand.len();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == bears_id))
        .expect("the Bears are castable");
    game.apply(PlayerId::One, cast).expect("they are cast");
    let spell = game.stack.last().expect("the Bears are on the stack").id;
    game.apply(PlayerId::One, Action::PassPriority).unwrap();

    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == reprieve_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(spell))
            }
            _ => false,
        })
        .expect("Reprieve can point at a spell");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    pass_until_decision(&mut game);
    drain_pending(&mut game);

    assert!(
        game.stack.is_empty(),
        "the Bears left the stack with Reprieve",
    );
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "and went back to their owner's hand",
    );
    assert!(
        game.players[0].graveyard.is_empty(),
        "rather than to the graveyard",
    );
    // Reprieve left the hand and one card was drawn, so the count is level.
    assert_eq!(game.players[1].hand.len(), held_before);
}

/// Returning a spell is not countering it, so a spell that cannot be
/// countered goes back all the same.
#[test]
fn reprieve_answers_a_spell_that_cannot_be_countered() {
    let mut game = ready_game();
    game.battlefield.clear();
    let halfling = creature(81_010, cards::DELIGHTED_HALFLING, PlayerId::One);
    let halfling_id = halfling.card.id;
    game.battlefield.push(halfling);
    let tifa = card(81_011, cards::TIFA_LOCKHART, PlayerId::One);
    let tifa_id = tifa.id;
    game.players[0].hand.push(tifa);
    let reprieve = card(81_012, cards::REPRIEVE, PlayerId::Two);
    let reprieve_id = reprieve.id;
    game.players[1].hand.push(reprieve);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::White, 2);

    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: halfling_id,
            ability: mana_ability_for(&game, halfling_id, ManaColor::Green),
            color: ManaColor::Green,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("it taps for green");
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == tifa_id))
        .expect("a legendary spell is castable on this mana");
    game.apply(PlayerId::One, cast).expect("it is cast");
    let spell = game.stack.last().expect("Tifa is on the stack").id;
    game.apply(PlayerId::One, Action::PassPriority).unwrap();

    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == reprieve_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(spell))
            }
            _ => false,
        })
        .expect("Reprieve can point at an uncounterable spell");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    pass_until_decision(&mut game);
    drain_pending(&mut game);

    assert!(game.stack.is_empty(), "she left the stack");
    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::TIFA_LOCKHART),
        "and is back in hand despite not being counterable",
    );
}

/// "Target spell" names no controller: your own is a legal target, which is
/// how it answers a counterspell rather than a threat.
#[test]
fn it_may_take_back_your_own_spell() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let bears = card(82_000, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.id;
    game.players[PlayerId::One.index()].hand.push(bears);
    let reprieve = card(82_001, cards::REPRIEVE, PlayerId::One);
    let reprieve_id = reprieve.id;
    game.players[PlayerId::One.index()].hand.push(reprieve);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    let library = game.players[PlayerId::One.index()].library.len();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == bears_id))
        .expect("the Bears are castable");
    game.apply(PlayerId::One, cast).expect("they are cast");
    let spell = game.stack.last().expect("the Bears are on the stack").id;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == reprieve_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(spell))
            }
            _ => false,
        })
        .expect("your own spell is as good a target as theirs");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_until_decision(&mut game);
    drain_pending(&mut game);

    assert!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "the Bears came back to the hand they were cast from",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].library.len(),
        library - 1,
        "and the draw is yours whoever the spell belonged to",
    );
}

/// The target is the whole of what it does: a spell that has already left
/// the stack takes the Reprieve with it, and the card it would have drawn.
#[test]
fn a_reprieve_with_nothing_left_to_return_draws_nothing() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    let bears = card(82_100, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.id;
    game.players[PlayerId::One.index()].hand.push(bears);
    let mine = card(82_101, cards::REPRIEVE, PlayerId::One);
    let mine_id = mine.id;
    game.players[PlayerId::One.index()].hand.push(mine);
    let theirs = card(82_102, cards::REPRIEVE, PlayerId::Two);
    let theirs_id = theirs.id;
    game.players[PlayerId::Two.index()].hand.push(theirs);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Colorless, 1);
    let their_library = game.players[PlayerId::Two.index()].library.len();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == bears_id))
        .expect("the Bears are castable");
    game.apply(PlayerId::One, cast).expect("they are cast");
    let spell = game.stack.last().expect("the Bears are on the stack").id;
    game.apply(PlayerId::One, Action::PassPriority).unwrap();

    // They point their Reprieve at the Bears...
    let cast = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == theirs_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(spell))
            }
            _ => false,
        })
        .expect("Reprieve can point at a spell");
    game.apply(PlayerId::Two, cast).expect("it is cast");
    game.apply(PlayerId::Two, Action::PassPriority)
        .expect("they pass it back with their Reprieve waiting");

    // ...and you take the Bears back first, leaving their Reprieve with
    // nothing to return.
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == mine_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(spell))
            }
            _ => false,
        })
        .expect("the Bears are still on the stack to be taken back");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_until_decision(&mut game);
    drain_pending(&mut game);

    assert!(
        game.players[PlayerId::One.index()]
            .hand
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "the Bears went back to hand, taken by your own Reprieve",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].library.len(),
        their_library,
        "and theirs never resolved, so it never drew",
    );
    assert!(
        game.players[PlayerId::Two.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::REPRIEVE),
        "it is in the graveyard, countered for want of a target",
    );
}
