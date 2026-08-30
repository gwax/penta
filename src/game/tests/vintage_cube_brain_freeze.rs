//! Brain Freeze: three cards a copy, and each copy chooses its own victim.

use super::*;

/// Player One with `earlier` cantrips already cast this turn and a Brain
/// Freeze in hand, aimed at `target` and left on the stack.
fn cast_freeze(game: &mut Game, earlier: u32, target: Target) {
    for index in 0..earlier {
        let opt = card(82_000 + index, cards::OPT, PlayerId::One);
        let opt_id = opt.id;
        game.players[0].hand.push(opt);
        game.players[0].mana_pool.blue = 1;
        game.priority = PlayerId::One;
        game.apply(
            PlayerId::One,
            cast_action(opt_id, Vec::new(), Vec::new(), 0),
        )
        .expect("a cantrip is castable");
        drain_pending(game);
    }
    let freeze = card(82_100, cards::BRAIN_FREEZE, PlayerId::One);
    let freeze_id = freeze.id;
    game.players[0].hand.push(freeze);
    let pool = &mut game.players[0].mana_pool;
    pool.blue = 1;
    pool.colorless = 1;
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(freeze_id, vec![target], Vec::new(), 0),
    )
    .expect("two mana buys a Freeze");
}

/// Answers each retarget offer with the option whose label starts with
/// `wanted`, then lets the stack empty.
fn settle_choosing(game: &mut Game, wanted: &str) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let option = decision
                .options
                .iter()
                .find(|option| option.label.starts_with(wanted))
                .unwrap_or_else(|| {
                    panic!(
                        "{wanted} is offered: {:?}",
                        decision
                            .options
                            .iter()
                            .map(|option| option.label.clone())
                            .collect::<Vec<_>>()
                    )
                })
                .id;
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: vec![option],
                },
            )
            .expect("the decision accepts what it offered");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            return;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            return;
        }
    }
}

fn library_sizes(game: &Game) -> (usize, usize) {
    (game.players[0].library.len(), game.players[1].library.len())
}

/// "You may choose new targets for any of the copies. You can make different
/// choices for each copy." One cantrip first makes one copy, and pointing it
/// back at yourself splits the milling between the two libraries.
#[test]
fn a_copy_may_mill_the_other_player() {
    let mut game = ready_game();
    cast_freeze(&mut game, 1, Target::Player(PlayerId::Two));
    let (mine_before, theirs_before) = library_sizes(&game);

    settle_choosing(&mut game, "Copy with targets you");

    let (mine_after, theirs_after) = library_sizes(&game);
    assert_eq!(
        theirs_before - theirs_after,
        3,
        "the original milled the player it named",
    );
    assert_eq!(
        mine_before - mine_after,
        3,
        "and the copy milled the one it was pointed at instead",
    );
}

/// Kept as it was, both the original and its copy read the same target: six
/// cards off one library rather than three off each.
#[test]
fn a_kept_copy_mills_the_same_player_again() {
    let mut game = ready_game();
    cast_freeze(&mut game, 1, Target::Player(PlayerId::Two));
    let (mine_before, theirs_before) = library_sizes(&game);

    settle_choosing(&mut game, "Keep original targets");

    let (mine_after, theirs_after) = library_sizes(&game);
    assert_eq!(theirs_before - theirs_after, 6, "three twice over");
    assert_eq!(mine_before, mine_after, "and none of your own");
}
