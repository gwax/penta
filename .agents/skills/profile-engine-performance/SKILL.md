---
name: profile-engine-performance
description: "Profile and attribute Penta engine performance with reproducible deterministic workloads and saved Samply captures. Use when investigating CPU or allocation hotspots, separating native-library samples from their engine callers, comparing before-and-after performance, or choosing the next optimization target. Do not compare captures made with different workloads, seeds, build profiles, or machines, and do not treat sample percentages as benchmark speedups."
---

# Profile Engine Performance

Use the optimized symbol-rich engine build for attribution and a separate
unprofiled run for wall-clock measurement. Keep every capture and its symbol
sidecar under ignored `target/profiles/`; never commit either file.

## Establish a reproducible baseline

Choose one workload and record its command, game count, seed, revision, and
machine. Use the default deterministic Random/Random deck rotation for normal
engine work:

```sh
make build-profile-engine
target/profiling/penta-match --p1 random --p2 random \
  --deck1 Random --deck2 Random --games 2000 --seed 1
```

Time the unprofiled command at least three times and report the median. Preserve
the printed outcome counts; they are a quick determinism check after an edit.
Use the broader `make profile-engine-all` policy gauntlet only when deck or
format breadth matters more than focused engine attribution.

Install Samply only with the user's approval if it is missing. Recording may
require host process-inspection or performance-counter permission; request that
permission when the sandbox or operating system denies it.

Capture a baseline without overwriting an earlier file:

```sh
make profile-engine PROFILE_GAMES=20000 PROFILE_SEED=1 \
  PROFILE_OUTPUT=target/profiles/engine-20000-seed1-before.json.gz
```

Samply 0.13 writes symbols to an adjacent `.json.syms.json` file. Keep it next
to the `.json.gz` capture so the analyzer can discover it automatically.

## Attribute the profile

Use the bundled standard-library analyzer from the repository root:

```sh
PROFILE_ANALYZER=.agents/skills/profile-engine-performance/scripts/profile_attribution.py
python3 "$PROFILE_ANALYZER" summary \
  target/profiles/engine-20000-seed1-before.json.gz --top 15
```

Repeat `--caller-of SUBSTRING` to identify the immediate engine callers of a
suspected function. Add `--json` when stable machine-readable output is more
useful than tables.

Interpret the sections distinctly:

- Raw leaf samples show what was executing at the sample instant.
- Attributed self assigns native allocator or memory-operation leaves to the
  nearest Penta application frame on that stack.
- Inclusive weight counts a function at most once per sample and shows stack
  presence, not exclusive time.
- Native-leaf attribution exposes which engine callers led to allocator,
  platform-memory, or kernel work.

Use absolute sample-weight changes first and shares as context. Profiler runtime
includes sampling overhead and is not a substitute for the separate wall-time
benchmark.

## Optimize and compare

Change one well-supported hotspot at a time. Preserve observable ordering,
identity, deterministic outcomes, and other semantics covered by the path.
Run the narrowest relevant tests during implementation, then the repository
gate required by `AGENTS.md` before handoff.

Capture the exact same workload, seed, game count, build profile, and machine:

```sh
make profile-engine PROFILE_GAMES=20000 PROFILE_SEED=1 \
  PROFILE_OUTPUT=target/profiles/engine-20000-seed1-after.json.gz
python3 "$PROFILE_ANALYZER" compare \
  target/profiles/engine-20000-seed1-before.json.gz \
  target/profiles/engine-20000-seed1-after.json.gz --top 15
```

The profile format does not record `PROFILE_GAMES` or `PROFILE_SEED`; verify
comparability from the commands and filenames before trusting a comparison.
Report the median wall-time delta, deterministic outcome check, absolute and
share changes for the targeted samples, the new leading hotspots, exact tests,
and paths to the ignored captures.

Open a saved capture interactively only when useful:

```sh
make profile-engine-open \
  PROFILE_OUTPUT=target/profiles/engine-20000-seed1-after.json.gz
```
