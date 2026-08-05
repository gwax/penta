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

export const turnPhases = [
  { label: "Beginning", steps: ["Upkeep", "Draw"] },
  { label: "Main 1", steps: ["Precombat Main"] },
  {
    label: "Combat",
    steps: [
      "Beginning Of Combat",
      "Declare Attackers",
      "Declare Blockers",
      "Combat Damage",
      "End Of Combat",
    ],
  },
  { label: "Main 2", steps: ["Postcombat Main"] },
  { label: "Ending", steps: ["End", "Cleanup"] },
] as const;

export const opponentActionDurationMs = 3200;

export const turnBannerDurationMs = 1800;
