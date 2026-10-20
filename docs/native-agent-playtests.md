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
