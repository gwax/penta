//! Launches one native in-memory match, or checks an organizer replay export.

use std::process::ExitCode;

use penta_host::{Config, Host, replay_bundle, server};

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut format = None;
    let mut p1_deck = None;
    let mut p2_deck = None;
    let mut match_mode = "first-to-two-wins".to_owned();
    let mut port = 0;
    while let Some(flag) = args.next() {
        if flag == "--help" || flag == "-h" {
            println!(
                "penta-host --format SLUG --p1-deck NAME --p2-deck NAME [--match-mode first-to-two-wins|one-conclusion] [--port 0]\npenta-host replay FILE\nLaunch prints the server URL and organizer secret. All state stays in memory until close.\nUse organizer export to capture a debug bundle; hard termination loses unexported state."
            );
            return Ok(());
        }
        if flag == "replay" {
            let path = args.next().ok_or("replay requires a JSON export path")?;
            if args.next().is_some() || format.is_some() || p1_deck.is_some() || p2_deck.is_some() {
                return Err("replay accepts only a JSON export path".into());
            }
            let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
            let bundle = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
            println!("{}", replay_bundle(&bundle)?);
            return Ok(());
        }
        let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--format" => format = Some(value),
            "--p1-deck" => p1_deck = Some(value),
            "--p2-deck" => p2_deck = Some(value),
            "--match-mode" => match_mode = value,
            "--port" => port = value.parse().map_err(|_| "invalid port")?,
            _ => return Err(format!("unknown option: {flag}")),
        }
    }
    let config = Config {
        format: format.ok_or("--format is required")?,
        p1_deck: p1_deck.ok_or("--p1-deck is required")?,
        p2_deck: p2_deck.ok_or("--p2-deck is required")?,
        match_mode,
    };
    server::serve(Host::new(config)?, port)
}

fn main() -> ExitCode {
    // Engine panic payloads may contain hidden state. The host records the failing
    // command for its organizer and restores the last committed in-memory session.
    std::panic::set_hook(Box::new(|_| {
        eprintln!(
            "penta-host: unexpected failure; inspect the organizer debug export if the host remains available"
        );
    }));
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
