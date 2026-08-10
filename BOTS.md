# Writing an AI bot for penta

penta is a deterministic engine for two-player constructed Magic. It currently
ships Eternal Central Old School 93/94 and the final pre-Theros ISD–RTR
Standard format. This guide is for writing a program that plays it: from
Python, C, C++, or Rust, against the included bots or against itself.

This guide describes the current development wire contract, protocol 7. The
engine crate remains version 0.6.0 while these changes are unreleased. Old
School remains the default for compatibility; new integrations should record
and pass an explicit format slug with each game.

A bot is a function from an **observation** (your seat's view of the game,
as JSON) to an **action index** (a position in that observation's
`legalActions` array). The engine validates every index against the legal
list, so an illegal move cannot even be expressed. Everything else —
mulligans, mana payment, combat — arrives as entries in that same list.

The included opponents:

- `random` — picks uniformly among legal actions. The sanity check: if your
  bot cannot beat noise, something is wrong. It plays a real, witless game
  rather than resigning, because nothing a bot can choose ends the game on
  the spot.
- `handcrafted` — a rules-based policy that plays lands on curve, attacks,
  blocks, and answers threats. The first real milestone.

For scale: the engine plays a full `handcrafted` vs `random` game in about
five milliseconds, and the Python loop below runs ~15 games/second single
threaded. Training-scale rollouts are practical on a laptop.

## Quick start: Python

