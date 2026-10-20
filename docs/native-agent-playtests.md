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
