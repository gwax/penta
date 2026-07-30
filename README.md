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

## Current scope

The initial slice validates decks, creates deterministically shuffled games,
draws opening hands, exposes player-safe observations, and handles concession.
The stack, priority, turn progression, combat, mana, and card effects are the
next engine layers.

## Proof-of-concept card corpus

The first playable target is a representative unpowered Mono-Red Atog deck
playing a mirror match. Its only non-red, non-artifact card is Mountain. The
list is adapted from the [Atog Unpowered Compendium][atog-list], replacing one
Strip Mine with a sixteenth Mountain. The main deck and sideboard together
require 20 distinct cards:

- Mountain
- Atog, Ball Lightning, and Stone Giant
- Lightning Bolt, Chain Lightning, Fireball, Fork, Detonate, Shatter, and Red
  Elemental Blast
- Blood Moon and Smoke
- Ankh of Mishra, Black Vise, Copper Tablet, Glasses of Urza, Iron Star,
  Su-Chi, and Winter Orb

This corpus is intentionally based on cards in an actual archetype rather than
all legal red cards. A card joins the implementation target when a deck we
want to simulate requires it.

```sh
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

[ec-rules]: https://www.eternalcentral.com/9394rules/
[atog-list]: https://tappedout.net/mtg-decks/os9394-atog-unpowered-compendium/
