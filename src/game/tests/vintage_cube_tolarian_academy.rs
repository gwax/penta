//! Tolarian Academy: a land that pays for the artifacts it made you play,
//! and a legend you may only have one of.

use super::*;

/// Player One with the Academy out and `artifacts` beside it, all of them
/// theirs unless `theirs` says otherwise.
fn staged(mine: &[CardDefinitionId], theirs: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let academy = game
        .put_onto_battlefield(PlayerId::One, cards::TOLARIAN_ACADEMY)
        .expect("cataloged");
    for definition in mine {
        game.put_onto_battlefield(PlayerId::One, *definition)
            .expect("cataloged");
    }
    for definition in theirs {
        game.put_onto_battlefield(PlayerId::Two, *definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, academy)
}

/// Taps the Academy for blue and reports how much came out.
fn tap_for_blue(game: &mut Game, academy: GameObjectId) -> u16 {
    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: academy,
            ability: mana_ability_for(game, academy, ManaColor::Blue),
            color: ManaColor::Blue,
            counters_removed: None,
            cost_object: None,
            combination: None,
            triggered_mana: None,
        },
    )
    .expect("the land taps for mana");
    game.players[0].mana_pool.blue
}

/// The count is of artifacts, and an artifact creature is an artifact: a
/// Lotus and an Ornithopter are two, not one and a creature.
#[test]
fn an_artifact_creature_counts_as_an_artifact() {
    let (mut game, academy) = staged(&[cards::BLACK_LOTUS, cards::ORNITHOPTER], &[]);

    assert_eq!(tap_for_blue(&mut game, academy), 2, "both are artifacts");
}

/// "Artifacts you control." Their side of the table is no part of it,
/// however many rocks are sitting on it.
#[test]
fn their_artifacts_are_not_counted() {
    let (mut game, academy) = staged(
        &[cards::BLACK_LOTUS],
        &[cards::MOX_SAPPHIRE, cards::MOX_RUBY, cards::ORNITHOPTER],
    );

    assert_eq!(tap_for_blue(&mut game, academy), 1, "one artifact is yours");
}

/// With no artifacts the ability is still there to activate -- it is the
/// land's only one -- and it taps the Academy for nothing at all.
#[test]
fn no_artifacts_makes_no_mana_and_still_spends_the_tap() {
    let (mut game, academy) = staged(&[], &[]);

    assert_eq!(tap_for_blue(&mut game, academy), 0, "nothing to count");
    assert!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == academy)
            .expect("still there")
            .tapped,
        "and the tap was spent anyway",
    );
}

/// The Academy is a land and not an artifact, so it does not count itself:
/// two artifacts beside it are two, not three.
#[test]
fn it_does_not_count_itself() {
    let (mut game, academy) = staged(&[cards::MOX_SAPPHIRE, cards::MOX_RUBY], &[]);

    assert_eq!(tap_for_blue(&mut game, academy), 2, "the land is no rock");
}

/// It is Legendary: a second one under the same control is put into the
/// graveyard as a state-based action, so the artifacts are only ever counted
/// once.
#[test]
fn a_second_academy_does_not_stay() {
    let (mut game, _academy) = staged(&[cards::BLACK_LOTUS], &[]);
    game.put_onto_battlefield(PlayerId::One, cards::TOLARIAN_ACADEMY)
        .expect("cataloged");
    drain_pending(&mut game);

    game.check_state_based_actions();

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == cards::TOLARIAN_ACADEMY)
            .count(),
        1,
        "the legend rule kept one",
    );
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .filter(|card| card.definition == cards::TOLARIAN_ACADEMY)
            .count(),
        1,
        "and buried the other",
    );
}

/// Their Academy is not yours: the legend rule is per player, and both
/// stand.
#[test]
fn each_player_may_have_their_own() {
    let (mut game, _academy) = staged(&[], &[cards::TOLARIAN_ACADEMY]);

    game.check_state_based_actions();

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.card.definition == cards::TOLARIAN_ACADEMY)
            .count(),
        2,
        "one apiece is legal",
    );
}
