# Premodern implementation roadmap

Penta's first Premodern tranche is the Top 8 of the 109-player [Sacred Torch
Showdown 2026][tournament], played on July 25, 2026. The submitted lists are
staged under `decks/premodern/` and covered by a repository test, but they are
not exposed as built-in playable decks yet. Publishing them before their cards
have honest execution coverage would create legal actions that cannot resolve
correctly.

## Snapshot

- 8 complete submitted main decks and 15-card sideboards captured (seven have
  60 cards; Drew Glauberg's Stasis list has 61)
- 145 distinct cards across the tranche, 25 of them not yet cataloged
- 5 lists registered and playable: Neal Sacks's Sligh, Daniel Sondike's GAT,
  Drew Glauberg's Stasis, TentacleFan's Landstill, and Andy Dominguez's RG
  Goblins. Nothing in any of them is metadata-only or partial
- per-card catalog and interaction-audit status tracked in the inventory below

What each remaining list is waiting on, counting main deck and sideboard
together: BW Control 10, Replenish 11, Angry Hermit 12. A list is blocked only by uncataloged cards -- no staged deck
contains a card that is cataloged but unplayable.

The eight staged decks, in finish order, are Neal Sacks's Sligh, Daniel
Sondike's GAT, Bryan Gulotta's Replenish, Drew Glauberg's Stasis, Chris Danis's
BW Control, TentacleFan's Landstill, Andy Dominguez's RG Goblins, and Ryan
Marvin's Angry Hermit.

## Format profile

`Format::Premodern` is in place: the twenty-nine-set window from Fourth
Edition through Scourge, the format's own thirty-three-card ban list, no
restricted list, and contemporary mana rules. All three are taken from the
[Premodern rules page][rules]. `CardSet` gained the fifteen sets in the window
it was missing, so the window can be stated in full even where no card has
been authored from a set yet.

The format is offered in the web client, and its picker lists exactly the
decks the engine has registered. Whole-game coverage matches the other two
formats: the deferred sweeps play every registered Premodern matchup to a
result and rebuild sampled Premodern positions from their observations.

## Remaining format work

- Add the missing printed-set modules and canonical printings. Definition IDs
  remain append-only even when a card belongs to an older set.
- Add accurate characteristics and Oracle clauses for uncataloged cards.
  Unsupported clauses must be metadata-only rather than executable no-ops.
- Implement reusable mechanics before card-local behavior. Cycling and
  typecycling, flashback, split cards, tutors, alternative costs that exile a
  card from hand, and single-card reanimation are all in place. What the
  remaining lists still need: fading (Parallax Wave); morph (Exalted Angel);
  and a mass graveyard-to-battlefield return (Replenish), plus the layer
  rewrite Opalescence asks for. Storm landed with Brain Freeze, along with a
  spell's own cast trigger and a copy chain; countering an ability landed
  with Teferi's Response, so Stifle needs only its own authoring; and the
  Stasis tranche added payments that return or sacrifice a named permanent
  and an additional cost counted in X. Naming a card and reading the name back,
  arranging the top of a library, spending a land's counters, and a payment
  that discards a card matching a predicate all landed with the GAT tranche.
- Audit the existing definitions against their Premodern Oracle text and
  interactions.
- Promote each staged deck into the runtime registry only when every main-deck
  card is playable and the sideboard has honest catalog coverage. GAT, RG
  Goblins, and Sligh are registered; the other five lists remain staged.
  Registration is also what first checks a list against the format's set
  window: promoting GAT found seven of its cards cataloged only from
  printings outside it, Landstill four more, and Stasis three.
- Keep the web deck notes in step with the registry as lists are promoted;
  `web/app/game-config.ts` and the deck names in `src/protocol/decks.rs` are
  checked against each other by the browser contract suite.

## Card inventory

Already cataloged (status annotations record the completed interaction audits;
older unannotated definitions still require one):

- `Adarkar Wastes` — complete
- `Akroma's Vengeance` — complete
- `Ancient Tomb` — complete
- `Annul` — complete
- `Arcane Denial` — complete; both draws wait a turn
- `Armageddon`
- `Aura of Silence` — complete
- `Barbarian Ring` — complete
- `Black Vise`
- `Blue Elemental Blast`
- `Brain Freeze` — complete; storm copies what came before it
- `Caves of Koilos` — complete
- `Cephalid Coliseum` — complete
- `Chain of Vapor` — complete; the chain is the opponent's to continue
- `Chill` — complete
- `Circle of Protection: Red` — complete; Fourth Edition brings it inside the window
- `City of Brass`
- `Claws of Gix` — complete
- `Coastal Tower` — complete
- `Counterspell`
- `Cursed Scroll` — complete; naming a card is modelled as naming one held
- `Cursed Totem` — complete
- `Daze` — complete
- `Decree of Justice` — complete; cycling buys Soldiers by the mana
- `Disenchant`
- `Duress`
- `Dust Bowl` — complete
- `Earthquake`
- `Engineered Plague` — complete
- `Enlightened Tutor` — complete
- `Eternal Dragon` — complete
- `Fact or Fiction` — complete
- `Fire // Ice` — complete
- `Fireblast` — complete
- `Flash of Insight` — complete; the flashback exiles X blue cards
- `Flooded Strand` — complete
- `Forest`
- `Forsaken City` — complete
- `Gempalm Incinerator` — complete
- `Gemstone Mine` — complete
- `Goblin Lackey` — complete
- `Goblin Matron` — complete
- `Goblin Patrol` — complete
- `Goblin Piledriver` — complete
- `Goblin Pyromancer` — complete
- `Goblin Ringleader` — complete
- `Goblin Sharpshooter` — complete
- `Goblin Tinkerer` — complete
- `Goblin Vandal` — complete
- `Goblin Warchief` — complete
- `Grim Lavamancer` — complete
- `Gush` — complete
- `Humility` — complete
- `Hydroblast` — complete
- `Impulse` — complete
- `Incinerate` — complete; the rider follows the damage, not the target
- `Island`
- `Jackal Pup` — complete
- `Karplusan Forest` — complete
- `Kor Haven` — complete
- `Lightning Bolt`
- `Llanowar Wastes` — complete
- `Lotus Petal` — complete
- `Mana Leak` — complete
- `Mana Short` — complete
- `Meddling Mage` — complete; the lock is symmetric and leaves with it
- `Mishra's Factory`
- `Mogg Fanatic` — complete
- `Mogg Salvage` — complete
- `Monk Realist` — complete
- `Mountain`
- `Mox Diamond` — complete; an unpaid entry is replaced, not undone
- `Naturalize` — complete
- `Opt` — complete
- `Overload` — complete
- `Phyrexian Arena` — complete
- `Phyrexian Furnace` — complete; the tap mode eats the oldest card
- `Plains`
- `Portent` — complete; the arrangement is the order the cards are named
- `Powder Keg` — complete
- `Presence of the Master` — complete
- `Prohibit` — complete
- `Psychatog` — complete
- `Pyroblast` — complete
- `Pyrokinesis` — complete
- `Quirion Dryad` — complete
- `Ray of Revelation`
- `Red Elemental Blast`
- `Reanimate` — complete
- `Rishadan Port` — complete
- `Root Maze` — complete
- `Seal of Cleansing` — complete
- `Seal of Fire` — complete
- `Secluded Steppe` — complete
- `Siege-Gang Commander` — complete
- `Skirk Prospector` — complete
- `Sleight of Hand` — complete
- `Standstill` — complete
- `Stasis` — complete
- `Swamp`
- `Swords to Plowshares`
- `Syncopate`
- `Sylvan Safekeeper` — complete
- `Teferi's Response` — complete; the countered ability's source dies with it
- `Thawing Glaciers` — complete; the return is a cleanup-step trigger
- `Thwart` — complete
- `Tormod's Crypt` — complete
- `Tranquil Domain` — complete
- `Treva's Ruins` — complete
- `Tsabo's Web` — complete
- `Underground River` — complete
- `Upheaval` — complete
- `Vindicate` — complete
- `Volcanic Hammer` — complete
- `Warmth` — complete
- `Wasteland` — complete
- `Wooded Foothills` — complete
- `Worldly Tutor` — complete
- `Wrath of God`
- `Yavimaya Coast` — complete

Not yet cataloged:

- [ ] `Abeyance`
- [ ] `Attunement`
- [ ] `Cabal Therapy`
- [ ] `Decree of Silence`
- [ ] `Defense Grid`
- [ ] `Dragon Breath`
- [ ] `Exalted Angel`
- [ ] `Frantic Search`
- [ ] `Gerrard's Verdict`
- [ ] `Gilded Drake`
- [ ] `Haunting Echoes`
- [ ] `Hermit Druid`
- [ ] `Intuition`
- [ ] `Krosan Reclamation`
- [ ] `Opalescence`
- [ ] `Parallax Wave`
- [ ] `Phyrexian Dreadnought`
- [ ] `Reflecting Pool`
- [ ] `Replenish`
- [ ] `Shallow Grave`
- [ ] `Skeletal Scrying`
- [ ] `Skycloud Expanse`
- [ ] `Stifle`
- [ ] `Sutured Ghoul`
- [ ] `Vision Charm`

[tournament]: https://melee.gg/Tournament/View/441083
[rules]: https://premodernmagic.com/
