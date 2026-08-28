//! Wrath of God: what a sweeper takes, and what it leaves standing.

use super::*;

/// Casts the Wrath from Player One's hand and lets it resolve.
fn wrath(game: &mut Game) {
    let spell = game
        .build_zone(PlayerId::One, &[cards::WRATH_OF_GOD])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let spell_id = spell.id;
    game.players[0].hand.push(spell);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 2);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.priority = PlayerId::One;
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == spell_id))
        .expect("four mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(game);
    game.check_state_based_actions();
}

fn on_battlefield(game: &Game, definition: CardDefinitionId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.definition == definition)
}

/// "All creatures" reaches across the table, and it stops at creatures: the
/// artifact beside them is untouched, and so is the one that cannot be
/// destroyed.
#[test]
fn it_sweeps_both_boards_and_leaves_the_rest() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    for definition in [cards::GRIZZLY_BEARS, cards::SOL_RING] {
        game.put_onto_battlefield(PlayerId::One, definition)
            .expect("cataloged");
    }
    for definition in [cards::SERRA_ANGEL, cards::BLIGHTSTEEL_COLOSSUS] {
        game.put_onto_battlefield(PlayerId::Two, definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);

    wrath(&mut game);

    assert!(!on_battlefield(&game, cards::GRIZZLY_BEARS), "yours died");
    assert!(!on_battlefield(&game, cards::SERRA_ANGEL), "and theirs did");
    assert!(
        on_battlefield(&game, cards::SOL_RING),
        "an artifact is not a creature",
    );
    assert!(
        on_battlefield(&game, cards::BLIGHTSTEEL_COLOSSUS),
        "and destroy does nothing to what cannot be destroyed",
    );
}

/// A token that is destroyed ceases to exist rather than going anywhere, and
/// the cards that died are in their owners' graveyards -- each in their own.
#[test]
fn what_it_kills_goes_to_its_own_owners_graveyard() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[1].graveyard.clear();
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    // Theirs, but under your control: the card goes home to its owner.
    let mut stolen = creature(120_000, cards::SAVANNAH_LIONS, PlayerId::Two);
    stolen.controller = PlayerId::One;
    game.battlefield.push(stolen);
    game.put_onto_battlefield(PlayerId::Two, cards::GRAVE_TITAN)
        .expect("cataloged");
    drain_pending(&mut game);
    assert_eq!(
        game.battlefield.len(),
        4,
        "the Titan brought two Zombies with it",
    );

    wrath(&mut game);

    assert!(game.battlefield.is_empty(), "nothing survived");
    assert_eq!(
        game.players[1]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::SAVANNAH_LIONS, cards::GRAVE_TITAN],
        "both cards are in their owner's graveyard, whoever controlled them, \
         and the two Zombie tokens left nothing behind at all",
    );
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::WRATH_OF_GOD],
        "and the only thing in yours is the spell you cast",
    );
}
