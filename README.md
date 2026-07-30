# osarena

`osarena` is a deterministic, headless simulator for two-player Old School
Magic: The Gathering. The first target is bot-vs-bot play and, eventually,
high-throughput training through Python bindings.

## Format

The engine targets **Eternal Central Old School 93/94** as one fixed format.
That means:

- Alpha, Beta, Unlimited, Collector's Edition, International Collector's
  Edition, Arabian Nights, Antiquities, Revised, Legends, The Dark, Fallen
  Empires, and the three 1994 promotional cards
- the Eternal Central banned and restricted list
- 60-card minimum decks and sideboards of up to 15 cards
- the London mulligan
- current Magic rules except where Eternal Central explicitly differs,
  notably mana burn

Paper-only policies such as which physical reprints are acceptable have no
meaning in the simulator.

The canonical format reference is [Eternal Central's 93/94 rules][ec-rules].

The format is intentionally not configurable. If another Old School variant
is added later, it should be a distinct format implementation rather than a
bag of switches spread throughout the engine.

## Engine principles

- Game state changes only through explicit actions.
- All randomness comes from a recorded seed and a versioned PRNG.
- Cards have stable numeric definition IDs and per-game instance IDs.
- A player's observation cannot expose an opponent's hidden cards.
- Legal actions are enumerated by the engine.
- The core crate has no UI, network, async runtime, or training dependencies.

See [the engine design notes](docs/engine.md) for state-machine invariants and
extension boundaries.

## Current scope

The engine currently supports:

- deck validation and seeded, reproducible setup
- hidden-information-safe observations and deterministic legal actions
- the priority-bearing turn skeleton, active player, and priority passing
- the stack and last-in-first-out spell resolution
- basic and nonbasic land plays, red and colorless mana sources, and EC
  phase-boundary mana burn
- player damage, concession, and empty-library loss conditions
- public battlefield, graveyard, and stack observations
- an authoritative event log for replay and debugging consumers
- London mulligans and player-selected cleanup discards
- staged attacker and blocker declaration, player-selected combat damage
  assignment, and trample
- summoning sickness, haste, temporary modifiers, marked damage, and death
- red and colorless mana, generic and variable-X costs, and mana burn
- multi-target spells, copy retargeting, activated and triggered choices, and
  restricted untaps
- functional behavior metadata and execution for all 40 catalog cards
- fixed 60-card `Goblins`, `Sligh`, and `Artifacts` decks with 15-card
  sideboards
- a small bot API with seeded random and card-aware handcrafted policies

The event log is intentionally omniscient and must not be passed directly to a
bot; bots consume `PlayerObservation`.

The POC is playable end to end, but it is not yet a general implementation of
the Comprehensive Rules. Fireball supports its multi-target additional cost
and even damage division, Fork can choose new targets for its copy, and combat
damage uses the current player-selected assignment rules. Non-mana activated
abilities generally resolve atomically, while Chaos Orb uses the stack so its
source can be removed in response. Simple upkeep/entry triggers still resolve
atomically. These constraints are explicit extension points rather than silent
support for cards outside the POC.

## Built-in decks

The proof of concept contains three powered, mono-red EC archetypes:

- `poc::goblins()` is a tribal aggro deck built around Goblin King, Goblin
  Grenade, Goblin Balloon Brigade, and Goblins of the Flarg.
- `poc::sligh()` is a curve-based aggro/burn deck with Ironclaw Orcs, Ball
  Lightning, Granite Gargoyle, Dragon Whelp, and direct damage.
- `poc::artifacts()` is Atog Smash, using Atog, Orcish Mechanics, Black Vise,
  Ankh of Mishra, Copper Tablet, and fast artifact mana.

Their cores are based on the [TC Decks Goblins aggregate][goblins-data], the
[Wak-Wak Sligh archetype guide][sligh-guide], and a representative
[EC Atog Smash list][atog-list].

All three use some combination of Mishra's Factory, Strip Mine, Black Lotus,
Mox Ruby, Wheel of Fortune, Chaos Orb, and Sol Ring. The artifact deck also
uses the off-color Moxen as generic-mana sources. In this red-only corpus their
colored output is represented as colorless mana, which is strategically
equivalent for every implemented cost.

EC Chaos Orb normally uses a physical dexterity flip. The headless simulator
instead treats a resolving Orb activation as a deterministic successful flip
against the chosen permanent. The activation uses the stack, and removing the
Orb before resolution nullifies the flip. This keeps seeded games reproducible
and makes the format playable without modeling a human motor skill.

This corpus is intentionally based on cards in an actual archetype rather than
all legal red cards. A card joins the implementation target when a deck we
want to simulate requires it.

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

The gauntlet uses mirror matches for all three built-in decks, isolating policy
quality from deck strength.

```sh
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

[ec-rules]: https://www.eternalcentral.com/9394rules/
[goblins-data]: https://www.tcdecks.net/archetype.php?archetype=Goblins&format=Old+School&src=all
[sligh-guide]: https://www.wak-wak.se/9394decks/sligh
[atog-list]: https://tappedout.net/mtg-decks/atog-smash-9394-1/
