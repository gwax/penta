# Native agent playtests

## Checkpoint: typed views and conditional yields

Implementation: `50a04ae2f8997d33acc568e324cb629537b8b8aa`.
Two fresh GPT-6 Astra pilots at Medium reasoning played an Old School 93/94
Sligh mirror using the native release host and `pilot-v1`.

P1 won 7–0 on global turn 15. Exact replay reproduced all 192 journal commands
and both final seat observations. Thirty yield instructions handled 139
optional priority passes; the pilots submitted six manual priority passes.
Both reported no lost response window, including after Wheel of Fortune,
opponent spell casts, and Strip Mine activation.

The run took about 19 minutes including setup, compared with about nine minutes
for the earlier different game. Accepted mutation requests were 80 versus 75.
This demonstrates less manual passing, not an end-to-end speedup. Different
shuffles, game lengths, decisions, and model/tool latency prevent a controlled
timing comparison.

### Feedback to address

1. Displayed exact actions include `index`, but exact-action submission rejects
   it. One batch was refused before mutation; choosing the ticket worked.
2. Ready views repeat verbose permanent data, printed rules, and event history.
   Re-rendered retained views measured 19,011 bytes median and 28,266 maximum,
   excluding submission receipts.
3. Receipt-time and current yield state need explicit temporal names. A receipt
   can correctly say active/six passes while the current policy says
   stopped/eight passes.
4. Ability labels should identify targets explicitly, including self-targets,
   without full UUID descriptors and routine defaults in the label.
5. Pilots need a documented local client entrypoint. Both initially tried a
   JavaScript tool runtime that could not reach localhost, then used shell Node
   helpers successfully.

P1 prioritized presentation size; P2 prioritized directly reusable actions.
Both praised conditional yields, automatic payment, typed choices,
submit-and-wait, and batch receipts.

The organizer retained the private export, seat transcripts, replay result,
measurements, and detailed feedback under the ignored local directory
`target/agent-matches/2026-10-20T08-11-09-112Z-astra-yield-sligh-mirror/`.
Those artifacts contain hidden game information and are not committed.
The host and both pilot helpers were closed after export.

## Follow-up method

Each improvement cycle uses two fresh pilots with the same model, reasoning
effort, format, and decks. Record the tested commit, exact replay verification,
accepted requests, manual and yielded passes, response size, and pilot feedback.
Keep the harness unchanged during a game. Use identical saved positions for
presentation-size comparisons; report live game elapsed time descriptively.

## Improvement round 1: persistent client and concise views

Harness: `20a90e50c75f20126496a75b02bf826ca5874a81`; unchanged release host
from `50a04ae2f8997d33acc568e324cb629537b8b8aa`.
Two fresh GPT-6 Astra Medium pilots used the maintained PTY client and `pilot-v2`.
P1 won 7–0 on turn 15. Exact replay reproduced 218 commands and both final
observations. The game took about 21 minutes including setup.

There were 95 accepted mutation requests, 151 yielded priority passes, eight
manual passes, and 176 separate forced continuations. Both pilots reported no
missed desired priority window or transport failure. Conditional stops enabled
responding to Bolt, blockers, and Chaos Orb, including an Orb response to the
opponent's activation. Both clients exited cleanly; the organizer exported and
closed the host.

The clients reported 124 calls, 1,527,484 output bytes, and two errors. Both
errors were malformed native batches after one context compaction per pilot:
P1 used hosted-style tickets; P2 omitted the `{action: ...}` wrapper. Both were
refused without mutation, then recovered using exact actions. A redundant attack
finish also produced a clearly reported accepted-prefix stop.

On this game's 220 retained observations, v2's median was 15,093 bytes versus
22,341 for v1. On the earlier baseline's identical 194 positions, the median
fell from 19,011 to 16,327 bytes, while v2 brought all previously paged rules and
battlefields inline. Consequently some responses grew despite simpler objects.
These are serialized view measurements excluding receipts, not live token counts
or controlled game-time comparisons.

Both pilots praised reliable setup, batching, explicit yield timing, and
retained events. Remaining feedback:

- Repeated printed rules, history, and long references still dominate reading.
- Ability ordinals change when the legal menu changes.
- Native batch mistakes need local diagnostics and concrete examples.
- Generic Chaos Orb selection prompts lack source context and distinct option
  labels. This remains an engine/presentation follow-up.

Private artifacts are under
`target/agent-matches/2026-10-20T08-51-43-888Z-improvement-round1/`.

## Improvement round 2: retained rules and explicit native inputs

Harness: `8bc5a2a0f3d4f746365dadbac6f5b1ed316423d0`; the same release host.
The session remembers successfully delivered printed summaries, marks identical
repeats explicitly, and reprints them after `full:true`. Current state, choices,
and events remain complete. References shrink to 47 opaque characters; ability
identities remain stable across legal menus. `NativePlayChoice` validates native
batch wrappers before submission, and the guide distinguishes them from hosted
ticket batches.

