//! Four old anthems switched on and off by the board.
//!
//! Each is "as long as", so the interesting half is that it lapses again.
//! Angelic Voices reads an *absence*, the Beasts read the other side of the
//! table, and the two Goblin Auras read what they are sitting on.

use super::*;
use crate::ImplementationStatus;

fn ready() -> Game {
    let mut game = ready_game();
    game.turn = 5;
    game.turns_started[PlayerId::One.index()] = 5;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.battlefield.clear();
    game
}

fn stats(game: &Game, id: GameObjectId) -> (Option<i16>, Option<i16>) {
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("still there");
    (game.power(permanent), game.toughness(permanent))
}

/// One nonwhite, nonartifact creature switches the whole anthem off.
#[test]
fn angelic_voices_wants_a_clean_board() {
    let mut game = ready();
    game.battlefield
        .push(creature(10_000, cards::ANGELIC_VOICES, PlayerId::One));
    let angel = creature(10_100, cards::SERRA_ANGEL, PlayerId::One);
    let angel_id = angel.card.id;
    game.battlefield.push(angel);

    assert_eq!(stats(&game, angel_id), (Some(5), Some(5)), "white, so fine");

    // An artifact creature is exempt too, being nonwhite but an artifact.
    let thopter = creature(10_101, cards::ORNITHOPTER, PlayerId::One);
    let thopter_id = thopter.card.id;
    game.battlefield.push(thopter);
    assert_eq!(
        stats(&game, angel_id),
        (Some(5), Some(5)),
        "an artifact creature does not break it",
    );
    assert_eq!(
        stats(&game, thopter_id),
        (Some(1), Some(3)),
        "and the 0/2 is pumped along with the rest",
    );

    let bear = creature(10_102, cards::GRIZZLY_BEARS, PlayerId::One);
    let bear_id = bear.card.id;
    game.battlefield.push(bear);
    assert_eq!(
        stats(&game, angel_id),
        (Some(4), Some(4)),
        "one green Bear and the whole anthem is gone",
    );

    game.battlefield
        .retain(|permanent| permanent.card.id != bear_id);
    assert_eq!(
        stats(&game, angel_id),
        (Some(5), Some(5)),
        "and it comes back when the Bear leaves",
    );
}

/// The Beasts read the opponent's board, and read "nontoken" strictly.
#[test]
fn the_beasts_want_a_nontoken_white_permanent_opposite() {
    let mut game = ready();
    let beasts = creature(10_000, cards::BEASTS_OF_BOGARDAN, PlayerId::One);
    let beasts_id = beasts.card.id;
    game.battlefield.push(beasts);

    assert_eq!(stats(&game, beasts_id), (Some(3), Some(3)));

    // A white creature of your own is on the wrong side of the table.
    game.battlefield
        .push(creature(10_100, cards::SERRA_ANGEL, PlayerId::One));
    assert_eq!(
        stats(&game, beasts_id),
        (Some(3), Some(3)),
        "yours, not theirs"
    );

    // A white token opposite is white but not nontoken.
    game.battlefield.push(creature(
        10_101,
        cards::HUMAN_TOKEN_1_1_WHITE,
        PlayerId::Two,
    ));
    assert_eq!(
        stats(&game, beasts_id),
        (Some(3), Some(3)),
        "a token is not a nontoken permanent",
    );

    game.battlefield
        .push(creature(10_102, cards::SERRA_ANGEL, PlayerId::Two));
    assert_eq!(
        stats(&game, beasts_id),
        (Some(4), Some(4)),
        "a white card opposite does it",
    );
}

/// Both Goblin Auras read the land under them, and reach every Goblin.
#[test]
fn the_goblin_auras_read_the_land_they_sit_on() {
    let mut game = ready();
    game.put_onto_battlefield(PlayerId::One, cards::MOUNTAIN)
        .expect("cataloged");
    let land = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::MOUNTAIN)
        .expect("it is there")
        .card
        .id;

    let mut caves = creature(10_000, cards::GOBLIN_CAVES, PlayerId::One);
    caves.attached_to = Some(land);
    game.battlefield.push(caves);
    let mut shrine = creature(10_001, cards::GOBLIN_SHRINE, PlayerId::One);
    shrine.attached_to = Some(land);
    game.battlefield.push(shrine);

    // "Goblin creatures", with no controller relation, so an opposing Goblin
    // is pumped too.
    let theirs = creature(10_100, cards::GOBLINS_OF_THE_FLARG, PlayerId::Two);
    let theirs_id = theirs.card.id;
    game.battlefield.push(theirs);
    assert_eq!(
        stats(&game, theirs_id),
        (Some(2), Some(3)),
        "a 1/1 with +1/+0 and +0/+2, whoever controls it",
    );

    // Move both onto a Forest and the condition fails.
    game.put_onto_battlefield(PlayerId::One, cards::FOREST)
        .expect("cataloged");
    let forest = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::FOREST)
        .expect("it is there")
        .card
        .id;
    for permanent in &mut game.battlefield {
        if permanent.attached_to == Some(land) {
            permanent.attached_to = Some(forest);
        }
    }
    assert_eq!(
        stats(&game, theirs_id),
        (Some(1), Some(1)),
        "a Forest is not a basic Mountain",
    );
}

#[test]
fn all_four_report_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [
        cards::ANGELIC_VOICES,
        cards::BEASTS_OF_BOGARDAN,
        cards::GOBLIN_CAVES,
        cards::GOBLIN_SHRINE,
    ] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}
