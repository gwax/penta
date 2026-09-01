//! Stern Scolding: one blue for a creature spell that is small in either
//! direction.
//!
//! Which sizes it may name is pinned in `vintage_cube_spells`. What is here
//! is where those sizes are read: on the stack, where an anthem on the
//! battlefield has nothing to say.

use super::*;

/// Player One casting `spell` with Player Two holding the Scolding.
fn staged(spell: CardDefinitionId, anthem: bool) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    if anthem {
        game.put_onto_battlefield(PlayerId::One, cards::BAD_MOON)
            .expect("cataloged");
        drain_pending(&mut game);
    }
    let cast = game
        .build_zone(PlayerId::One, &[spell])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let cast_id = cast.id;
    game.players[0].hand.push(cast);
    let scolding = game
        .build_zone(PlayerId::Two, &[cards::STERN_SCOLDING])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let scolding_id = scolding.id;
    game.players[1].hand.push(scolding);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    for color in ManaColor::COLORS {
        game.add_unrestricted_mana(PlayerId::One, color, 3);
    }
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 1);
    (game, cast_id, scolding_id)
}

/// Casts the creature and hands priority to the other seat, returning the
/// spell's id on the stack.
fn cast_it(game: &mut Game, spell: GameObjectId) -> GameObjectId {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell))
        .expect("the mana is there");
    game.apply(PlayerId::One, cast).expect("it is cast");
    let on_stack = game.stack.last().expect("it is on the stack").id;
    game.apply(PlayerId::One, Action::PassPriority)
        .expect("they pass");
    on_stack
}

fn scolding_offered(game: &Game, scolding: GameObjectId, spell: GameObjectId) -> bool {
    game.legal_actions(PlayerId::Two).into_iter().any(|action| {
        matches!(action, Action::CastSpell { card, ref choices, .. }
            if card == scolding
                && choices
                    .iter_targets()
                    .any(|target| *target == Target::Spell(spell)))
    })
}

/// An anthem pumps permanents, not the spells on their way to becoming
/// them: a Black Knight under a Bad Moon is a 2/2 while it is on the stack,
/// and the Scolding may still name it.
#[test]
fn an_anthem_does_not_protect_the_spell_it_will_pump() {
    let (mut game, knight, scolding) = staged(cards::BLACK_KNIGHT, true);
    let on_stack = cast_it(&mut game, knight);

    assert!(
        scolding_offered(&game, scolding, on_stack),
        "the spell is a 2/2 wherever the Bad Moon is",
    );

    // And the anthem is real: let the same Knight through and it is a 3/3.
    let (mut game, knight, _) = staged(cards::BLACK_KNIGHT, true);
    cast_it(&mut game, knight);
    drain_pending(&mut game);
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::BLACK_KNIGHT)
        .expect("it resolved");
    assert_eq!(
        (game.power(permanent), game.toughness(permanent)),
        (Some(3), Some(3)),
        "the Bad Moon pumps what arrives, which is why the stack read matters",
    );
}

/// "Target creature spell" does not say whose: your own is as legal a
/// target as theirs.
#[test]
fn it_may_name_your_own_creature_spell() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let bears = game
        .build_zone(PlayerId::One, &[cards::GRIZZLY_BEARS])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let bears_id = bears.id;
    game.players[0].hand.push(bears);
    let scolding = game
        .build_zone(PlayerId::One, &[cards::STERN_SCOLDING])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let scolding_id = scolding.id;
    game.players[0].hand.push(scolding);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == bears_id))
        .expect("two mana casts the Bears");
    game.apply(PlayerId::One, cast).expect("it is cast");
    let on_stack = game.stack.last().expect("it is on the stack").id;

    let answer = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == scolding_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(on_stack))
            }
            _ => false,
        })
        .expect("your own creature spell is a legal target");
    game.apply(PlayerId::One, answer).expect("it is cast");
    drain_pending(&mut game);

    assert!(game.battlefield.is_empty(), "the Bears never arrived");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::GRIZZLY_BEARS),
        "it was countered into your own graveyard",
    );
}
