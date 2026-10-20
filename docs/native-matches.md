# Native single-match host

`penta-host` owns one match in one native process. It calls the shared Rust
engine directly; it does not start Vite, a browser, WASM, or the hosted-room
daemon. The existing [hosted session workflow](bot-sessions.md) remains available.
The native host and its stateless client make no model calls. Pilots choose
exact actions or explicitly authorize conditional priority passes.

## Start and distribute seats

Build the host with `make build-native-host`. Start the resulting binary
with registered format and deck names:

```sh
target/release/penta-host --format FORMAT_SLUG --p1-deck P1_DECK --p2-deck P2_DECK --match-mode first-to-two-wins --port 0
```

Replace the uppercase placeholders with actual registry values, quoting names
containing spaces. `one-conclusion` is also supported. Port zero asks the OS for
an available port. Keep the process running; its startup stdout is JSON:

```json
{"apiVersion":1,"server":"http://127.0.0.1:PORT","organizerSecret":"SECRET","status":"ready"}
```

The organizer calls `seats` with its secret. Every native client invocation
accepts a single JSON object, inline, through `@FILE`, or through stdin (`-`):

```sh
node tools/penta-agent/native.mjs seats -
```

Supply `{"server":"http://127.0.0.1:PORT","secret":"ORGANIZER_SECRET"}` on stdin.
The result contains `seats.p1.secret` and `seats.p2.secret`. Give each pilot only
the server URL and its own seat secret. Retain the organizer secret separately.
Start pilots with fresh model context so they do not inherit the organizer's
launch output or the other seat's secret. For Codex subagents, use
`fork_turns: "none"` and pass only the seat assignment, skill, URL, and own secret.
The client never writes credentials, connection handles, retry bodies or
observations to disk. Stdin avoids putting credentials in command arguments;
tool transcripts may still contain submitted inputs.

## Play and reconnect

Use the same invocation pattern for each method. All inputs include `server`
and `secret`; the examples below show the additional fields.

| Method | Additional fields | Purpose |
| --- | --- | --- |
| `observe` | none | Current seat view |
| `wait` | `waitMs` up to 25000 | Bounded wait for a seat view |
| `choose` | `ticket`, `requestId`, optional `options` | Submit one displayed choice |
| `play` | `revision`, `choices`, `requestId` | Submit an explicitly decided batch |
| `yield` | `revision`, `until`, `requestId`, optional `maxPasses`, `waitMs` | Pass priority until any requested condition matches |
| `cancelYield` | `requestId` | Cancel this seat's retained yield instruction |
| `concede` | `requestId` | Concede |
| `receipt` | `requestId` | Recover a submitted request's outcome |
| `transcript` | optional `offset`, `limit` up to 100 | Recover seat-visible history |
| `note` | `text` | Retain a seat note in host memory |
| `catalog` | none | Public canonical card catalog |
| `inspect_ref` | `reference`, optional `offset`, `limit` up to 100 | Inspect a current compact-view reference |

Responses distinguish `ready`, `waiting`, `complete`, and `paused`. At `ready`,
the default `pilot-v1` presentation supplies choices and revision-bound
tickets such as `REVISION:a7`. Submit that ticket unchanged:

```json
{"server":"http://127.0.0.1:PORT","secret":"SEAT_SECRET","ticket":"REVISION:a7","requestId":"p1-move-17"}
```

A decision ticket requires the explicitly selected option IDs in `options`,
including `[]` when choosing an empty selection. Order matters. A single `play`
accepts `{index}`, `{decision, options}`, or `{action}`. Multi-choice batches
require exact action values or explicit decisions because indices can change.
An accepted prefix remains committed and is reported in the receipt; ticket
resolution for batches is not provided by this stateless adapter.
`host.receipt.updates` in the typed view preserves this seat's new events and automatic inspections
across the entire accepted prefix, including when the next view is `waiting`.
Read these alongside the current observation; retries return the same receipt.
Supply `waitMs: 25000` with a submission to wait for the next own decision in
the same request. Waiting never chooses another action.

Choose a new request ID for each logical mutation and retain its exact method
and arguments until the outcome is known. After a timeout or lost output,
resend the identical request or query `receipt`; never invent a replacement ID.
The host retains receipts in memory across client restarts. The client does
not generate IDs, retry automatically, or remember the last submission.

To reconnect a pilot, keep the host running and reuse that seat's server URL
and secret. Resolve an uncertain submission first, then call `observe` and
page through `transcript` as needed. A new model process does not require a new
match or a new CLI daemon. `note` can retain a concise strategic reminder for
that seat, but model conversation context is not automatically transferred.
Stop the old pilot before handing control to its replacement; reusing a seat
secret does not revoke the old pilot's access.

Compact references are rebuilt from the current same-revision view. They expire
when the revision changes; call `observe` again after a stale-reference error.
Pass `presentation: "exact"` to receive the host's raw response, including the
canonical observation. A catalog failure leaves the typed position available
with unavailable printed references and a warning. If rendering itself fails
after a committed move, the adapter returns the exact response and a
presentation warning, preserving the receipt.

## Typed pilot presentation

The native client defaults to `pilot-v1`. Its
[JSON Schema](../tools/penta-agent/pilot.schema.json) is the source for the
[TypeScript declarations](../tools/penta-agent/pilot.d.ts). Read the contract
without a running match or credentials:

```sh
node tools/penta-agent/native.mjs schema
```

