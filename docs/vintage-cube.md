# Vintage Cube implementation roadmap

The MTGO Vintage Cube is a 534-card singleton list, recorded verbatim in
[`src/format/vintage_cube.rs`](../src/format/vintage_cube.rs) as it stood on
2026-08-19. A cube is re-tuned between runs, so the pool is a dated snapshot
rather than a claim about what is current.

`Format::VintageCube` takes its legality from that list. This is the first
format here that is not a set window: a card is legal because the cube names
it, not because of where it was printed. Six names in the retrieved list match
no card Scryfall knows and were left out; the module records which.

## Snapshot

- 534 cards in the pool, of which 151 are cataloged and 383 are not
- The pool spans sets Penta has never touched, so most of the backlog needs a
  printed-set module before the card itself
- No decks are registered yet. `deck_names_for_format` returns nothing for the
  cube, so it is not offered in the web client
- Drafting is deferred. The engine has no draft, and the plan is to reach a
  playable pool first and play fixed lists from it

## Format profile

Forty-card minimum, one copy of each card, twenty life, seven-card opening
hand, contemporary mana rules, and no ban or restricted list -- a card is
either in the pool or it is not. `FormatRules::card_pool` carries the list, and
`Format::allows_card` consults it instead of `allowed_sets`, which the cube
leaves empty so nothing reads it as a set window by accident.

## Not yet cataloged

Grouped by color so a tranche can be scoped to one part of the pool. Basic
lands are legal in every format and are not listed.

### White (46)

- `Adeline, Resplendent Cathar`
- `Cathar Commando`
- `Containment Priest`
- `Cosmogrand Zenith`
- `Council's Judgment`
- `Descendant of Storms`
- `Eagles of the North`
- `Elite Spellbinder`
- `Elspeth, Knight-Errant`
- `Elspeth, Storm Slayer`
- `Enduring Innocence`
- `Ephemerate`
- `Flickerwisp`
- `Get Lost`
- `Giver of Runes`
- `Glimmer Lens`
- `Guide of Souls`
- `Jacked Rabbit`
- `Leyline Binding`
- `Lion Sash`
- `Loran of the Third Path`
- `Luminarch Aspirant`
- `Ocelot Pride`
- `Oust`
- `Overlord of the Mistmoors`
- `Palace Jailer`
- `Path to Exile`
- `Phelia, Exuberant Shepherd`
- `Portable Hole`
- `Prismatic Ending`
- `Reprieve`
- `Serra Paragon`
- `Skyclave Apparition`
- `Solitude`
- `Staff of the Storyteller`
- `Static Prison`
- `Stoneforge Mystic`
- `Sunfall`
- `The Wandering Emperor`
- `Thraben Inspector`
- `Touch the Spirit Realm`
- `Unexpectedly Absent`
- `Virtue of Loyalty`
- `Voice of Victory`
- `Winds of Abandon`
- `Witch Enchanter`

### Blue (54)

- `Abhorrent Oculus`
- `Astrologian's Planisphere`
- `Brainstorm`
- `Brainsurge`
- `Brazen Borrower`
- `Consider`
- `Consult the Star Charts`
- `Cryptic Command`
- `Displacer Kitten`
- `Duelist of the Mind`
- `Echo of Eons`
- `Emry, Lurker of the Loch`
- `Faerie Mastermind`
- `Flash`
- `Force of Negation`
- `Force of Will`
- `Forensic Gadgeteer`
- `Gitaxian Probe`
- `Hullbreacher`
- `Jace, Vryn's Prodigy`
- `Jace, Wielder of Mysteries`
- `Jace, the Mind Sculptor`
- `Kappa Cannoneer`
- `Kitsa, Otterball Elite`
- `Ledger Shredder`
- `Lose Focus`
- `Lórien Revealed`
- `Malcolm, Alluring Scoundrel`
- `Memory Lapse`
- `Mystic Confluence`
- `Narset, Parter of Veils`
- `Occult Epiphany`
- `Paradoxical Outcome`
- `Phantasmal Image`
- `Phyrexian Metamorph`
- `Plagon, Lord of the Beach`
- `Ponder`
- `Proft's Eidetic Memory`
- `Quantum Riddler`
- `Remand`
- `Show and Tell`
- `Sink into Stupor`
- `Stock Up`
- `Stormchaser's Talent`
- `Subtlety`
- `Thassa's Oracle`
- `Thieving Skydiver`
- `Thundertrap Trainer`
- `Time Spiral`
- `Tinker`
- `Tishana's Tidebinder`
- `Treasure Cruise`
- `Trinket Mage`
- `Urza, Lord High Artificer`

