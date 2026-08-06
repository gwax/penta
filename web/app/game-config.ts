export const deckNotes: Record<string, string> = {
  Goblins: "Tribal pressure · Grenade finish",
  Sligh: "Clean curve · Burn reach",
  Artifacts: "Fast mana · Atog engine",
  Robots: "Fast mana · Heavy artifact creatures",
  "The Deck": "Five-color control · Tome inevitability",
  "Mono Black": "Ritual starts · Discard and land destruction",
  "White Weenie": "Efficient threats · Crusade and Armageddon",
  Erhnamgeddon: "Green-white midrange · Armageddon lock",
  Counterburn: "Blue-red tempo · Counters and direct damage",
  "Lions DIB": "Blue-white tempo · Cheap threats and permission",
  "Lion Dib Bolt": "Blue-white tempo · Cheap threats and burn",
  "BWR Aggro": "Three-color pressure · Knights and burn",
  "GR Aggro": "Green-red pressure · Efficient creatures and tricks",
  "Troll Disk": "Black-red control · Trolls and sweepers",
  "Jeskai Aggro": "Blue-white-red tempo · Burn and permission",
};

/// Stands in for a deck in the setup dialog until the game is dealt, at which
/// point it resolves to one of the decks above. Not a deck name the engine
/// knows, so it must never reach `deck_by_name`.
export const randomDeck = "Random";

export const randomDeckNote = "Rolled fresh every time you deal";

export const deckNames = Object.keys(deckNotes);

export const deckChoices = [randomDeck, ...deckNames];

/// Rendered in the header until the first deal resolves the real deck. It has
/// to be a fixed name: picking one at random here would differ between the
/// server render and the client, and hydration would tear.
export const placeholderDeck = deckNames[0];

export const deckChoiceNote = (choice: string) =>
  choice === randomDeck ? randomDeckNote : (deckNotes[choice] ?? "");

export const defaultHumanDeck = randomDeck;
export const defaultBotDeck = randomDeck;

// `label` is the engine's identifier for a phase stop; `title` is what the
// player reads. They differ because the rules call it Main 2 and nobody else does.
export const turnPhases = [
  { label: "Beginning", title: "Beginning", steps: ["Upkeep", "Draw"] },
  { label: "Main 1", title: "First Main", steps: ["Precombat Main"] },
  {
    label: "Combat",
    title: "Combat",
    steps: [
      "Beginning Of Combat",
      "Declare Attackers",
      "Declare Blockers",
      "Combat Damage",
      "End Of Combat",
    ],
  },
  { label: "Main 2", title: "Second Main", steps: ["Postcombat Main"] },
  { label: "Ending", title: "Ending", steps: ["End", "Cleanup"] },
] as const;

// The board itself tells the story now — each beat only needs to be long
// enough to watch the cards move.
export const opponentActionDurationMs = 2000;

// A draw is one card moving and happens every single turn, so it gets just
// enough time to be seen leaving the library.
export const drawBeatDurationMs = 900;

export const turnBannerDurationMs = 1800;
