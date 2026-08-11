//! Rebuilds a bug report's game natively, where it can be debugged.
//!
//! Reads a bug report (or a bare replay) as JSON on stdin, replays it through
//! the same `WebGame` facade the browser runs, and prints where it landed:
//! the final human-visible snapshot on stdout, progress and failures on
//! stderr. A command that no longer applies stops the replay at its position,
//! which is itself diagnostic -- it names the moment this engine diverged
//! from the one the bug was reported against.
//!
//!     curl -s http://localhost:<port>/_bugs/<id> | cargo run -p penta-wasm --example replay_bug

use std::io::Read;

use penta_wasm::WebGame;

fn main() {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("a bug report on stdin");
    let value: serde_json::Value = serde_json::from_str(&input).expect("stdin is JSON");
    // A whole bug report wraps the replay; a bare replay is also accepted.
    let replay = if value["replay"].is_object() {
        if let Some(description) = value["description"].as_str() {
            eprintln!("bug: {description}");
        }
        value["replay"].clone()
    } else {
        value
    };
    let commands = replay["commands"].as_array().map_or(0, Vec::len);
    eprintln!("replaying {commands} commands…");
    let game = WebGame::from_replay_json(&replay.to_string())
        .unwrap_or_else(|_| panic!("replay failed; the reason is on stderr above"));
    println!("{}", game.state_json());
    eprintln!("replay complete: the snapshot above is the reported board");
}