On round one's identical 220 retained observations, median serialized view size
fell from 15,093 bytes to 13,124 with shorter references and stable labels, then
to 10,892 with session rule reuse. Total bytes fell from 3,276,749 to 2,417,960
(26%). This synthetic sequence includes intermediate yield observations that
pilots did not necessarily receive, excludes receipts, and assumes sequential
delivery without a full refresh. It measures presentation changes, not actual
client traffic or game-time speedup.

Two fresh GPT-6 Astra Medium pilots completed the second Sligh mirror. P1 won
18 to −1 on turn 19. Exact replay reproduced all 378 commands and both final
observations. This longer attrition game took about 32 minutes including setup.
All 380 retained seat observations validated under the current typed view.

| Measurement | Round 1 | Round 2 |
| --- | ---: | ---: |
| Accepted mutation requests | 95 | 107 |
| Recorded yield passes | 151 | 312 |
| Manual priority passes | 8 | 4 |
| Client calls | 124 | 159 |
| Client output bytes | 1,527,484 | 1,694,754 |
| Bytes per client call | 12,318 | 10,659 |
| Client errors | 2 | 2 |
| Context compactions, both pilots | 2 | 2 |
| Successful full refreshes | — | 2 |

Round two's yield count includes 275 optional windows and 37 already forced
windows; the engine also performed 119 separate forced continuations. Both
pilots reported no missed intended priority window, output truncation, or lost
gameplay information. Their full refreshes each restored 16 printed definitions
at the same current revision, then normal rule reuse resumed.

The native batch-shape errors from round one did not recur. Both round-two
errors instead came from selecting a decision ticket without separately passing
explicit option IDs. Each returned HTTP 400 without mutation and recovered on
retry. P1 also received one safe accepted-prefix stop: declaring the last eligible
attacker already finished declarations, so a redundant finish action was refused.

The reduction in bytes per call is descriptive: the games had different
positions, request mixes, and lengths. Total output and elapsed time increased
in the longer game. The controlled saved-position comparison supports smaller
views; these games do not establish an end-to-end speedup.

### Remaining priorities after the two rounds

1. **Explicit decision tickets.** Twelve Chaos Orb alternatives shared a generic
   label despite distinct object options. A displayed action already contained
   options, yet `choose` required them again. Show the selected objects and
   explicit-options requirement clearly, or design a concrete-choice operation
   that can submit precisely the displayed selection.
2. **Event delivery cursors.** P2 repeatedly received 74 retained updates spanning
   turns 12–16. A seat-scoped cursor should acknowledge delivery independently
   of manual game actions, preserve repeated events and ordered disclosures, and
   provide an explicit full-history refresh. Avoid heuristic event deduplication.
3. **Descriptive ability and decision context.** Stable fingerprints help identity,
   but labels still need effects such as “animate.” Preserve originating effect
   context when a decision source has left the battlefield; remove duplicate
   target wording.
4. **Large-menu and payment visibility.** A 30-choice menu required an extra
   inspection because paging removes all inline items. Automatic payment also
   consumed both Mountains when an untapped Factory could have preserved red
   mana; explicit preactivation remained available, but a payment preview would
   make that consequence clearer.

Private artifacts are under
`target/agent-matches/2026-10-20T09-19-17-529Z-improvement-round2/`.
Both pilot clients and the host exited cleanly after export. The two rounds
retain all game choices, existing web-client support, and native replay data;
they do not add direct native replay import or live spectating to the browser.

## Latency round 1: combined interactions and measured waiting

Harness and release host: `c5e704ad1`. Two fresh GPT-6 Astra Medium pilots
played another Sligh mirror. `interact` combined already-decided exact choices
with an optional explicit yield; the persistent session renewed bounded
read-only waits and cached the public catalog. Concrete decision labels carried
the chosen option IDs. New counters separated host execution, client HTTP work,
waiting, and the gap before the next pilot request.

P2 won on turn 14, with life totals −2 / 3. Exact replay reproduced all 310
journal commands and both final observations. The host exported at 701.296
seconds after startup (11 minutes 41 seconds). There were 44 interaction
requests, 48 manual choices, 262 yielded passes, two manual priority passes,
and 56 separate forced continuations.

The clients made 51 calls and emitted 561,135 bytes. Neither pilot reported an
error, context compaction, or missed desired response window. Three bounded
waits timed out and resumed through `next`. Both pilots appended a redundant
attack finish once; the already accepted attackers stayed committed, but the
partial batch prevented yield installation and required another interaction.

The first part of this game exposed an orchestration problem: a persistent
PTY's `write_stdin` waited its full requested 30 seconds even when a complete
response was already available. Both pilots observed it, and an isolated
immediate-echo process reproduced a 30.0019-second tool wait. At roughly
393 seconds, terminal collection changed to one-second writes and five-second
empty polls. The harness code stayed unchanged, but this game therefore does
**not** have a fixed orchestration baseline for elapsed-time comparison.
The pilots still reported 27 polling responses without a new decision between
them after counting empty output and timeout responses.

Recorded host command execution totaled 1.967 seconds, including 1.598 seconds
for `interact`. These figures exclude lock acquisition, HTTP long waits,
transport, rendering, model work, and terminal collection. Client wait durations
overlap across seats and must not be added into a match wall-time estimate.
Each client fetched the roughly 21 MB public catalog once.