### Black (51)

- `Animate Dead`
- `Archon of Cruelty`
- `Baleful Mastery`
- `Barrowgoyf`
- `Bitter Triumph`
- `Bloodchief's Thirst`
- `Bolas's Citadel`
- `Bone Shards`
- `Cabal Ritual`
- `Caustic Bronco`
- `Chain of Smog`
- `Collective Brutality`
- `Concealing Curtains`
- `Corpse Dance`
- `Crabomination`
- `Cut Down`
- `Damn`
- `Dark Confidant`
- `Dauthi Voidwalker`
- `Deep-Cavern Bat`
- `Dismember`
- `Doomsday`
- `Emperor of Bones`
- `Exhume`
- `Fatal Push`
- `Grave Titan`
- `Grief`
- `Harvester of Misery`
- `Imperial Seal`
- `Infernal Grasp`
- `Inquisition of Kozilek`
- `Metamorphosis Fanatic`
- `Necromancy`
- `Nethergoyf`
- `Night's Whisper`
- `Orcish Bowmasters`
- `Overlord of the Balemurk`
- `Preacher of the Schism`
- `Recurring Nightmare`
- `Sedgemoor Witch`
- `Sheoldred's Edict`
- `Sheoldred, the Apocalypse`
- `Snuff Out`
- `Tendrils of Agony`
- `Thoughtseize`
- `Toxic Deluge`
- `Troll of Khazad-dûm`
- `Unearth`
- `Vampire Hexmage`
- `Wishclaw Talisman`
- `Yawgmoth's Will`

### Red (51)

- `Abrade`
- `Amped Raptor`
- `Bonecrusher Giant`
- `Broadside Bombardiers`
- `Burst Lightning`
- `Chainsaw`
- `Chandra, Torch of Defiance`
- `Cori-Steel Cutter`
- `Death-Greeter's Champion`
- `Delayed Blast Fireball`
- `Detective's Phoenix`
- `Dragon's Rage Channeler`
- `Dreadhorde Arcanist`
- `Embereth Shieldbreaker`
- `Fable of the Mirror-Breaker`
- `Fear of Missing Out`
- `Fiery Confluence`
- `Flame Slash`
- `Fury`
- `Galvanic Blast`
- `Galvanic Discharge`
- `Gau, Feral Youth`
- `Generous Plunderer`
- `Glorybringer`
- `Goblin Rabblemaster`
- `Goldspan Dragon`
- `Gut, True Soul Zealot`
- `Headliner Scarlett`
- `Inti, Seneschal of the Sun`
- `Kari Zev, Skyship Raider`
- `Kellan, Planar Trailblazer`
- `Laelia, the Blade Reforged`
- `Legion Extruder`
- `Magda, Brazen Outlaw`
- `Mine Collapse`
- `Monstrous Rage`
- `Oliphaunt`
- `Orcish Lumberjack`
- `Ragavan, Nimble Pilferer`
- `Robber of the Rich`
- `Screaming Nemesis`
- `Seasoned Pyromancer`
- `Slickshot Show-Off`
- `Sneak Attack`
- `Suplex`
- `Tarfire`
- `Tersa Lightshatter`
- `Through the Breach`
- `Underworld Breach`
- `Unholy Heat`
- `Voldaren Epicure`

### Green (45)

- `Baloth Prime`
- `Bristly Bill, Spine Sower`
- `Cankerbloom`
- `Courser of Kruphix`
- `Elvish Reclaimer`
- `Endurance`
- `Esika's Chariot`
- `Eternal Witness`
- `Exploration`
- `Fanatic of Rhonas`
- `Fastbond`
- `Green Sun's Zenith`
- `Hexdrinker`
- `Icetill Explorer`
- `Ignoble Hierarch`
- `Invigorate`
- `Keen-Eyed Curator`
- `Legolas's Quick Reflexes`
- `Lotus Cobra`
- `Malevolent Rumble`
- `Mightform Harmonizer`
- `Mutagenic Growth`
- `Natural Order`
- `Nissa, Who Shakes the World`
- `Noble Hierarch`
- `Oath of Druids`
- `Once Upon a Time`
- `Ouroboroid`
- `Pest Infestation`
- `Questing Beast`
- `Scythecat Cub`
- `Sentinel of the Nameless City`
- `Six`
- `Springheart Nantuko`
- `Sylvan Caryatid`
- `Tear Asunder`
- `Tireless Tracker`
- `Titania, Protector of Argoth`
- `Traveling Chocobo`
- `Ulvenwald Oddity`
- `Ursine Monstrosity`
- `Vaultborn Tyrant`
- `Walk-In Closet // Forgotten Cellar`
- `Woodfall Primus`
- `Worldspine Wurm`

