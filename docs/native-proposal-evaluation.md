# Native action proposal evaluation

Manual evaluation on 2026-10-20, on one Apple Silicon host, using release
builds. This evaluates the state/proposal path described in
[native matches](native-matches.md). Raw exports, CPU captures, request logs,
probe sources, binaries, and timing JSON are retained locally under ignored
`target/profiles/direct-agent-actions/`; private game exports are not committed.

## Controlled enumeration workload

The baseline source tree matched upstream main
`139fbedccbb618110c6b3c81cfec7dd2f870891e`. The synthetic fixture uses seed 1,
turn 3 precombat main, twelve opposing Llanowar Elves, four Jeskai Revelations
in hand, and fourteen mana sources: four Great Hall of the Biblioplex, two
Starting Town, and eight cycling basic lands. This is a stress position, not
a claim about the reported remote game's actual board.

The old observation generated 1,456 cast alternatives. The proposal path
returns visible state and checks the selected action. Both builds produced
identical visible state after excluding `legalActions`, `checkpoint`, and the
simulation fingerprint. No game choice was replaced with a policy decision.

Three unprofiled measurements after one warmup, with identical fixture and
machine and no live evaluation game or compiler running:

| Operation | Baseline median | Proposal median |
| --- | ---: | ---: |
| Observe the position | 5,475.681 ms | 0.516 ms |
| Submit PassPriority | 5,471.700 ms | 0.338 ms |

Hyperfine measured the entire probe, including catalog/setup, observation
serialization, and pass: 11.386 s ± 0.158 s versus 0.275 s ± 0.021 s,
about 41.4× faster for that workload. This is not a whole-game speedup claim.
The earlier CPU sample attributed the expensive path to cast-target expansion
and repeated mana assignment.

## Model games

The organizer launched one host at a time. Each fresh-context pilot received
only its own seat capability and used the persistent memory-only CLI session.
Host processing times exclude HTTP waiting and authority-lock queueing.
Elapsed pilot time includes thinking, tool dispatch, and opponent waits.

| Run | Pilots | Result | Host interactions | Maximum processing | Total processing |
| --- | --- | --- | ---: | ---: | ---: |
| WOE-HOB, first iteration | Astra Max Control vs 6.1 Sol Medium Bant Airbending | Control won turn 21 by decking Bant | 120 | 136.532 ms | 2.463 s |
| Old School, revised interface | Astra Medium vs Astra Medium, Sligh mirror | P1 won turn 9 with Ball Lightning trample | 27 | 34.295 ms | 0.268 s |

The WOE-HOB run lasted about 17 minutes from host launch to completion. Pilots
reported 58 and 67 session calls, five proposal rejections in total, and one
explicit discovery. All recovered. Restricted mana, chosen funding order,
five-color converge, flashback, targets, combat, and additional costs were used.
The 581-command export replayed with identical final observations using its
matching evaluation binary.

The revised Sligh run took 3m10s from launch; pilot sessions lasted 2m49s and
2m38s. Both finished with `pilot-v3`, made no discovery calls, and reported
504,487 output bytes combined. One redundant finish-attack action was rejected
after its two-attack accepted prefix; the pilot recovered using the receipt.
Two inputs failed local validation: an omitted empty cast-choice object and an
unsupported yield condition. The 220-command export replayed with identical
final observations. Different decks, seeds, and model settings mean these games
are not a controlled comparison of total wall time or token cost.

Pilot feedback led to these changes before the second run:

- Expose runtime alternative-cost IDs, including granted flashback, with each
  visible object's ability metadata.
- Let a mana combination determine its internal representative color.
- Name cast payment resources `costObjects`, including cards exiled for evidence.
- Fix absent cast-offer metadata being serialized as null at completion.
- Suppress unchanged, successfully delivered ability metadata in persistent
  sessions, with `full:true` recovery and full delivery after changes/reappearance.

The first iteration produced 6,349,425 model-facing output bytes across both
pilots, larger than desirable. Reprocessing its recorded response revisions
with the metadata cache reduced ability metadata from 1,350,312 to 202,584 bytes
for Control and from 1,623,483 to 220,194 bytes for Bant, reductions of 85.0% and
86.4%. This isolates metadata delivery on the same positions; it is not a
measurement of model tokens, billing, or total game-output reduction.

Final small follow-ups allow omitted empty cast choices and omit artwork and
printing lists from inline card rules, retaining their exact references. These
were validated with focused tests after the Sligh run. Generic local schema
union errors remain less specific than the host's structured rejection messages.

## Scope and remaining limits

The reported remote multi-minute `commandMetrics.maxMicros` incidents have not
been reproduced from their exact states. These measurements establish a concrete
enumeration bottleneck and remove it from ordinary native observation, passing,
and cast proposal submission; they do not establish every remote stall's cause.

Optional `discover`, `nonManaAction` yields, explicit payment-editor discovery,
and some activated-ability checks still enumerate. `nonManaAction` also stops
conservatively for payment-editor entry, which can create extra interactions.
Phase/event yields avoid that discovery work. Cast refusal diagnostics identify
validation stages but do not yet isolate every invalid announcement field.
The host remains synchronous; this change adds no cancellation deadline or
background-job protocol for a genuinely expensive engine operation.
