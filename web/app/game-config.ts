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
  "Lion Dib Bolt": "Blue-white tempo · Cheap threats and burn",
  "BWR Aggro": "Three-color pressure · Knights and burn",
  "GR Aggro": "Green-red pressure · Efficient creatures and tricks",
  "Troll Disk": "Black-red control · Trolls and sweepers",
  "Jeskai Aggro": "Blue-white-red tempo · Burn and permission",
};

export const defaultHumanDeck = "The Deck";
export const defaultBotDeck = "Goblins";

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
