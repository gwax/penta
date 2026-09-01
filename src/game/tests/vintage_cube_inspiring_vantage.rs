//! Inspiring Vantage, and whose lands its clause counts.
//!
//! The cycle's boundary is checked as a family from one side of the table.
//! What this adds is the other side: "you control two or fewer other lands"
//! is read from the arriving land's own controller, not from whoever is
//! taking the turn.

use super::*;

fn tapped_on_arrival(mine: usize, theirs: usize, arriving: PlayerId) -> bool {
    let mut game = ready_game();
    game.battlefield.clear();
    for (player, count) in [(PlayerId::One, mine), (PlayerId::Two, theirs)] {
        for index in 0..count {
            game.battlefield.push(creature(
                101_500
                    + u32::try_from(index).expect("a small board")
                    + 20 * u32::from(player == PlayerId::Two),
                cards::MOUNTAIN,
                player,
            ));
        }
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    let vantage = game
        .put_onto_battlefield(arriving, cards::INSPIRING_VANTAGE)
        .expect("cataloged");
    drain_pending(&mut game);
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == vantage)
        .expect("it entered")
        .tapped
}

/// Their Vantage reads their own board: arriving on your turn, behind three
/// of your Mountains and none of theirs, it is still untapped.
#[test]
fn the_clause_counts_the_arriving_lands_own_side() {
    assert!(
        !tapped_on_arrival(5, 0, PlayerId::Two),
        "five lands across the table are not two or fewer of theirs",
    );
    assert!(
        tapped_on_arrival(0, 3, PlayerId::Two),
        "and three of their own is what taps it, on anyone's turn",
    );
}

/// The same boundary from your side, for the record: the fourth land you
/// control is the one that arrives tapped.
#[test]
fn your_own_fourth_land_arrives_tapped() {
    assert!(!tapped_on_arrival(2, 5, PlayerId::One));
    assert!(tapped_on_arrival(3, 0, PlayerId::One));
}

/// Both of its colours, and only those two.
#[test]
fn it_taps_for_red_and_white() {
    let mut game = ready_game();
    game.battlefield.clear();
    let vantage = game
        .put_onto_battlefield(PlayerId::One, cards::INSPIRING_VANTAGE)
        .expect("cataloged");
    drain_pending(&mut game);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    assert_eq!(
        game.legal_actions(PlayerId::One)
            .into_iter()
            .filter_map(|action| match action {
                Action::ActivateManaAbility { source, color, .. } if source == vantage =>
                    Some(color),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![ManaColor::Red, ManaColor::White],
        "red or white, and nothing colourless",
    );
}