Every pilot position response has `presentation`, `status`, `revision`, `host`,
and `position`.
`position` is null while waiting or paused. Ready positions contain named
`situation`, `players`, `zones`, `updates`, `choices`, `decision`, `facts`,
`provenance`, and `rules` fields. Collections consistently use
`{items, total, reference}`: `items` is always an array, and `reference` is null
unless more detail requires inspection. No collection switches to a
`{shared, rows}` table. An absent canonical zone is explicitly null.

Each entry in `position.choices.items` has a `ticket`, a readable `label`,
and the exact canonical `action`. Selection decisions retain their bounds,
option IDs, order, and other schema fields. Labels identify player targets;
exact action fields remain authoritative and distinct choices are never merged.
Unknown engine fields remain available in the exact action or `facts`.
`inspect_ref` retrieves explicitly referenced detail.

`host` preserves the native envelope, including receipts and this seat's
`yield` state. An active or stopped yield reports its conditions, pass count,
limit, stop reason, and matching condition index. Waiting reveals no opponent
revision or choice count.

`presentation: "decision-v1"` retains the previous compact native view;
`presentation: "exact"` bypasses presentation. The hosted adapter keeps its
existing default. Regenerate declarations after a schema change with
`node tools/penta-agent/generate-pilot-types.mjs`.

## Conditional priority yields

The usual instruction is "pass until a particular phase or step, unless the
opponent does something." Conditions are a typed OR list, not a natural-language
program. For example, call `yield` with the current revision:

```json
{
  "server": "http://127.0.0.1:PORT",
  "secret": "SEAT_SECRET",
  "requestId": "p1-yield-17",
  "revision": "CURRENT_REVISION",
  "until": [
    {"type": "nextStep", "step": "End", "activeSeat": "either"},
    {"type": "opponentActs"}
  ],
  "waitMs": 25000
}
```

This passes ordinary priority until the next end step begins or an opponent
action interrupts it, whichever happens first. It stays active while the other
pilot plays and across client reconnects. It returns control at the next legal
decision for this seat; it cannot interrupt resolution or grant priority.

| Condition | Meaning |
| --- | --- |
| `nextStep` | Stop on a future entry into `step`, optionally on `self`, `opponent`, or `either` player's turn |
| `opponentActs` | Stop for an opponent's visible spell, ability, land, mana activation, or combat declaration |
| `opponentCastsSpell` | Stop specifically when the opponent casts a spell |
| `nonManaAction` | Stop when this seat has a legal alternative other than passing or activating a mana ability |

`nextStep` uses the canonical names `Upkeep`, `Draw`, `PrecombatMain`,
`BeginningOfCombat`, `DeclareAttackers`, `DeclareBlockers`, `CombatDamage`,
`EndOfCombat`, `PostcombatMain`, `End`, and `Cleanup`. Use `PrecombatMain`,
`BeginningOfCombat`, or `PostcombatMain` for the corresponding phase boundary.
The current occurrence does not count: requesting `End` during an end step
targets a later end step. A skipped step does not count as an entry.

Add `{"type":"nonManaAction"}` to the example when the pilot also wants to
reconsider as soon as another legal action is available. Explicit payment
entry counts as a non-mana action; the host does not guess whether a payment
will enable a useful play. If this condition already matches, yielding stops
without passing. `opponentActs` includes mana, so pilots wanting a narrower
interruption should select `opponentCastsSpell` explicitly.

Every non-priority decision requiring a pilot stops the yield; attackers,
blockers, targets, payments, and other selections still belong to the pilot.
The engine continues to advance uniquely determined continuations, such as
finishing an empty attack declaration. The instruction expires at a game
boundary and after `maxPasses` automatic passes (default 64,
maximum 256), or can be stopped with `cancelYield`. Conditions are evaluated
using only the yielding seat's visible information. The host retains the
stop reason and the seat's updates, and records each actual pass for replay.
Yielding does not change the engine's classification of forced actions.

## Lifetime, isolation and debug export

All match state, receipts, transcripts and notes are volatile host memory.
Closing the client does not close the host. A hard host crash, process exit,
or machine restart loses the live match; this is not a durable restart service.
Keep the original host alive for reconnects.

The organizer can call `status`, `export`, and `close`. Export before closing
when a debug record is wanted:

```sh
node tools/penta-agent/native.mjs export - > native-match-export.json
```

Supply the organizer's server and secret through stdin. This explicitly saved
debug bundle can contain information hidden from players; do not give it to
pilots during play or treat it as a sanitized public transcript. Export is not
a promise of an import/resume facility. Call `close` with the organizer secret
when the match is no longer needed.

An unexpected engine failure restores the last committed position and pauses
play. The organizer can still export the accepted command prefix, last good
observations, failing command, and bounded diagnostics. Completed matches also
stay available until explicit close. Transcripts retain seat-visible positions,
choices, receipts and notes; their historical positions omit the large derived
reconstruction checkpoint, which remains available in current exact observations.

Check that a saved bundle reproduces its committed position with:

```sh
target/release/penta-host replay native-match-export.json
```

Replay requires the same simulation fingerprint and native replay version. It
compares both final seat observations and reports the failed command separately;
it does not automatically retry that command. The private seed is encoded as a
decimal string so JSON tools preserve its full precision. Exports omit credentials.

Seat credentials select the host's seat-specific observations and operations;
the organizer is separate from the pilots. This is not a hardened sandbox:
same-user agents with unrestricted shell, process inspection or access to each
other's tool transcripts may bypass intended secrecy. For enforced isolation,
restrict pilots to seat-bound tools and prevent access to organizer credentials,
other pilots' inputs, and host memory. Separate model prompts alone provide an
honor-system boundary.
