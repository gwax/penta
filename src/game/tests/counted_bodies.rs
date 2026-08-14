//! Creatures whose printed power and toughness are a battlefield count.
//!
//! These are declared as a zero-or-one body plus a static counted bonus, which
//! is what the token vocabulary already did. That is right on the battlefield
//! and is not a characteristic-defining ability, so each card names the gap;
//! the last test here pins the difference rather than leaving it prose.

use super::*;
use crate::ImplementationStatus;

fn body(game: &Game, id: GameObjectId) -> (i16, i16) {
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("the creature is on the battlefield");
    (
        game.power(permanent).expect("it is a creature"),
        game.toughness(permanent).expect("it is a creature"),
    )
}

#[test]
fn keldon_warlord_counts_your_non_wall_creatures_and_recounts_as_they_change() {
    let mut game = ready_game();
    let warlord = creature(10_000, cards::KELDON_WARLORD, PlayerId::One);
    let warlord_id = warlord.card.id;
    game.battlefield.push(warlord);
    assert_eq!(body(&game, warlord_id), (1, 1), "it counts itself");

    game.battlefield
        .push(creature(10_001, cards::SAVANNAH_LIONS, PlayerId::One));
    assert_eq!(body(&game, warlord_id), (2, 2));

    // A Wall is a creature and still does not count.
    game.battlefield
        .push(creature(10_002, cards::WALL_OF_WOOD, PlayerId::One));
    assert_eq!(body(&game, warlord_id), (2, 2), "Walls are excluded");

    // Neither does an opposing creature.
    game.battlefield
        .push(creature(10_003, cards::SAVANNAH_LIONS, PlayerId::Two));
    assert_eq!(body(&game, warlord_id), (2, 2), "and so is the opponent's");
}

/// Plague Rats counts every Plague Rats anywhere on the battlefield, which is
/// the one query in this group that is not controller-scoped.
#[test]
fn plague_rats_count_each_other_across_both_sides() {
    let mut game = ready_game();
    let rats = creature(10_000, cards::PLAGUE_RATS, PlayerId::One);
    let rats_id = rats.card.id;
    game.battlefield.push(rats);
    assert_eq!(body(&game, rats_id), (1, 1));

    game.battlefield
        .push(creature(10_001, cards::PLAGUE_RATS, PlayerId::Two));
    assert_eq!(
        body(&game, rats_id),
        (2, 2),
        "an opposing Rat counts for yours too"
    );

    game.battlefield
        .push(creature(10_002, cards::SAVANNAH_LIONS, PlayerId::One));
    assert_eq!(body(&game, rats_id), (2, 2), "and nothing else does");
}

/// Gaea's Avenger prints "1 plus", which is carried by the body rather than
/// the bonus, so it is a 1/1 with no artifacts opposing it.
#[test]
fn gaeas_avenger_starts_at_one_and_grows_with_opposing_artifacts() {
    let mut game = ready_game();
    let avenger = creature(10_000, cards::GAEAS_AVENGER, PlayerId::One);
    let avenger_id = avenger.card.id;
    game.battlefield.push(avenger);
    assert_eq!(body(&game, avenger_id), (1, 1));

    game.battlefield
        .push(creature(10_001, cards::SOL_RING, PlayerId::Two));
    assert_eq!(body(&game, avenger_id), (2, 2));

    // Your own artifacts are not what it answers.
    game.battlefield
        .push(creature(10_002, cards::SOL_RING, PlayerId::One));
    assert_eq!(body(&game, avenger_id), (2, 2));
}

/// A creature whose count is zero is a 0/0 and dies to state-based actions,
/// which is the printed behaviour rather than an accident of the encoding.
#[test]
fn a_counted_body_with_nothing_to_count_dies() {
    let mut game = ready_game();
    let dakkon = creature(10_000, cards::DAKKON_BLACKBLADE, PlayerId::One);
    let dakkon_id = dakkon.card.id;
    game.battlefield.push(dakkon);

    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == dakkon_id),
        "no lands means a 0/0"
    );
}

/// The declared gap, pinned: the bonus is a battlefield continuous effect, so
/// a card of the same identity outside the battlefield carries only what is
/// printed on it. A real characteristic-defining ability would not.
#[test]
fn the_counted_body_is_absent_outside_the_battlefield() {
    let catalog = poc::catalog().expect("catalog builds");
    let warlord = catalog
        .get(cards::KELDON_WARLORD)
        .expect("the card is cataloged");
    let printed = warlord
        .rules
        .creature_stats()
        .expect("Keldon Warlord is a creature");
    assert_eq!(
        (printed.power, printed.toughness),
        (0, 0),
        "the printed body is what any zone but the battlefield sees"
    );
}

#[test]
fn every_counted_body_reports_its_declared_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [
        cards::PLAGUE_RATS,
        cards::KELDON_WARLORD,
        cards::GAEAS_AVENGER,
        cards::DAKKON_BLACKBLADE,
    ] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Partial,
            "{} is declared with its characteristic-defining gap named",
            card.name,
        );
    }
}
