export type Owner = "human" | "opponent";

export type Card = {
  id: number;
  name: string;
  kind: string;
  isLand?: boolean;
  rulesText: string;
  manaCost?: {
    generic: number;
    white: number;
    blue: number;
    black: number;
    red: number;
    green: number;
    x: boolean;
  } | null;
  owner?: Owner;
  tapped?: boolean;
  power?: number | null;
  toughness?: number | null;
  damage?: number;
  attacking?: boolean;
  blocking?: number | null;
  flying?: boolean;
  canAttack?: boolean;
  enteredThisTurn?: boolean;
  xValue?: number | null;
};

export type Action = {
  index: number;
  label: string;
  kind: "primary" | "combat" | "pass" | "danger";
  cardId?: number | null;
  targetCardId?: number | null;
  targetPlayer?: Owner | null;
  targetStackId?: number | null;
  targetCardIds?: number[];
  targetPlayers?: Owner[];
  targetStackIds?: number[];
  targetCount?: number;
  manaAbility?: boolean;
  spellAction?: boolean;
  sacrificeCardIds?: number[];
  combatDamageAttacker?: number | null;
  x?: number | null;
  paymentAction?: boolean;
  manaSourceIds?: number[];
  decisionId?: number | null;
  decisionOptionIds?: number[];
};

export type OpponentAction = {
  label: string;
  kind: "land" | "spell" | "ability" | "combat" | "choice";
  card?: string | null;
  cardId?: number | null;
  manaSources?: string[];
  state: GameState;
};

export type PlayerState = {
  life: number;
  library: number;
  mana: {
    white: number;
    blue: number;
    black: number;
    red: number;
    green: number;
    colorless: number;
  };
  hand?: Card[];
  handSize?: number;
  graveyard: string[];
};

export type DecisionOption = {
  id: number;
  label: string;
  cardId?: number | null;
  cardName?: string | null;
  zone: string;
};

export type DecisionState = {
  id: number;
  prompt: string;
  minimum: number;
  maximum: number;
  cancellable: boolean;
  visibility: string;
  options: DecisionOption[];
};

export type GameState = {
  turn: number;
  gameTurn: number;
  step: string;
  active: string;
  priority: string;
  human: PlayerState & { hand: Card[] };
  opponent: PlayerState & { handSize: number };
  battlefield: Card[];
  stack: Array<{
    id: number;
    cardId: number;
    name: string;
    owner: Owner;
    kind: string;
    cardKind: string;
    isLand?: boolean;
    manaCost?: Card["manaCost"];
    rulesText: string;
    power?: number | null;
    toughness?: number | null;
    x: number;
    targetCardIds: number[];
    targetPlayers: Owner[];
    targetStackIds: number[];
  }>;
  actions: Action[];
  passLabel: string | null;
  decision: DecisionState | null;
  opponentActions?: OpponentAction[];
  canUndoMana: boolean;
  canCancelAttackers: boolean;
  phaseStops: string[];
  autopassEnabled: boolean;
  result: null | { outcome: "win" | "loss" | "draw"; message: string };
  events: string[];
};