### Multicolor (53)

- `Ajani, Nacatl Pariah`
- `Arwen, Mortal Queen`
- `Atraxa, Grand Unifier`
- `Baleful Strix`
- `Bloodbraid Challenger`
- `Bloodtithe Harvester`
- `Brightglass Gearhulk`
- `Carnage Interpreter`
- `Dack Fayden`
- `Ertai Resurrected`
- `Etali, Primal Conqueror`
- `Expressive Iteration`
- `Fallen Shinobi`
- `Figure of Destiny`
- `Fire Covenant`
- `Flame of Anor`
- `Forth Eorlingas!`
- `Fractured Identity`
- `Grist, the Hunger Tide`
- `Kaito, Bane of Nightmares`
- `Knight of the Reliquary`
- `Kolaghan's Command`
- `Leovold, Emissary of Trest`
- `Loot, the Pathfinder`
- `Lurrus of the Dream-Den`
- `Lutri, the Spellchaser`
- `Manamorphose`
- `Minsc & Boo, Timeless Heroes`
- `Nadu, Winged Wisdom`
- `No More Lies`
- `Oko, Thief of Crowns`
- `Omnath, Locus of Creation`
- `Otharri, Suns' Glory`
- `Phlage, Titan of Fire's Fury`
- `Pillage the Bog`
- `Psychic Frog`
- `Saheeli, Sublime Artificer`
- `Shorikai, Genesis Engine`
- `Sorin of House Markov`
- `Tamiyo, Collector of Tales`
- `Tamiyo, Inquisitive Student`
- `Teferi, Hero of Dominaria`
- `Teferi, Time Raveler`
- `Territorial Kavu`
- `Third Path Iconoclast`
- `Thopter Foundry`
- `Torsten, Founder of Benalia`
- `Uro, Titan of Nature's Wrath`
- `Vivi Ornitier`
- `Wight of the Reliquary`
- `Witherbloom Apprentice`
- `Wrenn and Six`
- `Zirda, the Dawnwaker`

### Colorless (46)

- `Aether Spellbomb`
- `Agatha's Soul Cauldron`
- `Blightsteel Colossus`
- `Chromatic Star`
- `Chrome Mox`
- `Coalition Relic`
- `Coveted Jewel`
- `Crucible of Worlds`
- `Currency Converter`
- `Emrakul, the Aeons Torn`
- `Everflowing Chalice`
- `Expedition Map`
- `Ghost Vacuum`
- `Haywire Mite`
- `Kaldra Compleat`
- `Karn, Scion of Urza`
- `Lavaspur Boots`
- `Lightning Greaves`
- `Lion's Eye Diamond`
- `Manifold Key`
- `Memory Jar`
- `Mishra's Bauble`
- `Mox Opal`
- `Myr Battlesphere`
- `Pentad Prism`
- `Portal to Phyrexia`
- `Relic of Sauron`
- `Retrofitter Foundry`
- `Sensei's Divining Top`
- `Smuggler's Copter`
- `Soul-Guide Lantern`
- `Sowing Mycospawn`
- `Sword of the Meek`
- `Talisman of Conviction`
- `Talisman of Creativity`
- `Talisman of Curiosity`
- `Talisman of Dominance`
- `Talisman of Progress`
- `Tezzeret, Cruel Captain`
- `The Endstone`
- `The Mightstone and Weakstone`
- `The One Ring`
- `Ugin, Eye of the Storms`
- `Umezawa's Jitte`
- `Urza's Bauble`
- `Walking Ballista`

### Lands (37)

- `Arena of Glory`
- `Blazemire Verge`
- `Bleachbone Verge`
- `Boseiju, Who Endures`
- `Bountiful Landscape`
- `Celestial Colonnade`
- `City of Traitors`
- `Commercial District`
- `Creeping Tar Pit`
- `Dark Depths`
- `Fabled Passage`
- `Field of the Dead`
- `Hedge Maze`
- `Horizon Canopy`
- `Lush Portico`
- `Meticulous Archive`
- `Multiversal Passage`
- `Otawara, Soaring City`
- `Prismatic Vista`
- `Raucous Theater`
- `Riverpyre Verge`
- `Shadowy Backstreet`
- `Shelldock Isle`
- `Sheltering Landscape`
- `Shifting Woodland`
- `Starting Town`
- `Sunbaked Canyon`
- `Sunbillow Verge`
- `Talon Gates of Madara`
- `Thornspire Verge`
- `Thundering Falls`
- `Twisted Landscape`
- `Undercity Sewers`
- `Underground Mortuary`
- `Urza's Saga`
- `Wastewood Verge`
- `Waterlogged Grove`

