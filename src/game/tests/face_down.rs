//! The body a face-down permanent presents, and who may see the card under it.

use super::*;

/// A face-down permanent is a 2/2 creature with no name whatever the card
/// under it says, and the card under it is still what the game holds.
#[test]
fn a_face_down_permanent_is_a_nameless_two_two() {
    let mut game = ready_game();
    // Serra Angel is a 4/4 flier with two keywords face up.
    let mut angel = creature(10_000, cards::SERRA_ANGEL, PlayerId::One);
    angel.face_down = true;
    game.battlefield.push(angel);

    let permanent = &game.battlefield[0];
    let stats = game
        .creature_stats(permanent)
        .expect("a face-down permanent is a creature");
    assert_eq!(
        (stats.power, stats.toughness),
        (2, 2),
        "the body, not the card",
    );
    assert!(
        !game.has_flying(permanent),
        "and none of the card's abilities",
    );
    assert_eq!(
        game.effective_permanent_name(permanent),
        None,
        "a face-down permanent has no name",
    );
    assert_eq!(
        permanent.card.definition,
        cards::SERRA_ANGEL,
        "the physical card is unchanged underneath",
    );
    assert!(
        !game.is_token(permanent.card.definition),
        "and it is not a token",
    );
}

/// Its controller may look at it. Nobody else may.
#[test]
fn only_its_controller_sees_what_it_is() {
    let mut game = ready_game();
    let mut angel = creature(10_000, cards::SERRA_ANGEL, PlayerId::One);
    angel.face_down = true;
    game.battlefield.push(angel);

    let mine = game.observe(PlayerId::One);
    let theirs = game.observe(PlayerId::Two);
    assert_eq!(
        mine.battlefield[0].definition,
        cards::SERRA_ANGEL,
        "its controller knows what they played",
    );
    assert_eq!(
        theirs.battlefield[0].definition,
        cards::FACE_DOWN_CREATURE,
        "and the opponent sees only a body",
    );
    assert!(
        mine.battlefield[0].face_down && theirs.battlefield[0].face_down,
        "both seats see that it is face down",
    );
    assert_eq!(
        (theirs.battlefield[0].power, theirs.battlefield[0].toughness),
        (Some(2), Some(2)),
        "which is a 2/2 from either side",
    );
}