Feedback drove the next cycle:

- Replace persistent-terminal request collection with finite commands that
  exit as soon as the reply arrives, retaining session context in memory.
- Deliver new events through an explicit seat cursor. P2's upkeep-to-main
  interaction repeated 38 old events alongside three new ones.
- Keep useful choices inline when a menu is large. P1 needed inspections for
  a land and Ball Lightning when 29- and 54-choice menus were entirely paged.
- Trim cached rule rows while preserving explicit references and full refresh.

Private exports, transcripts, feedback, counters, timing correction, and replay
verification are under
`target/agent-matches/2026-10-20T14-52-55-116Z-latency-round1/`.
The host and both clients exited after collection.

## Latency round 2: finite requests and event delivery

Harness and release host: `660dd01be`. A loopback session retains credentials,
catalog, delivered rules, and event cursor in memory. Each agent command is a
finite process, so terminal tools return when its response is ready.
Disconnect recovery restores context; uncertain mutations still require receipt
recovery or identical retries. No mutation retry or gameplay choice is automated.

Seat-local event streams preserve repeated events and private inspections
without heuristic deduplication. Normal delivery contains new events, while
`full:true` reprints history and rules. Canonical observations and their
references remain unchanged. Large menus retain an exact inline prefix plus
typed family summaries pointing to the full menu. Cached rule rows retain their
definition and reference with an explicit previously-delivered marker.

On latency round one's identical 312 retained observations, minimal cached rows
and partial menus reduced synthetic total view bytes from 3,369,060 to 3,279,303
(2.7%). Median size was 10,931 versus 10,631 bytes. All four entirely paged menus
now had inline choices. This comparison excludes receipts and event-cursor
savings, includes intermediate yield observations, and assumes sequential rule
delivery. It is a presentation comparison, not live traffic or game timing.

Two fresh GPT-6 Astra Medium pilots completed the second game with this fixed
harness and orchestration. P1 won 16–0 on turn 17. Exact replay reproduced all
431 commands and both final observations. The host exported at 605.616 seconds
after startup (10 minutes 6 seconds).

| Measurement | Latency round 1 | Latency round 2 |
| --- | ---: | ---: |
| Global turn at completion | 14 | 17 |
| Host elapsed time at export | 701.296 s | 605.616 s |
| Journal commands | 310 | 431 |
| Interaction requests | 44 | 72 |
| Manual choices | 48 | 80 |
| Yielded / manual priority passes | 262 / 2 | 351 / 1 |
| Separate forced continuations | 56 | 29 |
| Client calls | 51 | 74 |
| Client output bytes | 561,135 | 859,497 |
| Bytes per client call | 11,003 | 11,615 |
| Client errors / context compactions | 0 / 0 | 0 / 0 |
| Recorded host execution | 1.967 s | 2.008 s |

The second game needed only one extra gameplay terminal poll. P1's 41 calls and
P2's 33 calls otherwise returned directly from finite request processes. There
were no inspections or surfaced session timeouts; three read-only waits renewed
internally. Both pilots reported no missed desired response window. Each local
session and the host closed cleanly after export.

This is evidence that the transport removes unproductive collection delay,
supported by the isolated PTY reproduction and finite-process tests. The
different deals, positions, game lengths, and first game's mid-run cadence
change prevent a controlled end-to-end speedup claim. Actual total bytes and
bytes per call **increased** in the second game. Byte counts are a token-cost
proxy, not tokenization or billing measurements; live token savings are not
established by these two games.

Remaining feedback is concrete:

1. **Bound presentation size.** P1 still received 4,252 tool-output tokens for
   one combat outcome (`p1-32`); battlefield, graveyard, and rule references
   remain bulky. Its tool output budget truncated `p1-26` and `p1-40`, although
   it reported no lost needed decision information. Attribute bytes by section
   before further compression. Any paged event delivery must acknowledge only
   delivered items, and state deltas must retain explicit full refresh.
2. **Expose payment consequences.** In `p2-21`, animating a second Factory used
   the first intended blocker as a mana source. This was one material pilot
   payment error, with no API failure. Explicitly floating mana was available;
   a preview would make automatic source selection easier to assess.
3. **Disambiguate targets and compound decisions.** Chain Lightning's copy
   required payment, copy confirmation, then target selection (`p2-23`–`p2-25`).
   Three Ironclaw Orcs options had identical labels. The targets happened to be
   equivalent here, but object IDs and source context would avoid inspection
   when they differ. Preserve separate choices when new information matters.
4. **Add precise stack stop conditions.** P1 wanted to stop after its spell
   resolved. `nonManaAction` instead stopped immediately because a Factory
   activation was available (`p1-18`), requiring another interaction. A typed
   stack/object condition could express that intent without skipping responses.

Private evidence is under
`target/agent-matches/2026-10-20T15-16-31-501Z-latency-round2/`.
Both new games retained every legal choice and exact replay compatibility.
The hosted web client remains available for human-versus-agent games.
