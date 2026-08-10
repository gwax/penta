# Repository instructions

This file is canonical for every agent. `CLAUDE.md` imports it rather than
restating it, so put repository guidance here and leave that file alone.

Skills live in `.agents/skills/<name>/`, and `.claude/skills/<name>/SKILL.md`
symlinks to the same file so Claude Code discovers them too. A skill body is
therefore read through two directories: write paths inside one from the
repository root, never relative to the file. `tests/agent_skills.rs` enforces
all of this.

## Optional reference material

Magic rules and card data may be available in an optional development cache
under Git's common directory. One cache is shared by every linked worktree in
the clone. Use `$query-magic-references` for efficient read-only access to its
generated Scryfall SQLite index and `$refresh-magic-references` to locate,
inspect, migrate, populate, or rebuild the cache.

Locating the cache, querying it, checking `status scryfall-index`, and viewing
`lock-status` are low-friction read operations; they do not justify a refresh.
Fetch, index, and migration commands mutate shared clone state and require
explicit human approval. Run them rarely: only when material is missing or
corrupt, the required database schema is unavailable, or the current task
genuinely requires fresher source data. Do not refresh merely because a new
worktree was created. If the cache is absent, stale for an irrelevant purpose,
or unavailable, continue with appropriate authoritative online sources.

Treat `refresh.lock` metadata as diagnostic information. The kernel lock is
authoritative; never delete or bypass the lock merely because its recorded
owner appears stale. Do not commit or ship downloaded reference payloads.

Treat both reference skills as maintained development tooling. When repeated
work exposes a missing field, relationship, index, or query pattern, update the
refresh builder, both skills, and the documented schema together, then rebuild
and validate the cache. Avoid expanding them for isolated one-off questions.

## Card implementations

Use a card's ordered `AbilityDef` clauses as the primary implementation
boundary. New or migrated card work should start by representing each printed
clause there with its explicit category and, where applicable, costs, targets,
and effects, even when its final effect still needs custom execution. Reuse
constructors from `card::abilities` and declarative rules primitives where they
fit, and keep rules text, implementation coverage, and execution tied to the
same clause.

When the current vocabulary cannot express a card, choose the extension by the
semantics rather than by the fastest place to add a branch:

- A recurring mechanic or general Magic rules concept belongs in a reusable,
  card-agnostic engine primitive that ability definitions can invoke. Adding
  such a primitive is building the engine; it is not an engine-level
  implementation of one named card.
- Genuinely card-specific behavior, or behavior whose reusable shape is not yet
  clear, should use or introduce a card-scoped implementation reached from the
  relevant ability clause. Today a clause can own its custom selector,
  coverage, and explanation even though some compatibility handlers remain
  centralized. Keep timing, costs, targets, and stack behavior declarative
  around that custom portion whenever possible. Custom resolution should not
  change an explicit ability category or let an activated or triggered
  non-mana ability bypass the shared stack.
- A direct card-identity special case in generic `Game` or state-machine flow
  that bypasses the clause-attached custom-resolution boundary is an escape
  valve for particularly weird or difficult cards, not the default
  implementation path. Keep it narrow, explain why the definition or
  card-scoped path is insufficient, preserve accurate clause-level coverage,
  and test both the shared rules behavior and the exceptional result.

Be pragmatic about the boundary. Do not build a speculative framework for one
exceptional card, but do not multiply one-off engine branches for a rules
concept that should be shared. Existing engine-level special cases are
migration inventory rather than precedent; when a repeated pattern emerges or
an existing case is already being changed, move it toward a reusable definition
or card-scoped implementation when that work is reasonably in scope.

## Performance awareness

Treat performance as review context, not a merge gate, and prefer clear,
correct designs. Watch mainly for accidental multiplier-sized slowdowns, such
as 2× or 4×; roughly 20% slower is ordinarily context, not a reason to optimize
or block. These examples calibrate judgment rather than set thresholds.

Keep performance checks out of the normal edit-test loop. Most changes need
only a qualitative assessment. Measure only when evidence is likely to change
a decision, then prefer one `$profile-engine-performance` comparison at a
coherent checkpoint. It uses a lazily refreshed local-`main` baseline shared
under Git's common directory; compare like-for-like and note any limits. Once a
routine check rules out a scale-changing regression, report and stop rather
than profiling or rerunning to recover another roughly 10%, unless performance
is explicitly the task or a concrete user-visible requirement exists.

In pull requests and handoffs, include brief performance context only when
useful. “No expected impact” and “Not measured” are valid; never benchmark just
to fill in a report.

## Protocol versioning

A branch or pull request containing one or more incompatible protocol changes
must set the protocol version to exactly one greater than the target branch's
version. Do not bump it again for additional incompatible changes or
intermediate commits in the same branch or pull request. After rebasing,
re-check the target branch's protocol version and adjust if it changed.

## Validation

Use the root `Makefile` as the canonical entry point; `make help` lists the
available suites.

- During implementation, run the narrowest target or filtered test that
  exercises the changed behavior. Do not run the full gate after every edit.
- Run `make check-fast` at coherent checkpoints. It covers formatting, lints,
  normal Rust tests, and the fast browser-facing WASM suite without a
  production web build or the simulation-heavy tests.
- Run the relevant slow target when a change affects simulation, policy,
  auto-pass, combat progression, or another behavior covered by that suite.
- Run `make check` once the change is stable and ready to push or open as a PR.
  If bindings changed, also run `make check-bindings`; `make ci` runs every
  repository gate.
- Do not rerun an unchanged passing suite unless later edits could affect it.
  In the handoff, list the exact targets run and call out any deferred gate.
- For UI changes, command-line checks do not replace the visual verification
  below.

Map changed paths to the narrowest useful target before broadening:

- `src/game/**`, `src/card/**`, and core rules: `make test-engine-unit
  FILTER=<name>` or `make test-engine-integration FILTER=<name>`.
- `src/policy.rs` and policy behavior: `make test-policy FILTER=<name>`; add
  `make test-rust-slow` when the simulation sweeps are relevant.
- `src/protocol.rs`: `make test-engine-unit FILTER=protocol`, then
  `make test-wasm-rust` plus the matching browser contract suite when the
  exposed bridge can change. For `wasm/**`, start with the latter two targets.
- `web/app/**`: `make lint-web`, `make typecheck-web`, and the matching WASM or
  render target, followed by the required browser verification for UI changes.
- `bindings/penta-ffi/**` or `bindings/penta-py/**`: use the corresponding
  `make check-bindings-*` target; use strict `make check-bindings` before handoff.
- `Makefile`, `scripts/**`, or `.github/workflows/**`: `make lint-infra` and
  exercise the changed orchestration target. For `.github/dependabot.yml`,
  validate against GitHub's Dependabot 2.0 schema. Run `make doctor` when
  prerequisites are in question.

## UI changes

For every change that can affect the web interface:

1. Start or restart the local server from the current working tree. Confirm that
   the worktree-specific URL from `cd web && pnpm run dev:url` is served by that
   process; do not accept a fallback port or assume an older server picked up
   the change.
2. Open the rendered application in a browser and inspect it visually. A
   successful build, DOM snapshot, or HTTP response is not sufficient.
3. Check at least a 1280×720 laptop viewport. Verify that important content is
   visible and readable, with no unintended clipping, overlap, off-screen
   controls, or inaccessible horizontal overflow.
4. Exercise enough UI state to display the changed component. For game-table
   changes, check cards in hand and cards on the battlefield when applicable.
5. Take a fresh screenshot after the final code change and inspect it before
   reporting completion.

Keep the verified local server running for the user unless they ask otherwise.
