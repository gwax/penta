# Changelog

Two numbers matter to a bot, and they move independently:

- **Protocol version** (`penta.protocol_version()`, `penta_protocol_version()`)
  covers the JSON shapes and the action space they describe. It bumps when a
  bot written against the old number could misread the new output.
- **Engine version** (`penta.engine_version()`, the crate version) covers
  rules behavior. It bumps for anything that changes what a policy sees,
  including rules fixes that leave the shapes alone.

Pin both alongside trained weights. Until 1.0 the engine version bumps its
minor for breaking changes, per Cargo's 0.x convention.

## 0.2.0 — protocol 1

### Changed

- **Conceding is no longer a bot action.** It appeared in `legalActions` in
  every state, always at index 0, and is strictly dominated for a bot —
  resigning only loses a game that playing on might win. A bot that picked
  blindly or explored uniformly resigned on turn one, which made the
  `random` baseline meaningless to measure against. It is gone from the
  bot's list entirely, so **every index in `legalActions` shifts down by
  one**; a bot that hardcoded indices needs revisiting, one that reads the
  `type` tags does not. Humans still concede in the browser client, which
  reads the engine's own action list.

### Added

- Local matches between the built-in policies via `penta-match`.
- CI on every push and pull request, running the same two scripts as local
  development.
- `rust-toolchain.toml` pins the Rust version, components, and wasm target,
  so contributors, maintainers, and CI share one compiler.

## 0.1.0 — protocol 0

First release of the bot-facing surfaces: the `penta::protocol` module and
its canonical JSON, the Python bindings, the C ABI, self-play through an
external opponent, and [BOTS.md](BOTS.md).
