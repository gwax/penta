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

- 534 cards in the pool, of which 192 are cataloged and 342 are not
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

### White (36)

- `Adeline, Resplendent Cathar`
- `Cathar Commando`
- `Cosmogrand Zenith`
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
- `Phelia, Exuberant Shepherd`
- `Portable Hole`
- `Serra Paragon`
- `Skyclave Apparition`
- `Solitude`
- `Staff of the Storyteller`
- `Sunfall`
- `The Wandering Emperor`
- `Thraben Inspector`
- `Touch the Spirit Realm`
- `Virtue of Loyalty`
- `Voice of Victory`
- `Witch Enchanter`

### Blue (51)

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
- `Forensic Gadgeteer`
- `Gitaxian Probe`
- `Hullbreacher`
- `Jace, Vryn's Prodigy`
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

### Black (40)

- `Animate Dead`
- `Archon of Cruelty`
- `Baleful Mastery`
- `Barrowgoyf`
- `Bitter Triumph`
- `Bolas's Citadel`
- `Cabal Ritual`
- `Caustic Bronco`
- `Collective Brutality`
- `Concealing Curtains`
- `Crabomination`
- `Cut Down`
- `Dark Confidant`
- `Dauthi Voidwalker`
- `Dismember`
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
- `Tendrils of Agony`
- `Thoughtseize`
- `Troll of Khazad-dûm`
- `Unearth`
- `Vampire Hexmage`
- `Yawgmoth's Will`

### Red (46)

- `Abrade`
- `Amped Raptor`
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
- `Underworld Breach`
- `Voldaren Epicure`

### Green (42)

- `Baloth Prime`
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
- `Vaultborn Tyrant`
- `Walk-In Closet // Forgotten Cellar`
- `Woodfall Primus`
- `Worldspine Wurm`

### Multicolor (49)

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
- `Figure of Destiny`
- `Fire Covenant`
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
- `Tamiyo, Inquisitive Student`
- `Teferi, Hero of Dominaria`
- `Teferi, Time Raveler`
- `Territorial Kavu`
- `Third Path Iconoclast`
- `Thopter Foundry`
- `Torsten, Founder of Benalia`
- `Uro, Titan of Nature's Wrath`
- `Wight of the Reliquary`
- `Witherbloom Apprentice`
- `Wrenn and Six`
- `Zirda, the Dawnwaker`

### Colorless (42)

- `Aether Spellbomb`
- `Agatha's Soul Cauldron`
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
- `Urza's Bauble`
- `Walking Ballista`

### Lands (36)

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
- `Waterlogged Grove`

## Already cataloged

These 192 pool cards are in the catalog because an earlier format needed them.
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
- `Blightsteel Colossus`
- `Blood Crypt`
- `Bloodchief's Thirst`
- `Bloodstained Mire`
- `Blooming Marsh`
- `Bone Shards`
- `Bonecrusher Giant`
- `Botanical Sanctum`
- `Brain Freeze`
- `Breeding Pool`
- `Bristly Bill, Spine Sower`
- `Candelabra of Tawnos`
- `Cecil, Dark Knight`
- `Chain Lightning`
- `Chain of Smog`
- `Channel`
- `Concealed Courtyard`
- `Containment Priest`
- `Copperline Gorge`
- `Corpse Dance`
- `Council's Judgment`
- `Counterspell`
- `Crop Rotation`
- `Damn`
- `Dark Ritual`
- `Darkslick Shores`
- `Daze`
- `Deathrite Shaman`
- `Deep-Cavern Bat`
- `Delighted Halfling`
- `Demonic Tutor`
- `Doomsday`
- `Duress`
- `Elvish Mystic`
- `Entomb`
- `Faithless Looting`
- `Fallen Shinobi`
- `Fireblast`
- `Firebolt`
- `Flame Slash`
- `Flame of Anor`
- `Flooded Strand`
- `Force of Vigor`
- `Force of Will`
- `Frantic Search`
- `Fury`
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
- `Jace, Wielder of Mysteries`
- `Jetmir's Garden`
- `Karakas`
- `Ketria Triome`
- `Kitesail Freebooter`
- `Library of Alexandria`
- `Life // Death`
- `Lightning Bolt`
- `Lightning Greaves`
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
- `Nissa, Who Shakes the World`
- `Overgrown Tomb`
- `Palace Jailer`
- `Parallax Wave`
- `Path to Exile`
- `Plateau`
- `Polluted Delta`
- `Preordain`
- `Primeval Titan`
- `Prismatic Ending`
- `Pyrogoyf`
- `Pyrokinesis`
- `Raffine's Tower`
- `Rancor`
- `Raugrin Triome`
- `Razorverge Thicket`
- `Reanimate`
- `Reprieve`
- `Sacred Foundry`
- `Savai Triome`
- `Savannah`
- `Scalding Tarn`
- `Scrubland`
- `Seachrome Coast`
- `Securitron Squadron`
- `Shallow Grave`
- `Sheoldred, the Apocalypse`
- `Skullclamp`
- `Snapcaster Mage`
- `Snuff Out`
- `Sol Ring`
- `Sowing Mycospawn`
- `Spara's Headquarters`
- `Spell Pierce`
- `Spellseeker`
- `Spirebluff Canal`
- `Static Prison`
- `Steam Vents`
- `Stern Scolding`
- `Stock Up`
- `Stomping Ground`
- `Stoneforge Mystic`
- `Strip Mine`
- `Swords to Plowshares`
- `Sylvan Safekeeper`
- `Taiga`
- `Tamiyo, Collector of Tales`
- `Temple Garden`
- `Thalia, Guardian of Thraben`
- `Thespian's Stage`
- `Thought Scour`
- `Through the Breach`
- `Tidehollow Sculler`
- `Tifa Lockhart`
- `Time Walk`
- `Time Warp`
- `Timetwister`
- `Tolarian Academy`
- `Toxic Deluge`
- `Tropical Island`
- `Tundra`
- `Umezawa's Jitte`
- `Underground Sea`
- `Unexpectedly Absent`
- `Unholy Heat`
- `Unruly Krasis`
- `Upheaval`
- `Urborg, Tomb of Yawgmoth`
- `Ursine Monstrosity`
- `Vampiric Tutor`
- `Verdant Catacombs`
- `Vindicate`
- `Vivi Ornitier`
- `Volcanic Island`
- `Wasteland`
- `Wastewood Verge`
- `Watery Grave`
- `Wheel of Fortune`
- `Winds of Abandon`
- `Windswept Heath`
- `Wishclaw Talisman`
- `Wooded Foothills`
- `Wrath of God`
- `Xander's Lounge`
- `Yavimaya, Cradle of Growth`
- `Zagoth Triome`
- `Ziatora's Proving Ground`
- `Zuran Orb`