## Already cataloged

These 151 pool cards are in the catalog because an earlier format needed them.
Being cataloged is not the same as being audited against the rest of the cube:
a card authored for Old School or Premodern may meet cards here it has never
been played beside.

- `Ancestral Recall`
- `Ancient Tomb`
- `Arid Mesa`
- `Avacyn's Pilgrim`
- `Badlands`
- `Balance`
- `Basalt Monolith`
- `Bayou`
- `Berserk`
- `Birds of Paradise`
- `Black Lotus`
- `Blackcleave Cliffs`
- `Blood Crypt`
- `Bloodstained Mire`
- `Blooming Marsh`
- `Botanical Sanctum`
- `Brain Freeze`
- `Breeding Pool`
- `Candelabra of Tawnos`
- `Cecil, Dark Knight`
- `Chain Lightning`
- `Channel`
- `Concealed Courtyard`
- `Copperline Gorge`
- `Counterspell`
- `Crop Rotation`
- `Dark Ritual`
- `Darkslick Shores`
- `Daze`
- `Deathrite Shaman`
- `Delighted Halfling`
- `Demonic Tutor`
- `Duress`
- `Elvish Mystic`
- `Entomb`
- `Faithless Looting`
- `Fireblast`
- `Firebolt`
- `Flooded Strand`
- `Force of Vigor`
- `Frantic Search`
- `Gaea's Cradle`
- `Generous Ent`
- `Goblin Bombardment`
- `Godless Shrine`
- `Grim Monolith`
- `Griselbrand`
- `Gush`
- `Hallowed Fountain`
- `Hymn to Tourach`
- `Indatha Triome`
- `Inspiring Vantage`
- `Ivora, Insatiable Heir`
- `Jetmir's Garden`
- `Karakas`
- `Ketria Triome`
- `Kitesail Freebooter`
- `Library of Alexandria`
- `Life // Death`
- `Lightning Bolt`
- `Liliana of the Veil`
- `Lingering Souls`
- `Llanowar Elves`
- `Lotus Petal`
- `Mana Confluence`
- `Mana Crypt`
- `Mana Drain`
- `Mana Leak`
- `Mana Tithe`
- `Mana Vault`
- `Marsh Flats`
- `Mind Stone`
- `Mind Twist`
- `Miscalculation`
- `Mishra's Workshop`
- `Misty Rainforest`
- `Mother of Runes`
- `Mox Diamond`
- `Mox Emerald`
- `Mox Jet`
- `Mox Pearl`
- `Mox Ruby`
- `Mox Sapphire`
- `Mystical Tutor`
- `Nettlecyst`
- `Overgrown Tomb`
- `Parallax Wave`
- `Plateau`
- `Polluted Delta`
- `Preordain`
- `Primeval Titan`
- `Pyrogoyf`
- `Pyrokinesis`
- `Raffine's Tower`
- `Rancor`
- `Raugrin Triome`
- `Razorverge Thicket`
- `Reanimate`
- `Sacred Foundry`
- `Savai Triome`
- `Savannah`
- `Scalding Tarn`
- `Scrubland`
- `Seachrome Coast`
- `Securitron Squadron`
- `Shallow Grave`
- `Skullclamp`
- `Snapcaster Mage`
- `Sol Ring`
- `Spara's Headquarters`
- `Spell Pierce`
- `Spellseeker`
- `Spirebluff Canal`
- `Steam Vents`
- `Stern Scolding`
- `Stomping Ground`
- `Strip Mine`
- `Swords to Plowshares`
- `Sylvan Safekeeper`
- `Taiga`
- `Temple Garden`
- `Thalia, Guardian of Thraben`
- `Thespian's Stage`
- `Thought Scour`
- `Tidehollow Sculler`
- `Tifa Lockhart`
- `Time Walk`
- `Time Warp`
- `Timetwister`
- `Tolarian Academy`
- `Tropical Island`
- `Tundra`
- `Underground Sea`
- `Unruly Krasis`
- `Upheaval`
- `Urborg, Tomb of Yawgmoth`
- `Vampiric Tutor`
- `Verdant Catacombs`
- `Vindicate`
- `Volcanic Island`
- `Wasteland`
- `Watery Grave`
- `Wheel of Fortune`
- `Windswept Heath`
- `Wooded Foothills`
- `Wrath of God`
- `Xander's Lounge`
- `Yavimaya, Cradle of Growth`
- `Zagoth Triome`
- `Ziatora's Proving Ground`
- `Zuran Orb`
