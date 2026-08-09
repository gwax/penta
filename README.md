# penta

[![CI](https://github.com/lacker/penta/actions/workflows/ci.yml/badge.svg)](https://github.com/lacker/penta/actions/workflows/ci.yml)

`penta` is a deterministic, headless simulator for two-player constructed
Magic: The Gathering, built for writing AI bots against.

The current development bot wire contract is protocol 7; the engine crate
remains version 0.6.0 while these changes are unreleased. Old School remains
the default at compatibility entry points; callers select ISD–RTR Standard
explicitly with the `"isd-rtr-standard"` format slug.

**Want to write a bot?** You can drive the engine from Python, C, C++, or
Rust, play against the included bot algorithms, and train against self-play.
The instructions are in [BOTS.md](BOTS.md).

## Formats

The engine currently supports two explicit format profiles:

- **Eternal Central Old School 93/94**: the original card pool, EC banned and
  restricted lists, phase-boundary mana burn, and fifteen powered archetypes.
- **ISD–RTR Standard (final pre-Theros snapshot)**: Innistrad, Dark Ascension,
  Avacyn Restored, Magic 2013, Return to Ravnica, Gatecrash, Dragon's Maze,
  and Magic 2014; no banned or restricted cards; modern mana-pool emptying with
  no mana burn; and the eight decks from SCG Open Atlanta in September 2013.

Both use 20 starting life, 60-card minimum decks, sideboards of up to 15 cards,
and a four-copy limit except for basic lands. The simulator currently uses the
London mulligan for both formats.

The Old School profile includes:

- Alpha, Beta, Unlimited, Collector's Edition, International Collector's
  Edition, Arabian Nights, Antiquities, Revised, Legends, The Dark, Fallen
  Empires, and the three 1994 promotional cards
- the Eternal Central banned and restricted list
- current Magic rules except where Eternal Central explicitly differs,
  notably mana burn

Paper-only policies such as which physical reprints are acceptable have no
meaning in the simulator.

The canonical format reference is [Eternal Central's 93/94 rules][ec-rules].

The selected format is stored on each game. Format-specific construction and
mana rules live in one profile rather than being scattered as global switches,
so more formats can be added without changing existing games.

## Engine principles

- Game state changes only through explicit actions.
- All randomness comes from a recorded seed and a versioned PRNG.
- Cards have stable canonical definition IDs and exact printing IDs; runtime
  rules objects use zone-scoped game-object IDs while private physical-card
  lineage follows the underlying cardboard.
- A player's observation cannot expose an opponent's hidden cards.
- Legal actions are enumerated by the engine.
- The core crate has no UI, network, async runtime, or training dependencies.

See [the engine design notes](docs/engine.md) for state-machine invariants and
extension boundaries.

## Repository layout

- `src/card/` owns the card model, catalog, stable IDs, and corpus. Files under
  `src/card/sets/y<year>/<set>.rs` give each set its own module. Its `CARDS`
  records declare canonical cards whose rules live there, while
  `ADDITIONAL_PRINTINGS` references those definitions for reprints and
  alternate-art variants without duplicating their rules. Ordered ability
  clauses own both their rules text and implementation; aggregate Complete,
  Partial, or MetadataOnly coverage is derived from those clauses, whose
  non-declarative implementations explain any remaining gaps in place. New
  card work starts at that clause boundary: general rules concepts become
  reusable definition vocabulary and card-agnostic engine primitives, while
  bespoke behavior or behavior without a clear reusable shape can use a
  clause-owned custom selector and a card-specific resolver. Direct
  card-identity branches outside that boundary are reserved as a pragmatic
  escape valve for particularly weird or difficult cards.
- `decks/<format>/` contains the built-in decklists as YAML mappings from
  canonical card names to copy counts. `src/decks.rs` compiles those files into
  the binary, so the engine and browser build do not need runtime filesystem
  access.
- `src/game/` keeps the rules state machine together while separating its
  decision, event, mana, observation, and test vocabulary into small modules.

The original `poc` module remains as a compatibility façade. New code should
prefer `card::catalog()`, `card::cards::*`, and the functions in `decks`.

Format legality is reprint-aware: a canonical card is legal when it is a basic
land or at least one of its cataloged printings belongs to an allowed set.
Copy limits, banned lists, and restricted lists still apply to the canonical
card rather than separately to each printing. Decks and games currently carry
canonical `CardDefinitionId` values; the set-and-variant `CardPrintingId`
provides room for future basic-land art selection, but the UI does not select
art variants today.

## Current scope

The engine currently supports:

- per-game format profiles with isolated card legality, construction rules,
  restricted/banned lists, and mana-pool behavior
- deck validation and seeded, reproducible setup
- hidden-information-safe observations and deterministic legal actions
- the priority-bearing turn skeleton, active player, and priority passing
- the stack and last-in-first-out resolution for spells and non-mana activated
  and triggered abilities
- basic and nonbasic land plays, five-color and colorless mana sources, and EC
  phase-boundary mana burn
- player damage, concession, and empty-library loss conditions
- public battlefield, graveyard, and stack observations
- an authoritative event log for replay and debugging consumers
- London mulligans and player-selected cleanup discards
- staged attacker and blocker declaration, player-selected combat damage
  assignment, and trample
- summoning sickness, haste, temporary modifiers, marked damage, and death
- colored and colorless mana, generic and variable-X costs, and mana burn
- multi-target spells, copy retargeting, activated and triggered choices, and
  restricted untaps
- staged public/private decisions with bounded multi-selection, cancellation,
  bot preferences, and continuations across costs and effect resolution
- functional behavior for the original 128-card Old School corpus, with engine
  support for its shared mana, removal, discard, draw, tutor, and global-effect
  package
- complete declarative records for every additional card in the Standard Top 8,
  with common casting, mana, land-entry, creature, flash, and combat metadata
  already consumed by the engine
- twenty-three fixed 60-card decks with 15-card sideboards across two formats
- a small bot API with seeded random and card-aware handcrafted policies

The event log is intentionally omniscient and must not be passed directly to a
bot; bots consume `PlayerObservation`.

Complex choices are represented by `DecisionObservation`. The engine exposes
the prompt, visibility, selectable objects, selection bounds, and legal
`ChooseDecision` actions without exposing private options to the other player.
Recall, Balance, Demonic Tutor, Time Vault, and Sylvan Library all use this
shared path.

The POC is playable end to end, but it is not yet a general implementation of
the Comprehensive Rules. Fireball supports its multi-target additional cost
and even damage division, Fork can choose new targets for its copy, and combat
damage uses the current player-selected assignment rules. Supported non-mana
activated and triggered abilities become independent stack objects with frozen
source, target, and event information; simultaneous triggers are explicitly
ordered and targeted before priority returns. Mana abilities remain immediate
and never use the stack. Unsupported clauses stay visible in the catalog's
derived coverage instead of silently pretending to work.

## Built-in decks

### Old School 93/94

The proof of concept contains fifteen powered EC archetypes:

- `decks::goblins()` is a tribal aggro deck built around Goblin King, Goblin
  Grenade, Goblin Balloon Brigade, and Goblins of the Flarg.
- `decks::sligh()` is a curve-based aggro/burn deck with Ironclaw Orcs, Ball
  Lightning, Granite Gargoyle, Dragon Whelp, and direct damage.
- `decks::artifacts()` is Atog Smash, using Atog, Orcish Mechanics, Black Vise,
  Ankh of Mishra, Copper Tablet, and fast artifact mana.
- `decks::robots()` uses Mana Vault to accelerate Juggernaut, Su-Chi, and
  Triskelion, backed by Atog and red removal.
- `decks::the_deck()` is the format's namesake control strategy: Counterspell,
  Mana Drain, Swords to Plowshares, Balance, Demonic Tutor, Jayemdae Tome, and
  the format's restricted card-draw suite.
- `decks::mono_black()` combines Dark Ritual, Hypnotic Specter, Hymn to Tourach,
  Sinkhole, and Juzam Djinn.
- `decks::white_weenie()` curves Savannah Lions and Icatian Javelineers into
  Crusade and Armageddon.
- `decks::erhnamgeddon()` pairs Birds of Paradise and Erhnam Djinn with white
  removal and Armageddon.
- `decks::counterburn()` is blue-red tempo with Serendib Efreet, permission,
  Psionic Blast, and burn.
- `decks::lions_dib()` is the blue-white Savannah Lions/Serendib Efreet tempo
  shell.
- `decks::bwr_aggro()` is a black-white-red knight and burn aggro shell.
- `decks::gr_aggro()` is a green-red creature deck built around Kird Ape,
  Argothian Pixies, mana Elves, and pump spells.
- `decks::troll_disk()` is black-red Sedge Troll control with Nevinyrral's Disk
  and land destruction.
- `decks::jeskai_aggro()` is a blue-white-red tempo deck with burn and
  permission.
- `decks::lions_dib_bolt()` is the Lion/Dib shell with a dedicated Bolt package.

Their cores are based on the [TC Decks Goblins aggregate][goblins-data], the
[Wak-Wak Sligh archetype guide][sligh-guide], a representative
[EC Atog Smash list][atog-list], and the [TC Decks Artifact Aggro
aggregate][robots-data]. The Robots list stays mono-red for this implementation
slice while preserving the archetype's fast-mana and large-artifact core.
The control shell follows the recurring core in the [TC Decks The Deck
aggregate][the-deck-data].

The decks use some combination of Mishra's Factory, Strip Mine, Black Lotus,
Mox Ruby, Wheel of Fortune, Chaos Orb, and Sol Ring. The artifact deck also
uses the off-color Moxen; every Mox now produces its printed color.

EC Chaos Orb normally uses a physical dexterity flip. The headless simulator
instead treats a resolving Orb activation as a deterministic successful flip
against the chosen permanent. The activation uses the stack, and removing the
Orb before resolution does not remove that independent ability object, but its
custom source-presence check nullifies the flip; an illegal target also makes
it fail normally. This keeps seeded games reproducible and makes the format
playable without modeling a human motor skill.

The expanded corpus now includes the recurring top-table cards that make those
decks distinct: Copy Artifact, Tetravus, Icy Manipulator, Relic Barrier, The
Abyss, Sedge Troll, Stone Rain, Kird Ape, Scryb Sprites, Llanowar Elves, Giant
Growth, Berserk, Pendelhaven, Moat, Wrath of God, Dust to Dust, Hurkyl's
Recall, Energy Flux, and City in a Bottle. It is still intentionally based on
cards in actual archetypes rather than every card legal in the format.

### ISD–RTR Standard

The Standard profile contains the complete 60-card main deck and 15-card
sideboard for each member of the [SCG Open Atlanta Top 8][scg-atlanta]:

- Rudy Briksza — Naya Midrange
- Joseph Greer — G/R Aggro
- Mike Fyrberg — B/G Midrange
- Jimmie Smith — Naya Midrange
- Korey McDuffie — U/W/R Flash
- Phillip Lorren — U/W Flash
- Clayton Arch — U/W Flash
- Drew Kuenzinger — Junk Reanimator

The published Clayton Arch list contains three copies of Celestial Purge, a
card that was not legal in this Standard pool. Its built-in playable list uses
Celestial Flare as the likely transcription correction and records that
inference in the YAML source comments.

The catalog carries the canonical rules identity and every known set-and-
variant printing, along with the cost, type, rules text, and stats for every
card used by these lists. Ordinary mana sources and creatures are playable
now; specialized effects are being implemented incrementally as reusable
rules primitives are added or, when a reusable shape is not yet clear, through
a clause-owned custom selector and card-specific resolution. Until a
metadata-only nonland, noncreature card is implemented, the engine withholds it
from legal actions instead of silently resolving an approximation; card
previews label this staged support explicitly.

## Bot policies

Bots implement the `Policy` trait by choosing one of the legal actions in a
hidden-information-safe `PlayerObservation`. `play_game` drives two policies
until the game ends or a caller-provided action limit is reached.

The built-in `RandomPolicy` samples uniformly from non-concession actions with
a seeded PRNG. `HandcraftedPolicy` is a deterministic baseline with simple
mulligan, casting, targeting, combat, mana, and card-specific heuristics. It is
deliberately inspectable rather than sophisticated.

Run the reproducible, seat-swapped sanity gauntlet with:

```sh
cargo run --release --bin policy_sanity
```

The gauntlet uses mirror matches for every built-in deck in both formats,
isolating policy quality from deck strength.

Run the broader rules audit with:

```sh
cargo run --release --bin rules_audit
```

This plays Random/Random and both seatings of Handcrafted/Random across every
built-in deck matchup and 100 seeds. After every action it checks that both
players agree on public state, hidden hand sizes remain consistent, legal
actions are unique, the correct player owns the decision, and completed games
expose no further actions. Pass a different seed count as the final argument
when doing a longer soak run, for example
`cargo run --release --bin rules_audit -- 1000`.

## Engine performance

Measure the native Rust engine first. The default workload drives deterministic
Random/Random games through the normal `observe`/`apply` loop; the cheap policy
keeps policy selection from obscuring engine costs. Profile the WASM adapter or
browser only when a consumer-specific problem remains after measuring the
native engine.

Use [Hyperfine](https://github.com/sharkdp/hyperfine) on the normal release
binary for elapsed-time evidence:

```sh
cargo install --locked hyperfine
make benchmark-engine PROFILE_GAMES=2000 PROFILE_SEED=1 \
  BENCHMARK_OUTPUT=target/profiles/engine-before-benchmark.json
```

The target prints one untimed run so its deterministic outcome counts can be
checked, then runs Hyperfine with warmups and repeated measurements. Override
`BENCHMARK_WARMUP` and `BENCHMARK_RUNS` when needed. Use Criterion only after a
hot path can be isolated as a stable in-process benchmark; whole-game
throughput remains the primary regression measure. [Criterion](https://github.com/bheisler/criterion.rs)
is not a dependency until such a benchmark exists.

Performance measurement is deliberately outside the normal edit-test loop.
Most changes do not need a benchmark, and a qualitative expected-impact note
is often enough. Contributors and agents should use judgment and measure when
the result is likely to inform a design or review decision, preferably once at
a coherent checkpoint rather than repeatedly during implementation.

For a selected advisory branch comparison, ensure a baseline exists for the
exact commit currently named by the local `main` ref, then benchmark its saved
binary and the current worktree together in one Hyperfine session:

```sh
make benchmark-engine-baseline
make benchmark-engine-compare
```

`benchmark-engine-baseline` reuses a matching saved result and measures only
when the selected revision or relevant environment has changed.
`benchmark-engine-compare` performs the same lazy baseline check, so when local
`main` advances the next chosen comparison uses the new commit without
touching or switching the current worktree. Neither command fetches; use
`PERFORMANCE_BASELINE_REF=origin/main` when that is deliberately the local ref
to measure. The shared release binary, deterministic outcome, metadata, and
baseline Hyperfine JSON live under
`$(git rev-parse --path-format=absolute --git-common-dir)/penta-performance-cache/layout-v1/`,
where every linked worktree in the clone can reuse them. The comparison JSON
and its metadata default to `target/profiles/engine-main-compare*.json`.

The cache keys main revision, machine, build environment, effective Cargo
configuration, compiler/tool versions, workload, seed, and measurement
settings separately. Comparisons still run main and current together rather
than treating an older saved mean as same-session evidence. A deterministic
outcome mismatch is reported because timing then includes changed game paths
as well as implementation cost. There is no regression threshold or CI
performance gate; when measured, report the distribution and delta as review
context.

Use [Samply](https://github.com/mstange/samply) to locate CPU hotspots with the
Firefox Profiler call tree, flame graph, timeline, and source views. The
separate `profiling` build keeps release optimization and adds symbols:

```sh
cargo install --locked samply
make profile-engine PROFILE_GAMES=20000 PROFILE_SEED=1 \
  PROFILE_OUTPUT=target/profiles/engine-before.json.gz
make profile-engine-open \
  PROFILE_OUTPUT=target/profiles/engine-before.json.gz
```

The profiler UI visualizes a native process capture; it does not profile
Penta's browser interface.

Branch captures, ad hoc Hyperfine exports, and allocation traces belong under
the ignored `target/profiles/` directory. The maintained main benchmark above
is the exception and is managed under Git's common directory. Samply releases
that store symbols in an adjacent `.syms.json` sidecar need that file kept
beside the capture. For a broader CPU sample covering every built-in deck in
both formats, use `make profile-engine-all`.

Samples in allocator functions measure allocator CPU time, not allocation
counts or bytes. When heap churn is the question, run the same symbol-rich
native workload under a real allocation profiler:

```sh
make build-profile-engine

# macOS with full Xcode installed
xcrun xctrace record --template 'Allocations' \
  --output target/profiles/engine-allocations.trace --launch -- \
  target/profiling/penta-match --p1 random --p2 random \
  --deck1 Random --deck2 Random --games 2000 --seed 1

# Linux with Valgrind installed
valgrind --tool=dhat \
  --dhat-out-file=target/profiles/engine-dhat.out \
  target/profiling/penta-match --p1 random --p2 random \
  --deck1 Random --deck2 Random --games 2000 --seed 1
```

See Apple's [Instruments memory guidance](https://developer.apple.com/documentation/xcode/gathering-information-about-memory-use)
and the [Valgrind DHAT manual](https://valgrind.org/docs/manual/dh-manual.html)
for interpreting the allocation results.

Use sampled profiles to choose one optimization target, then repeat the exact
game count, seed, build profile, and machine before and after the change. The
shared `profile-engine-performance` skill also provides an optional,
schema-guarded headless attribution script for agent workflows. Standard
profiler views remain the primary way to inspect a capture. Profiling is opt-in
and is not part of CI; routine baseline tracking does not record a Samply
capture.

## Web interface

The `web/` application runs the same Rust engine in the browser through the
small `wasm/` adapter. Generated bindings are built locally under
`web/app/wasm`, which is ignored by Git, so Vite can bundle the module and its
binary together. The browser submits an index from the engine's current
legal-action list for ordinary actions. Generic decisions submit a decision ID
and option IDs through the same WASM facade; the browser never reconstructs or
mutates game rules in TypeScript.

Build fresh browser bindings after changing the Rust API:

```sh
cargo install wasm-bindgen-cli \
  --version "$(./scripts/wasm-bindgen-version.sh)" --locked
make doctor
make build-wasm
```

Then start the interface:

```sh
cd web
pnpm install
pnpm dev
```

The primary checkout uses port 3000. Each linked Git worktree automatically
uses its own stable port; run `pnpm run dev:url` from `web/` to print that
checkout's URL.

The `dev`, `build`, and `test` workflows ensure the ignored WASM bindings are
up to date before they run. Cargo checks incrementally, and `wasm-bindgen` is
skipped when the compiled module and generator version are unchanged.

## Checks

The root `Makefile` is the shared task catalog for Rust, WASM, web, and binding
checks. `make help` lists every target. During development, select the closest
suite and optionally narrow it by test name:

```sh
make test-engine-unit FILTER=mana_burn
make test-engine-integration FILTER=mountain_casts
make test-web-wasm-pacing PATTERN='auto-pass'
make typecheck-web
make lint-web
```

Use `make check-fast` for a broad checkpoint without the production web build
or simulation-heavy sweeps. Once a change is stable and ready for a push or PR,
`make check` runs the complete engine and web gate. Binding changes can use
`make check-bindings-c` or `make check-bindings-python` while iterating, then
strict `make check-bindings` for both. `make check-bindings-available` is the
explicit best-effort local variant when Python is unavailable; `make ci` never
skips a repository gate. The old `scripts/check-all.sh` entry point remains as
a compatibility wrapper.

All Cargo validation commands enforce the committed lockfiles, matching the
web install's frozen-lockfile behavior. `make doctor` checks the local compiler,
Node, pnpm, Python/C binding prerequisites, ShellCheck, Actionlint, the pinned
Rust compiler/components and WASM target, and the exact `wasm-bindgen` CLI
version required by `Cargo.lock`; it reports setup problems without installing
tools implicitly. Install ShellCheck and Actionlint through your platform's
package manager (`brew install shellcheck actionlint` on macOS).

`rust-toolchain.toml` pins the Rust version, components, and the wasm
target, so rustup installs the same compiler for contributors, maintainers,
and CI. Clippy runs pedantic with `-D warnings`, where a newer toolchain can
fail a build an older one passes; pinning makes that a deliberate upgrade
commit instead of a surprise. A fresh clone needs no `rustup` commands.

[ec-rules]: https://www.eternalcentral.com/9394rules/
[goblins-data]: https://www.tcdecks.net/archetype.php?archetype=Goblins&format=Old+School&src=all
[sligh-guide]: https://www.wak-wak.se/9394decks/sligh
[atog-list]: https://tappedout.net/mtg-decks/atog-smash-9394-1/
[robots-data]: https://www.tcdecks.net/archetype.php?archetype=Artifact+Aggro&format=Old+School&src=all
[the-deck-data]: https://www.tcdecks.net/archetype.php?archetype=The+Deck&format=Old+School&src=all
[scg-atlanta]: https://www.mtgtop8.com/event?e=5640&f=ST
