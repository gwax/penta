//! A card's `CardBehavior` stops being reachable the moment its clauses become
//! declarative: `special_behavior()` returns `None`, and every `match` arm
//! keyed on that behavior silently stops matching. Nothing fails to compile and
//! no unit test notices, because the arm is still perfectly valid code for a
//! behavior no card reports any more.
//!
//! That has already cost real behavior. Migrating Mishra's Factory killed two
//! policy rules this way -- one that kept the bot from tapping the Factory for
//! the mana that animates it -- and the only thing that caught it was a slow
//! browser test several commits later.
//!
//! So: a behavior named outside the card definitions has to still be named by a
//! card, or be listed here as deliberately retired. Migrating a card will fail
//! this test until someone looks at what else was reading that key.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Keys no card reports any more, each checked once for readers that had to be
/// rewritten. Add a name here only after confirming that nothing still depends
/// on the arm it feeds.
const RETIRED: [&str; 0] = [];

/// Trees that dispatch on behavior to decide something, as opposed to the card
/// definitions that supply it and the tables that map it back to rules.
///
/// Directories, not files: naming `src/policy.rs` was enough until the policy
/// was split into modules, at which point the guard was reading a 64-line
/// facade and quietly checking nothing.
const READER_TREES: [&str; 2] = ["src/policy", "wasm/src"];

/// Single files worth reading alongside those trees.
const READER_FILES: [&str; 1] = ["src/policy.rs"];

fn behaviors_in(source: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = source;
    while let Some(index) = rest.find("CardBehavior::") {
        rest = &rest[index + "CardBehavior::".len()..];
        let name: String = rest
            .chars()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect();
        if !name.is_empty() {
            found.insert(name);
        }
    }
    found
}

fn rust_files_under(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files_under(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

#[test]
fn every_behavior_a_reader_dispatches_on_is_still_reported_by_a_card() {
    let root = repo_root();

    // `sets/mod.rs` maps every behavior back to its rules, including retired
    // keys, so it cannot be the evidence that a card still reports one.
    let mut definition_files = Vec::new();
    rust_files_under(&root.join("src/card/sets"), &mut definition_files);
    definition_files.retain(|path| !path.ends_with("sets/mod.rs"));

    let mut reported = BTreeSet::new();
    for path in &definition_files {
        let source = fs::read_to_string(path).expect("a card definition file is readable");
        reported.extend(behaviors_in(&source));
    }
    assert!(
        reported.len() > 20,
        "the scan found almost no behaviors, so it is measuring the wrong thing"
    );

    let mut reader_files = Vec::new();
    for tree in READER_TREES {
        rust_files_under(&root.join(tree), &mut reader_files);
    }
    for file in READER_FILES {
        let path = root.join(file);
        if path.exists() {
            reader_files.push(path);
        }
    }

    let retired: BTreeSet<String> = RETIRED.iter().map(|name| (*name).to_string()).collect();
    let mut dead = Vec::new();
    let mut seen_any = false;
    for reader in &reader_files {
        let source = fs::read_to_string(reader).expect("a reader file is readable");
        let relative = reader.strip_prefix(&root).unwrap_or(reader).display();
        for behavior in behaviors_in(&source) {
            seen_any = true;
            if !reported.contains(&behavior) && !retired.contains(&behavior) {
                dead.push(format!("{relative}: CardBehavior::{behavior}"));
            }
        }
    }
    // Without this the guard passes loudest when it has stopped working: move
    // the dispatch somewhere unscanned and every arm looks fine.
    assert!(
        seen_any,
        "no reader mentions CardBehavior at all, so this is scanning the wrong \
         place. Point READER_TREES at wherever the dispatch went."
    );

    assert!(
        dead.is_empty(),
        "these arms can never match, because no card reports the behavior any \
         more. Rewrite each one to read the card's abilities instead, then add \
         the name to RETIRED:\n  {}",
        dead.join("\n  ")
    );
}
