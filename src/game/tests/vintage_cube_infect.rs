//! Blightsteel Colossus: infect, and a card that will not stay dead.

use super::*;

/// Infect damage to a player is poison rather than life.
#[test]
fn infect_damage_to_a_player_is_poison() {
    let mut game = ready_game();
    game.battlefield.clear();
    let colossus = creature(95_000, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::One);
    let colossus_id = colossus.card.id;
    game.battlefield.push(colossus);
    let life = game.players[1].life;

    game.damage_target_from_kind(
        Some(colossus_id),
        Some(Target::Player(PlayerId::Two)),
        11,
        true,
    );
    drain_pending(&mut game);

    assert_eq!(game.players[1].life, life, "no life is lost");
    assert_eq!(
        game.players[1].counters.count(CounterKind::Poison),
        11,
        "eleven poison instead, which is past the ten it takes to lose",
    );
    game.check_state_based_actions();
    assert_eq!(
        game.result(),
        Some(GameResult::Winner {
            winner: PlayerId::One,
            reason: WinReason::OpponentPoisoned,
        }),
        "and the tenth counter is a state-based loss",
    );
}

/// Infect changes what damage does to creatures and players, not to
/// planeswalkers: those still lose that much loyalty.
#[test]
fn infect_damage_to_a_planeswalker_is_still_loyalty() {
    let mut game = ready_game();
    game.battlefield.clear();
    let colossus = creature(95_040, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::One);
    let colossus_id = colossus.card.id;
    game.battlefield.push(colossus);
    let jace = game
        .put_onto_battlefield(PlayerId::Two, cards::JACE_MEMORY_ADEPT)
        .expect("cataloged");
    drain_pending(&mut game);
    let starting = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == jace)
        .expect("he is there")
        .counters(CounterKind::Loyalty);

    game.damage_target_from_kind(Some(colossus_id), Some(Target::Permanent(jace)), 2, true);
    drain_pending(&mut game);

    let walker = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == jace)
        .expect("five loyalty survives two");
    assert_eq!(
        walker.counters(CounterKind::Loyalty),
        starting - 2,
        "the loyalty is what the damage removed",
    );
    assert_eq!(
        walker.counters(CounterKind::MinusOneMinusOne),
        0,
        "and infect put no counters on him",
    );
}

/// Infect damage to a creature is -1/-1 counters rather than damage marks.
#[test]
fn infect_damage_to_a_creature_is_counters() {
    let mut game = ready_game();
    game.battlefield.clear();
    let colossus = creature(95_010, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::One);
    let colossus_id = colossus.card.id;
    game.battlefield.push(colossus);
    let angel = creature(95_011, cards::SERRA_ANGEL, PlayerId::Two);
    let angel_id = angel.card.id;
    game.battlefield.push(angel);

    game.damage_target_from_kind(
        Some(colossus_id),
        Some(Target::Permanent(angel_id)),
        2,
        true,
    );
    drain_pending(&mut game);

    let angel = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == angel_id)
        .expect("a 4/4 survives two");
    assert_eq!(angel.counters(CounterKind::MinusOneMinusOne), 2);
    assert_eq!(angel.damage, 0, "and takes no damage at all");
    assert_eq!(game.power(angel), Some(2), "it is permanently smaller");
    assert_eq!(game.toughness(angel), Some(2));
}

/// Destroying it puts it back in the library rather than the graveyard, and
/// nothing is left behind to reanimate.
#[test]
fn the_colossus_shuffles_itself_back_instead_of_dying() {
    let mut game = ready_game();
    game.battlefield.clear();
    let colossus = creature(95_020, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::One);
    let colossus_id = colossus.card.id;
    game.battlefield.push(colossus);
    game.players[0].library.clear();
    let before = game.players[0].library.len();

    // Indestructible stops destruction, so this is the sacrifice route --
    // which the replacement answers all the same.
    game.move_permanents_to_graveyard(&[colossus_id]);
    drain_pending(&mut game);

    assert!(
        game.players[0].graveyard.is_empty(),
        "it never reaches the graveyard",
    );
    assert_eq!(
        game.players[0].library.len(),
        before + 1,
        "and goes back into the library instead",
    );
    assert!(
        game.players[0]
            .library
            .iter()
            .any(|card| card.definition == cards::BLIGHTSTEEL_COLOSSUS),
        "as itself",
    );
}

/// "From anywhere" means from anywhere: a discarded Colossus goes back too,
/// and it is shuffled in rather than left on top.
#[test]
fn a_discarded_colossus_goes_back_and_is_shuffled_in() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].library.clear();
    for id in 95_040..95_060 {
        game.players[0]
            .library
            .push(card(id, cards::GRIZZLY_BEARS, PlayerId::One));
    }
    let colossus = card(95_030, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::One);
    let colossus_id = colossus.id;
    game.players[0].hand.push(colossus);

    // Discarded from hand rather than dying, which is a graveyard move from
    // a different zone entirely.
    game.discard_cards(PlayerId::One, &[colossus_id]);
    drain_pending(&mut game);

    assert!(
        game.players[0].graveyard.is_empty(),
        "a discard does not put it in the graveyard either",
    );
    let position = game.players[0]
        .library
        .iter()
        .position(|card| card.definition == cards::BLIGHTSTEEL_COLOSSUS)
        .expect("it goes back into the library from hand as well");
    assert_ne!(
        position,
        game.players[0].library.len() - 1,
        "shuffled in rather than left on top, where it would just be redrawn",
    );
}

/// "The -1/-1 counters remain on the creature indefinitely. They're not
/// removed ... at end of turn." Ordinary damage wears off in the cleanup
/// step; what infect does instead is the reason it is worth the drawback.
#[test]
fn the_counters_outlast_the_turn_that_marked_damage_does_not() {
    let mut game = ready_game();
    game.battlefield.clear();
    let colossus = creature(95_060, cards::BLIGHTSTEEL_COLOSSUS, PlayerId::One);
    let colossus_id = colossus.card.id;
    game.battlefield.push(colossus);
    let bears = creature(95_061, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let poisoned = creature(95_062, cards::SERRA_ANGEL, PlayerId::Two);
    let poisoned_id = poisoned.card.id;
    game.battlefield.push(poisoned);
    let bruised = creature(95_063, cards::SERRA_ANGEL, PlayerId::Two);
    let bruised_id = bruised.card.id;
    game.battlefield.push(bruised);

    // One Angel takes two from the Colossus, the other two from the Bears.
    game.damage_target_from_kind(
        Some(colossus_id),
        Some(Target::Permanent(poisoned_id)),
        2,
        true,
    );
    game.damage_target_from_kind(
        Some(bears_id),
        Some(Target::Permanent(bruised_id)),
        2,
        false,
    );
    drain_pending(&mut game);
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == bruised_id)
            .expect("it survived")
            .damage,
        2,
        "the ordinary two is marked damage",
    );

    game.finish_cleanup();

    let poisoned = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == poisoned_id)
        .expect("still a 2/2");
    assert_eq!(
        poisoned.counters(CounterKind::MinusOneMinusOne),
        2,
        "the counters are still there a turn later",
    );
    assert_eq!(game.toughness(poisoned), Some(2), "and still shrink it");
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == bruised_id)
            .expect("still a 4/4")
            .damage,
        0,
        "while the marked damage is gone, which is the whole difference",
    );
}
