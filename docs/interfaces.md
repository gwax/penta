# Engine interfaces

Penta exposes one authoritative state machine through several adapters. This
document describes the boundaries between them. Bot authors should use the
task-oriented [bot guide](bots.md), which also documents the JSON observation
and action vocabulary.

## Core `Game` API

`Game` owns authoritative state. Consumers do not mutate zones, life, mana,
priority, or the stack directly. They ask for `legal_actions(player)` and
submit one of those values to `apply(player, action)`. `apply` checks legality
again so a stale decision fails without changing state.

For a generic `DecisionObservation`, `legal_actions` returns a compact
`ChooseDecision` marker. Callers select option IDs from the observation and use
`is_legal_action` and `apply` for validation without expanding every possible
combination.

A runner asks `decision_player()` who must act, observes that player, and
submits one of the observation's legal actions:

```rust
while let Some(player) = game.decision_player() {
    let observation = game.observe(player);
    let action = bots[player.index()].choose_action(&observation);
    game.apply(player, action)?;
}
```

The decision player is normally the player with priority, but differs during
mulligans, blocker declaration, restricted untaps, cleanup discards, and
triggered or combat-damage choices.

## Observations and events

`PlayerObservation` is the hidden-information-safe input for a player or bot.
It contains that player's hand and only counts for an opponent's unrevealed
hidden zones. Rules-driven disclosures can add the known, possibly stale
`lastSeenHand` snapshot documented in the [bot guide](bots.md); they never
expose cards the player has not learned. `GameEvent` is an omniscient debugging
and replay stream; it must not be used as a player observation.

The engine enumerates legal actions rather than asking consumers to construct
partially legal commands. Complex multi-selection decisions expose bounded
options and are submitted through the same checked state-machine boundary.

## Consumer layers

- `Game` is the native Rust rules state machine.
- `protocol::BotGame` presents stable JSON observations and indexed actions for
  bot and binding consumers.
- `bindings/penta-py` and `bindings/penta-ffi` expose that same protocol to
  Python, C, C++, and other FFI-capable languages.
- `wasm/` exposes the engine to the browser. The web client selects from engine
  actions and decisions rather than reconstructing rules in TypeScript.

Protocol shapes and rules behavior are versioned independently. Query
`protocol_version()` and `engine_version()` through the relevant binding and
pin both alongside trained policies or recorded integrations. Release history
and migration notes live in the [changelog](../CHANGELOG.md).
