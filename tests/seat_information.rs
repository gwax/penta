//! A seat gets what `Game::events_for*` decides it gets. The raw log is a
//! different thing: it opens with the shuffle seed, and decklists are public,
//! so a seat holding it can rebuild both libraries -- the opponent's hand and
//! every draw either player will make.
//!
//! The browser client is also the engine today, so nothing stops it reading
//! the raw log by hand. That is exactly the habit that stops being safe the
//! moment the engine moves server-side, and a reviewer cannot see it in a
//! diff. So it is a build failure instead.

use std::fs;
use std::path::PathBuf;

/// Files that stand in for a seat: they hold a view, not the engine.
const SEAT_SIDE: [&str; 1] = ["wasm/src/lib.rs"];

/// Everything the seat-facing client does to the engine goes through the
/// session, so the day a remote one appears there is a list of methods to
/// implement rather than a hunt through 3,000 lines. Test code may still
/// reach the engine to build a position; that is what the `#[cfg(test)]`
/// accessors are for, and this only reads the code above the test module.
#[test]
fn a_seat_holds_a_session_rather_than_the_engine() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("wasm/src/lib.rs")).expect("readable");
    let seat_side = source
        .split_once("mod tests {")
        .map_or(source.as_str(), |(before, _)| before);
    let offenders: Vec<_> = seat_side
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains("self.game"))
        .map(|(number, line)| format!("wasm/src/lib.rs:{}: {}", number + 1, line.trim()))
        .collect();
    assert!(
        offenders.is_empty(),
        "these reach past the session to the engine. Add the method to \
         LocalSession instead, so a remote session has to answer for it \
         too:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn a_seat_never_reads_the_raw_event_log() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut offenders = Vec::new();
    for relative in SEAT_SIDE {
        let source = fs::read_to_string(root.join(relative)).expect("a seat file is readable");
        for (number, line) in source.lines().enumerate() {
            if line.contains(".events()") {
                offenders.push(format!("{relative}:{}: {}", number + 1, line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "these read the unprojected log. Use `events_for` for the whole \
         stream, or `event_cursor` and `events_for_since` for what one action \
         caused:\n  {}",
        offenders.join("\n  ")
    );
}
