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
- 145 distinct cards across the tranche
- 27 cards present in Penta's catalog
- 118 cards still need catalog records
- 7 cards have completed their Premodern interaction audit; the other 138
  still need one even when an older format already exercises the card

The eight staged decks, in finish order, are Neal Sacks's Sligh, Daniel
Sondike's GAT, Bryan Gulotta's Replenish, Drew Glauberg's Stasis, Chris Danis's
BW Control, TentacleFan's Landstill, Andy Dominguez's RG Goblins, and Ryan
Marvin's Angry Hermit.

## Remaining format work

- Add the `Premodern` format profile and its legal set window, from Fourth
  Edition through Scourge. Premodern uses contemporary game rules and its own
  maintained banned list; take both from the [Premodern rules page][rules] at
  implementation time.
- Add the missing printed-set modules and canonical printings. Definition IDs
  remain append-only even when a card belongs to an older set.
- Add accurate characteristics and Oracle clauses for the 118 uncataloged
  cards. Unsupported clauses must be metadata-only rather than executable
  no-ops.
- Implement reusable mechanics before card-local behavior: cycling and
  landcycling, flashback, threshold, fading, echo, alternative costs, split
  cards, graveyard replacement/reanimation, named-card choices, tutors,
  continuous type/PT changes, and triggered mana/untap restrictions.
- Audit the existing definitions against their Premodern Oracle text and
  interactions.
- Promote each staged deck into the runtime registry only when every main-deck
  card is playable and the sideboard has honest catalog coverage.
- Add protocol/binding documentation and UI format/deck selection when the
  profile becomes public.

## Card inventory

Already cataloged (27; the seven marked below were audited in the first
implementation tranche, while the older definitions still require an audit):

- `Armageddon`
- `Black Vise`
- `Blue Elemental Blast`
- `City of Brass`
- `Counterspell`
- `Disenchant`
- `Duress`
- `Earthquake`
- `Forest`
- `Hydroblast` — complete
- `Incinerate` — damage complete; no-regeneration rider is partial
- `Jackal Pup` — complete
- `Island`
- `Lightning Bolt`
- `Mishra's Factory`
- `Mogg Fanatic` — complete
- `Mountain`
- `Naturalize` — complete
- `Plains`
- `Pyroblast` — complete
- `Ray of Revelation`
- `Red Elemental Blast`
- `Seal of Fire` — complete
- `Swamp`
- `Swords to Plowshares`
- `Syncopate`
- `Wrath of God`

Not yet cataloged (118):

- [ ] `Abeyance`
- [ ] `Adarkar Wastes`
- [ ] `Akroma's Vengeance`
- [ ] `Ancient Tomb`
- [ ] `Annul`
- [ ] `Arcane Denial`
- [ ] `Attunement`
- [ ] `Aura of Silence`
- [ ] `Barbarian Ring`
- [ ] `Brain Freeze`
- [ ] `Cabal Therapy`
- [ ] `Caves of Koilos`
- [ ] `Cephalid Coliseum`
- [ ] `Chain of Vapor`
- [ ] `Chill`
- [ ] `Circle of Protection: Red`
- [ ] `Claws of Gix`
- [ ] `Coastal Tower`
- [ ] `Cursed Scroll`
- [ ] `Cursed Totem`
- [ ] `Daze`
- [ ] `Decree of Justice`
- [ ] `Decree of Silence`
- [ ] `Defense Grid`
- [ ] `Dragon Breath`
- [ ] `Dust Bowl`
- [ ] `Engineered Plague`
- [ ] `Enlightened Tutor`
- [ ] `Eternal Dragon`
- [ ] `Exalted Angel`
- [ ] `Fact or Fiction`
- [ ] `Fire // Ice`
- [ ] `Fireblast`
- [ ] `Flash of Insight`
- [ ] `Flooded Strand`
- [ ] `Forsaken City`
- [ ] `Frantic Search`
- [ ] `Gempalm Incinerator`
- [ ] `Gemstone Mine`
- [ ] `Gerrard's Verdict`
- [ ] `Gilded Drake`
- [ ] `Goblin Lackey`
- [ ] `Goblin Matron`
- [ ] `Goblin Patrol`
- [ ] `Goblin Piledriver`
- [ ] `Goblin Pyromancer`
- [ ] `Goblin Ringleader`
- [ ] `Goblin Sharpshooter`
- [ ] `Goblin Tinkerer`
- [ ] `Goblin Vandal`
- [ ] `Goblin Warchief`
- [ ] `Grim Lavamancer`
- [ ] `Gush`
- [ ] `Haunting Echoes`
- [ ] `Hermit Druid`
- [ ] `Humility`
- [ ] `Impulse`
- [ ] `Intuition`
- [ ] `Karplusan Forest`
- [ ] `Kor Haven`
- [ ] `Krosan Reclamation`
- [ ] `Llanowar Wastes`
- [ ] `Lotus Petal`
- [ ] `Mana Leak`
- [ ] `Mana Short`
- [ ] `Meddling Mage`
- [ ] `Mogg Salvage`
- [ ] `Monk Realist`
- [ ] `Mox Diamond`
- [ ] `Opalescence`
- [ ] `Opt`
- [ ] `Overload`
- [ ] `Parallax Wave`
- [ ] `Phyrexian Arena`
- [ ] `Phyrexian Dreadnought`
- [ ] `Phyrexian Furnace`
- [ ] `Portent`
- [ ] `Powder Keg`
- [ ] `Presence of the Master`
- [ ] `Prohibit`
- [ ] `Psychatog`
- [ ] `Pyrokinesis`
- [ ] `Quirion Dryad`
- [ ] `Reanimate`
- [ ] `Reflecting Pool`
- [ ] `Replenish`
- [ ] `Rishadan Port`
- [ ] `Root Maze`
- [ ] `Seal of Cleansing`
- [ ] `Secluded Steppe`
- [ ] `Shallow Grave`
- [ ] `Siege-Gang Commander`
- [ ] `Skeletal Scrying`
- [ ] `Skirk Prospector`
- [ ] `Skycloud Expanse`
- [ ] `Sleight of Hand`
- [ ] `Standstill`
- [ ] `Stasis`
- [ ] `Stifle`
- [ ] `Sutured Ghoul`
- [ ] `Sylvan Safekeeper`
- [ ] `Teferi's Response`
- [ ] `Thawing Glaciers`
- [ ] `Thwart`
- [ ] `Tormod's Crypt`
- [ ] `Tranquil Domain`
- [ ] `Treva's Ruins`
- [ ] `Tsabo's Web`
- [ ] `Underground River`
- [ ] `Upheaval`
- [ ] `Vindicate`
- [ ] `Vision Charm`
- [ ] `Volcanic Hammer`
- [ ] `Warmth`
- [ ] `Wasteland`
- [ ] `Wooded Foothills`
- [ ] `Worldly Tutor`
- [ ] `Yavimaya Coast`

[tournament]: https://melee.gg/Tournament/View/441083
[rules]: https://premodernmagic.com/
