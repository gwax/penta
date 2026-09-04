//! A seeded game is the same game every time it is played.
//!
//! Its own target because everything downstream leans on it -- replay,
//! checkpoint reconstruction, the simulation fingerprint, bot versioning --
//! and because the way it broke was invisible to every other tier.

use penta::{Format, Game, Policy, RandomPolicy, poc};

/// Premodern Stasis against Replenish on this seed reaches a turn-12 upkeep
/// where Stasis wants one blue and the board offers it both with and without
/// a point of life.
const SEED: u64 = 102_894;

/// The engine had a payment planner that kept its dynamic-programming states
/// in a `HashMap`. Two plans can reach the same payment capacity at the same
/// rank -- tapping either of two lands that make the same colour is the
/// ordinary case -- and both the retention test and the final `min_by` keep
/// whichever they meet first, so the answer followed hash iteration order.
/// That order is not stable even within a process: `RandomState` takes a fresh
/// key per map. This exact position replayed twice would tap a painland once
/// and a basic the next time, and end a point of life apart.
///
/// Everything downstream assumes this does not happen -- replay, checkpoint
/// reconstruction, the simulation fingerprint, bot versioning -- so it is
/// worth a cheap guard in the normal tier rather than waiting for a nightly
/// sweep to flake.
#[test]
fn a_seeded_game_replays_identically_within_one_process() {
    let catalog = poc::catalog().unwrap();
    // That turn-12 upkeep is the position the hashed planner could not answer
    // the same way twice.
    let play = || {
        let [first, second] = ["Stasis", "Replenish"].map(|name| {
            penta::protocol::deck_by_name_for_format(Format::Premodern, name)
                .unwrap_or_else(|| panic!("{name} is a registered Premodern deck"))
        });
        let mut game =
            Game::new_with_format(Format::Premodern, catalog.clone(), [first, second], SEED)
                .unwrap();
        let mut policies = [
            RandomPolicy::new(SEED ^ 0xa1a1),
            RandomPolicy::new(SEED ^ 0xb2b2),
        ];
        let mut trace = Vec::new();
        for _ in 0..300 {
            let Some(player) = game.decision_player() else {
                break;
            };
            let observation = game.observe(player);
            let Some(action) = policies[player.index()].choose_action(&observation) else {
                break;
            };
            trace.push(format!(
                "{}{:?}{:?}",
                observation.turn, observation.step, observation.life_totals
            ));
            if game.apply(player, action).is_err() {
                break;
            }
        }
        trace
    };

    // Several replays rather than two. A hashed planner does not disagree with
    // itself on every pair -- two maps can happen to iterate alike -- so a
    // single comparison caught the original defect only about half the time.
    // Each replay is a few hundredths of a second.
    let first = play();
    assert!(
        first.len() > 200,
        "the replay only reached {} actions, too few to cross the position",
        first.len()
    );
    for attempt in 1..6 {
        let again = play();
        let divergence = first.iter().zip(&again).position(|(l, r)| l != r);
        assert_eq!(
            divergence, None,
            "replay {attempt} of the same seeded game diverged from the first \
             at action {divergence:?}",
        );
        assert_eq!(
            again.len(),
            first.len(),
            "replay {attempt} ran a different length"
        );
    }
}
