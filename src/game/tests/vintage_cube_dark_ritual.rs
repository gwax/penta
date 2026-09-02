//! Dark Ritual: one black into three, on the stack where anyone can see it.

use super::*;

/// Player One holding a Ritual with one black up.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::Two.index()].hand.clear();
    let ritual = game
        .build_zone(PlayerId::One, &[cards::DARK_RITUAL])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let ritual_id = ritual.id;
    game.players[PlayerId::One.index()].hand.push(ritual);
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    (game, ritual_id)
}

fn cast(game: &Game, ritual: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == ritual))
}

fn pool(game: &Game) -> Vec<ManaColor> {
    game.players[PlayerId::One.index()]
        .mana
        .iter()
        .map(|mana| mana.color)
        .collect()
}

/// One black in, three black out: the two-mana profit the deck is built on.
#[test]
fn it_turns_one_black_into_three() {
    let (mut game, ritual) = staged();
    assert_eq!(pool(&game), vec![ManaColor::Black], "one to pay with");

    let action = cast(&game, ritual).expect("one black casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(
        pool(&game),
        vec![ManaColor::Black; 3],
        "three black, and black specifically",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::DARK_RITUAL),
        "and the Ritual resolved into the graveyard",
    );
}

/// Three black is three black: it pays a black spell and gets nowhere near a
/// red one, whatever the total says.
#[test]
fn the_three_it_makes_are_black() {
    let (mut game, ritual) = staged();
    let action = cast(&game, ritual).expect("one black casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(&mut game);

    let bolt = game
        .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let bolt_id = bolt.id;
    game.players[PlayerId::One.index()].hand.push(bolt);
    let tutor = game
        .build_zone(PlayerId::One, &[cards::DEMONIC_TUTOR])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let tutor_id = tutor.id;
    game.players[PlayerId::One.index()].hand.push(tutor);

    let castable = |game: &Game, id| {
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == id))
    };
    assert!(
        castable(&game, tutor_id),
        "a {{1}}{{B}} sorcery is what the three are for",
    );
    assert!(
        !castable(&game, bolt_id),
        "and none of them is the red one a Bolt wants",
    );
}

/// The whole difference between a Ritual and a Talisman: this is a spell. It
/// waits on the stack where the other player can answer it, and a countered
/// Ritual makes no mana at all -- the black that paid for it is simply gone.
#[test]
fn it_is_a_spell_and_a_countered_one_makes_nothing() {
    let (mut game, ritual) = staged();
    let counter = game
        .build_zone(PlayerId::Two, &[cards::COUNTERSPELL])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let counter_id = counter.id;
    game.players[PlayerId::Two.index()].hand.push(counter);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 2);

    let action = cast(&game, ritual).expect("one black casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    let spell = game
        .stack
        .iter()
        .find(|object| object.controller == PlayerId::One)
        .expect("a mana ability would never have got here")
        .id;
    assert!(
        pool(&game).is_empty(),
        "the black that paid for it is spent while it waits",
    );

    game.priority = PlayerId::Two;
    let answer = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == counter_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(spell))
            }
            _ => false,
        })
        .expect("Counterspell may point at it");
    game.apply(PlayerId::Two, answer).expect("it is cast");
    drain_pending(&mut game);

    assert!(
        pool(&game).is_empty(),
        "countered, so the three never arrived",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::DARK_RITUAL),
        "and the Ritual is in the graveyard having done nothing",
    );
}