Requires Python 3.9+ and [rustup](https://rustup.rs), which installs the
repository's pinned Rust version automatically. From the repository root:

```bash
cd bindings/penta-py
cargo build --release
cp target/release/libpenta.dylib penta.so   # Linux: cp target/release/libpenta.so penta.so
python3 -c "import penta; print(penta.engine_version())"
```

(With [maturin](https://maturin.rs) installed, `maturin develop --release`
does the copy for you and installs into your virtualenv.)

Then, in a file next to `penta.so`:

```python
import json
import penta

game = penta.Game("Sligh", "The Deck", opponent="handcrafted", seed=42)
while game.result() is None:
    observation = json.loads(game.observe())
    actions = observation["legalActions"]
    choice = actions[0]["index"]          # your bot's decision goes here
                                          # (nothing in the list resigns)
    game.act(choice)
print(game.result())                       # "p1", "p2", or "draw"
```

Old School remains the default for compatibility. Select Standard explicitly:

```python
game = penta.Game(
    "Briksza Naya Midrange",
    "Greer G/R Aggro",
    opponent="external",
    format="isd-rtr-standard",
    seed=42,
)
```

A complete bot that plays lands, casts its biggest spells, and attacks —
and beats `random` 100 games out of 100 — is in
[`examples/python/first_bot.py`](examples/python/first_bot.py). Copy it next
to your built `penta.so` and run it.

The module surface:

| call | meaning |
| --- | --- |
| `penta.Game(p1_deck, p2_deck, opponent=, opponent_seat=, seed=, format=)` | start a game; `format` defaults to `"old-school-93-94"` and `opponent` is `"handcrafted"`, `"random"`, or `"external"` |
| `game.observe(seat=None)` | one seat's observation as JSON (default: the seat that must act) |
| `game.act(index)` | play one entry from `legalActions` |
| `game.choose_decision([ids])` | answer a multi-pick decision explicitly (see below) |
| `game.decision_seat()` | `"p1"` / `"p2"` / `None` when the game is over |
| `game.clone()` | an independent copy of the game — fork it, try a line, discard it |
| `game.hand(seat)`, `game.library(seat)` | a zone's real contents, unredacted — for simulating, not for playing |
| `game.set_hand(seat, defs)`, `game.set_library(seat, defs)` | say what a zone holds, in a fork |
| `game.result()` | `None`, `"p1"`, `"p2"`, or `"draw"` |
| `penta.catalog(format=)` | every canonical definition annotated with legality for the selected format, as JSON |
| `penta.deck_names(format=)` | the selected format's built-in decks |
| `penta.engine_version()`, `penta.protocol_version()` | pin these with your trained weights |

## Quick start: C and C++

```bash
cargo build --release -p penta-ffi
```

produces `target/release/libpenta_ffi.a` (and a shared library). Include
[`bindings/penta-ffi/include/penta.h`](bindings/penta-ffi/include/penta.h)
and link the library; the header documents every call and the ownership
rules. A complete program that plays full games through this interface is
[`bindings/penta-ffi/smoke.c`](bindings/penta-ffi/smoke.c):

```bash
cc mybot.c target/release/libpenta_ffi.a -I bindings/penta-ffi -o mybot
```

The C ABI is the same protocol with the same JSON: `penta_new` takes a
config, including an optional `"format"` slug; `penta_observe_json` returns an
observation; and `penta_act` takes an index. The original catalog and deck-name
functions remain Old School-compatible. New callers can use
`penta_catalog_json_for_format` and `penta_deck_names_for_format_json`.
`penta_legal_action_count` lets a minimal client act without parsing JSON at
all. From C++, wrap the header and parse observations with any JSON library
(e.g. nlohmann/json). Anything else with a C FFI — Julia, Go, C# — can consume
the same library.

## Quick start: Rust

The engine is an ordinary crate. Depend on it by path (or git) and use the
same facade the bindings use:

```rust
use penta::protocol::{BotGame, Opponent};
use penta::{Format, PlayerId};

let mut game = BotGame::new_with_format(
    Format::IsdRtrStandard,
    "Briksza Naya Midrange",
    "Greer G/R Aggro",
    Opponent::Handcrafted,
    PlayerId::Two,
    42,
)?;
while game.result().is_none() {
    let observation = game.observe_json(game.decision_seat().unwrap());
    game.act(0)?; // your bot's index here
}
```

Rust bots can also implement the `penta::Policy` trait directly and skip
JSON entirely; that is how the built-in bots are written.

## Running matches

`penta-match` pits the built-in policies against each other, alternating
seats, with deterministic seeds:

```bash
cargo run --release --bin penta-match -- \
    --p1 random --p2 handcrafted --deck1 Sligh --deck2 "The Deck" \
    --games 100 --seed 1
```

A deck of `Random` (the default) rotates through the built-in list. For
your own bot, the harness in `examples/python/first_bot.py` shows the
pattern: a seed loop, one `penta.Game` per seed, win counting.

## Self-play

`opponent="external"` disables the built-in opponent entirely: the game
stops at **every** decision, and `decision_seat()` tells you whose it is.
One loop drives both sides — your current model against a frozen
checkpoint, or against another author's bot:

```python
game = penta.Game("Goblins", "White Weenie", opponent="external", seed=7)
while game.result() is None:
    seat = game.decision_seat()
    observation = json.loads(game.observe(seat))
    bot = my_model if seat == "p1" else frozen_model
    game.act(bot(observation))
```

Observations are per-seat and redacted — `p1`'s observation never contains
`p2`'s hand — so neither side of a self-play loop can accidentally peek.

Search bots build on the same surface plus one call: `game.clone()`
(`penta_clone` in C, plain `.clone()` on a Rust `BotGame`) forks a game
mid-state into an independent copy, the built-in opponent's state included.
Fork at a decision, roll each candidate action out to the end, play the
winner in the real game — the clone and the original never disturb each
other, and a clone fed the same indices replays byte-identically.


### Rolling out against worlds you cannot see

A clone forks the *true* state, hidden zones included. For self-play training
that is exactly right. For a search bot choosing a move in a real match it is
not: rollouts on the true world are influenced by cards the searcher has not
seen, so the outcomes it measures encode information it does not have.

The fix is to search over worlds consistent with what your seat actually
knows. You do not know their last card — it could be a Lightning Bolt, it
could be a Counterspell — so build both worlds and roll each one out:

```python
catalog = {c["name"]: c["definition"] for c in json.loads(penta.catalog())["cards"]}

for guess in ("Lightning Bolt", "Counterspell"):
    world = game.clone()
    world.set_hand("p2", [catalog["Mountain"], catalog[guess]])
    # ... roll this world out and score it
```

`set_hand` and `set_library` say what a zone *holds*, by card definition. The
cards are built fresh, so you are stating a hypothesis rather than shuffling
the real one, and nothing is conserved: stack a library, empty it, or hand
someone a card that was never in their deck. A world you invented has no
reason to balance.

`hand(seat)` and `library(seat)` read the zones back as
`[{objectId, definition}]`, unredacted, so you can see the true state you are
replacing and weight your guesses however you like. The engine ships no
sampler — a uniform re-deal, a weighting by what the opponent has cast, and a
belief filter maintained across turns are all just different lists of
definitions.

Two things to know. Rewritten cards get new object IDs, so rewrite an
opponent's zones rather than your own if you are holding IDs from an earlier
observation. And these accessors are not redacted, which is not a hole in
match secrecy: a tournament server hands a bot redacted observations over a
wire and never a game object, so transparency here only reaches someone
simulating in their own process, where there is nobody to hide from.

## The observation

`observe()` returns one JSON object. The essential fields:

| field | meaning |
| --- | --- |
| `format` | the rules/deck profile slug, such as `"old-school-93-94"` or `"isd-rtr-standard"` |
| `seat` | whose view this is: `"p1"` or `"p2"` |
| `pregame` | true while mulligans are being settled |
| `turn`, `activeSeat`, `prioritySeat`, `step` | where the game is; `step` is one of `Upkeep`, `Draw`, `PrecombatMain`, `BeginningOfCombat`, `DeclareAttackers`, `DeclareBlockers`, `CombatDamage`, `EndOfCombat`, `PostcombatMain`, `End`, `Cleanup` |
| `regularCombatDamagePending` | true during the priority window after first-strike damage and before regular combat damage; both damage waves otherwise use `step: "CombatDamage"` |
| `life`, `manaPools`, `librarySizes` | two-element arrays, indexed p1 then p2 |
| `hand` | your cards: `{instance, definition, name}` |
| `opponentHandSize` | their hand as a count — never the cards |
| `battlefield` | every permanent, including its current-zone object ID, canonical definition, and presented card-part ID |
| `stack` | pending spells, activated abilities, and triggered abilities, bottom to top, including each object's frozen source, ability origin/text, targets, and locked cast signature when applicable |
| `graveyards`, `exiles` | public zones, both players |
| `decision` | a pending choice (see below), or null |
| `result` | null while running, else `{winner, reason}` |
| `legalActions` | what you can do, each with an `index` |

Cards are referenced two ways: the object ID identifies one rules object in
its current zone, while `definition` identifies the canonical card kind and is
the key into `penta.catalog(format)`. A true zone change creates a new object
ID, so a Goblin Balloon Brigade card in hand, its spell on the stack, and its
permanent on the battlefield are distinct. Transforming, flipping, and phasing
do not create a new object. Physical-card lineage is private engine state and
never appears in a player's observation. Fetch the format's catalog once at
startup.

### Actions

Every entry in `legalActions` has an `index` (what you pass to `act`) and a
`type` naming the engine action, plus fields saying what it operates on:

`KeepHand`, `TakeMulligan`, `BottomCards`, `PlayLand` (with a
`playOptionId`), `CastSpell` (with the play option, ordered modes, cost
configuration, target slots, sacrifices, and X already filled in — one entry
per legal casting choice), `ActivateAbility`, `ActivateManaAbility`, `PayLifeForMana`,
`DeclareAttacker`, `FinishDeclaringAttackers`, `DeclareBlocker`,
`FinishDeclaringBlockers`, `AssignCombatDamage`, `DiscardCards`,
`ChooseUntap`, `ChooseDecision`, `CancelDecision`, `PassPriority`.

Ability actions identify the exact clause being used in an `ability` object.
Its `kind` determines the rest of its provenance:

- `printed` carries the canonical `definition`, positional `partId`, and
  positional `abilityId`.
- `intrinsicBasicLand` carries the lowercase `landType` whose rules supplied
  the ability.
- `granted` carries the granting `source` object together with
  `sourceDefinition`, `sourcePartId`, `sourceAbilityId`, and `grantId`.

`ActivateAbility` also carries `targetSelections`, the flattened `targets`,
and a compatibility `target` containing the first selected target. Targets are
chosen before the activated ability becomes an independent stack object.
`ActivateManaAbility` uses the same origin vocabulary but resolves immediately
because mana abilities never use the stack. The engine does not infer that
classification merely because an effect happens to produce mana.

Three things worth knowing:

- **Nothing in the list loses on the spot.** Conceding is legal in every
  state of Magic, but it is strictly dominated for a bot — resigning can
  only lose a game that playing on might win — so it is not offered here at
  all. Picking blindly, by index or at random, makes a weak bot rather than
  an instant loss. (Humans concede through the browser client, which reads
  the engine's own action list.)

- **Mana is handled for you.** If a `CastSpell` appears in `legalActions`,
  you can afford it; playing it taps lands automatically. Tapping lands by
  hand (`ActivateManaAbility`) exists but is never required.
- **Costs and targets are enumerated.** A Lightning Bolt with three legal
  targets appears as three `CastSpell` entries. Your bot chooses among
  ready-made legal plays; it never constructs one.

### Decisions

Every decision has a `kind`:

- `Choice` asks an ordinary question during costs or resolution — "copy Chain
  Lightning?", "choose a card to return", and so on.
- `TriggerOrder` asks a player to order simultaneous triggers. Each option has
  a `triggerId` and frozen `abilityText`; `orderSemantics: "resolution"` means
  the submitted list is first-resolving-first, even though the stack itself is
  displayed bottom-to-top.
- `TriggerPlacement` asks for targets while one triggered ability is being put
  on the stack. Every player orders and targets their own triggers, in
  active-player/nonactive-player placement order, before priority returns.

These arrive as a `decision` object (prompt, options,
`minimum`/`maximum` counts) *and* as `ChooseDecision` entries in
`legalActions`: a pick-exactly-one decision becomes one indexed action per
option, so an index-only bot handles it like anything else. For a
pick-several decision, `legalActions` carries one default selection (the first
`minimum` options) and `choose_decision([option_ids])` submits any other
selection you'd prefer.

Catalog cards and parts expose clause-derived `implementationStatus` as
`complete`, `partial`, or `metadataOnly`. The old execution gate is not public
coverage metadata. A card with no printed mana cost has `"manaCost": null`;
a printed `{0}` has a mana-cost object whose `generic` field is zero.

## Determinism and versioning

The same engine version, format, decks, seed, and action sequence produce the
identical game, byte for byte — replays, regression tests, and reproducible
training episodes are free. `(engine version, format, seed, decks, action
list)` is a complete record of a game.

Two numbers describe what you trained against, and both are worth pinning
alongside your weights:

- `protocol_version()` covers the JSON shapes and the action space they
  describe. It bumps when a bot written against the old number could
  misread the new output — including a change to what appears in
  `legalActions`, since that shifts every index.
- `engine_version()` covers rules behavior, which is part of the contract
  too: a rules fix can change what a trained policy sees even when the
  shapes hold still.

[CHANGELOG.md](CHANGELOG.md) records what moved between versions and what a
bot has to do about it. Before 1.0, expect the action space to keep
settling — reading the `type` tags rather than hardcoding indices costs
nothing now and survives those changes.

## What the engine covers, honestly

Old School 93/94 has 128 cards with functional behavior and fifteen built-in
decks; the Eternal Central banned/restricted list and mana-burn exception are
enforced. ISD–RTR Standard adds the eight SCG Atlanta Top 8 decks and complete
declarative card records. Baseline creatures, mana, land entry, flash, and
combat metadata are active while specialized Standard card effects are being
implemented incrementally; metadata-only noncreature spells are withheld from
legal actions rather than resolving as silent no-ops. `penta.catalog(format)`
is the authoritative description of the selected format's support.

What this is *not*: a complete Magic rules engine. Cards outside the catalog
do not exist here, and custom decks beyond the twenty-three built-ins are not
yet exposed through the protocol. Interactions are implemented to the depth
the supported tranche requires and are covered by the engine's test suite and
long random self-play sweeps — but a trained bot will probe every edge, and if
you find behavior that contradicts the printed cards, that is a bug worth
filing.

## Where this is going

The protocol you train against locally is the protocol a future tournament
server will speak over a socket: same observations, same indices, with the
authoritative engine on the server and your bot dialing in from your own
hardware. Nothing about a bot written today needs to change for that.
