"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import initWasm, { WebGame as RustWebGame } from "./wasm/osarena_wasm.js";

type Owner = "human" | "opponent";
type Card = {
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
};
type Action = {
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
  x?: number | null;
  paymentAction?: boolean;
  manaSourceIds?: number[];
};
type OpponentAction = {
  label: string;
  kind: "land" | "spell" | "ability" | "combat" | "choice";
  card?: string | null;
  cardId?: number | null;
  manaSources?: string[];
  state: GameState;
};
type PlayerState = {
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
type GameState = {
  turn: number;
  step: string;
  active: string;
  priority: string;
  human: PlayerState & { hand: Card[] };
  opponent: PlayerState & { handSize: number };
  battlefield: Card[];
  stack: Array<{ id: number; name: string; owner: Owner; kind: string }>;
  actions: Action[];
  opponentActions?: OpponentAction[];
  canUndoMana: boolean;
  phaseStops: string[];
  autopassEnabled: boolean;
  result: null | { outcome: "win" | "loss" | "draw"; message: string };
  events: string[];
};
const deckNotes: Record<string, string> = {
  Goblins: "Tribal pressure · Grenade finish",
  Sligh: "Clean curve · Burn reach",
  Artifacts: "Fast mana · Atog engine",
  Robots: "Fast mana · Heavy artifact creatures",
  "The Deck": "Five-color control · Tome inevitability",
};
const defaultHumanDeck = "The Deck";
const defaultBotDeck = "Goblins";

const turnPhases = [
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
];
const opponentActionDurationMs = 3200;

const cleanEvent = (event: string) =>
  event
    .replaceAll(/CardInstanceId\((\d+)\)/g, "card #$1")
    .replaceAll(/CardDefinitionId\((\d+)\)/g, "definition #$1")
    .replaceAll(/[{}]/g, "")
    .replaceAll(/([a-z])([A-Z])/g, "$1 $2")
    .replaceAll(",", " ·");

const randomSeed = () => crypto.getRandomValues(new Uint32Array(1))[0];

const initialSeed = () => {
  const requested = new URLSearchParams(window.location.search).get("seed");
  if (requested !== null && /^\d+$/.test(requested)) {
    const parsed = Number(requested);
    if (Number.isSafeInteger(parsed) && parsed <= 0xffff_ffff) return parsed;
  }
  return randomSeed();
};

const initialDeck = () => {
  const requested = new URLSearchParams(window.location.search).get("deck");
  return requested && requested in deckNotes ? requested : defaultHumanDeck;
};

const initialHumanFirst = () =>
  new URLSearchParams(window.location.search).get("first") !== "false";

const cardTargetKey = (id: number) => `card:${id}`;
const playerTargetKey = (owner: Owner) => `player:${owner}`;
const actionTargetKeys = (action: Action) => [
  ...(action.targetCardIds ?? []).map(cardTargetKey),
  ...(action.targetPlayers ?? []).map(playerTargetKey),
];
const sameTargets = (left: string[], right: string[]) =>
  left.length === right.length && left.every((target) => right.includes(target));

export function GameClient() {
  const game = useRef<RustWebGame | null>(null);
  const tableRef = useRef<HTMLElement | null>(null);
  const wasmReady = useRef(false);
  const finalStateAfterOpponentActions = useRef<GameState | null>(null);
  const [state, setState] = useState<GameState | null>(null);
  const [humanDeck, setHumanDeck] = useState(defaultHumanDeck);
  const [botDeck, setBotDeck] = useState(defaultBotDeck);
  const [policy, setPolicy] = useState("Handcrafted");
  const [humanFirst, setHumanFirst] = useState(true);
  const [draftHumanDeck, setDraftHumanDeck] = useState(defaultHumanDeck);
  const [draftBotDeck, setDraftBotDeck] = useState(defaultBotDeck);
  const [draftPolicy, setDraftPolicy] = useState("Handcrafted");
  const [draftHumanFirst, setDraftHumanFirst] = useState(true);
  const [setupOpen, setSetupOpen] = useState(true);
  const [setupDismissible, setSetupDismissible] = useState(false);
  const [seed, setSeed] = useState(9394);
  const [selectedCard, setSelectedCard] = useState<number | null>(null);
  const [selectedTargetCard, setSelectedTargetCard] = useState<number | null>(null);
  const [selectedTargetPlayer, setSelectedTargetPlayer] = useState<Owner | null>(null);
  const [selectedTargetStackId, setSelectedTargetStackId] = useState<number | null>(null);
  const [selectedX, setSelectedX] = useState<number | null>(null);
  const [xPickerCard, setXPickerCard] = useState<number | null>(null);
  const [fireballTargets, setFireballTargets] = useState<string[]>([]);
  const [selectedBlocker, setSelectedBlocker] = useState<number | null>(null);
  const [blockAssignments, setBlockAssignments] = useState<Record<number, number>>({});
  const [cardActionMenu, setCardActionMenu] = useState<number | null>(null);
  const [pendingAction, setPendingAction] = useState<Action | null>(null);
  const [hoverPaymentAction, setHoverPaymentAction] = useState<Action | null>(null);
  const pendingActionRef = useRef<Action | null>(null);
  const dragDropped = useRef(false);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [engineReady, setEngineReady] = useState(false);
  const [opponentActionQueue, setOpponentActionQueue] = useState<OpponentAction[]>([]);
  const currentOpponentAction = opponentActionQueue[0] ?? null;
  const watchingOpponent = currentOpponentAction !== null;

  const presentSnapshot = useCallback((snapshot: GameState) => {
    const opponentActions = snapshot.opponentActions ?? [];
    if (opponentActions.length > 0) {
      finalStateAfterOpponentActions.current = snapshot;
      setOpponentActionQueue(opponentActions);
      setState(opponentActions[0].state);
    } else {
      finalStateAfterOpponentActions.current = null;
      setOpponentActionQueue([]);
      setState(snapshot);
    }
  }, []);

  const refresh = useCallback(() => {
    if (!game.current) return;
    const snapshot = JSON.parse(game.current.state_json()) as GameState;
    presentSnapshot(snapshot);
    setSelectedCard(null);
    setSelectedTargetCard(null);
    setSelectedTargetPlayer(null);
    setSelectedTargetStackId(null);
    setSelectedX(null);
    setXPickerCard(null);
    setFireballTargets([]);
    setSelectedBlocker(null);
    setBlockAssignments({});
    setCardActionMenu(null);
    setPendingAction(null);
    setHoverPaymentAction(null);
    pendingActionRef.current = null;
  }, [presentSnapshot]);

  const newGame = useCallback(
    (
      nextSeed = randomSeed(),
      nextHumanDeck = humanDeck,
      nextBotDeck = botDeck,
      nextPolicy = policy,
      nextHumanFirst = humanFirst,
    ) => {
      if (!wasmReady.current) return;
      try {
        setSeed(nextSeed);
        setOpponentActionQueue([]);
        finalStateAfterOpponentActions.current = null;
        game.current?.free();
        game.current = new RustWebGame(
          nextHumanDeck,
          nextBotDeck,
          nextPolicy,
          nextHumanFirst,
          nextSeed,
        );
        setError(null);
        refresh();
      } catch (cause) {
        setError(String(cause));
      }
    },
    [botDeck, humanDeck, humanFirst, policy, refresh],
  );

  useEffect(() => {
    let alive = true;
    const load = async () => {
      try {
        await initWasm();
        if (!alive) return;
        const startingSeed = initialSeed();
        const startingHumanDeck = initialDeck();
        const startingHumanFirst = initialHumanFirst();
        setSeed(startingSeed);
        setHumanDeck(startingHumanDeck);
        setDraftHumanDeck(startingHumanDeck);
        setHumanFirst(startingHumanFirst);
        setDraftHumanFirst(startingHumanFirst);
        wasmReady.current = true;
        setEngineReady(true);
        game.current = new RustWebGame(
          startingHumanDeck,
          defaultBotDeck,
          "Handcrafted",
          startingHumanFirst,
          startingSeed,
        );
        const snapshot = JSON.parse(game.current.state_json()) as GameState;
        presentSnapshot(snapshot);
      } catch (cause) {
        if (alive) setError(`Could not start the Rust engine: ${String(cause)}`);
      } finally {
        if (alive) setLoading(false);
      }
    };
    void load();
    return () => {
      alive = false;
      game.current?.free();
    };
  }, [presentSnapshot]);

  useEffect(() => {
    if (currentOpponentAction === null) return;
    const timer = window.setTimeout(() => {
      const remaining = opponentActionQueue.slice(1);
      if (remaining.length > 0) {
        setState(remaining[0].state);
      } else if (finalStateAfterOpponentActions.current) {
        setState(finalStateAfterOpponentActions.current);
        finalStateAfterOpponentActions.current = null;
      }
      setOpponentActionQueue(remaining);
    }, opponentActionDurationMs);
    return () => window.clearTimeout(timer);
  }, [currentOpponentAction, opponentActionQueue]);

  const skipOpponentActions = () => {
    if (finalStateAfterOpponentActions.current) {
      setState(finalStateAfterOpponentActions.current);
      finalStateAfterOpponentActions.current = null;
    }
    setOpponentActionQueue([]);
  };

  const act = (action: Action) => {
    try {
      game.current?.act(action.index);
      refresh();
    } catch (cause) {
      setError(String(cause));
    }
  };

  const prepareAction = (action: Action) => {
    act(action);
  };

  const cancelPendingAction = () => {
    setPendingAction(null);
    pendingActionRef.current = null;
  };

  const confirmPendingAction = () => {
    const action = pendingActionRef.current;
    if (action) act(action);
  };

  const togglePhaseStop = (phase: string, enabled: boolean) => {
    try {
      game.current?.set_phase_stop(phase, enabled);
      setState((current) => {
        if (!current) return current;
        const phaseStops = current.phaseStops.filter((candidate) => candidate !== phase);
        if (enabled) phaseStops.push(phase);
        return { ...current, phaseStops };
      });
    } catch (cause) {
      setError(String(cause));
    }
  };

  const toggleAutopass = (enabled: boolean) => {
    try {
      game.current?.set_autopass(enabled);
      refresh();
    } catch (cause) {
      setError(String(cause));
    }
  };

  const attackAll = () => {
    try {
      game.current?.attack_all();
      refresh();
    } catch (cause) {
      setError(String(cause));
    }
  };

  const finalizeBlocks = () => {
    try {
      const assignments = Object.entries(blockAssignments).map(([blocker, attacker]) => [
        Number(blocker),
        attacker,
      ]);
      game.current?.finalize_blocks(JSON.stringify(assignments));
      refresh();
    } catch (cause) {
      setError(String(cause));
    }
  };

  const beginCardDrag = (id: number) => {
    const matching = state?.actions.filter((action) => action.cardId === id) ?? [];
    const playable = matching.filter(
      (action) =>
        !action.manaAbility &&
        action.targetCardId == null &&
        action.targetPlayer == null &&
        action.targetStackId == null,
    );
    dragDropped.current = false;
    if (playable.length === 1) {
      setPendingAction(playable[0]);
      pendingActionRef.current = playable[0];
    } else if (playable.length > 1) {
      selectCard(id);
    }
  };

  const finishCardDrag = () => {
    if (!dragDropped.current) cancelPendingAction();
  };

  const previewPaymentForCard = (id: number) => {
    const payment = state?.actions.find(
      (action) => action.cardId === id && action.paymentAction,
    );
    setHoverPaymentAction(payment ?? null);
  };

  const clearPaymentPreview = () => setHoverPaymentAction(null);

  const declaringBlockers =
    state?.step === "Declare Blockers" &&
    state.actions.some((action) => action.label === "Finish blocking");
  const preparingBlockers =
    state?.step === "Declare Attackers" &&
    state.active === "Opponent" &&
    state.priority === "You" &&
    state.battlefield.some((card) => card.attacking);
  const actionMatchesSelectedX = useCallback(
    (action: Action) =>
      selectedX === null || action.cardId !== selectedCard || action.x === selectedX,
    [selectedCard, selectedX],
  );
  const selectedHandCard = state?.human.hand.find((card) => card.id === selectedCard);
  const choosingFireballTargets =
    selectedHandCard?.name === "Fireball" && selectedX !== null;
  const fireballActions =
    state?.actions.filter(
      (action) =>
        choosingFireballTargets &&
        action.cardId === selectedCard &&
        action.x === selectedX,
    ) ?? [];
  const fireballCastAction = fireballActions.find((action) =>
    sameTargets(actionTargetKeys(action), fireballTargets),
  );
  const canChooseFireballTarget = (target: string) => {
    if (!choosingFireballTargets) return false;
    if (fireballTargets.includes(target)) return true;
    const proposed = [...fireballTargets, target];
    return fireballActions.some((action) =>
      proposed.every((candidate) => actionTargetKeys(action).includes(candidate)),
    );
  };
  const toggleFireballTarget = (target: string) => {
    if (!canChooseFireballTarget(target)) return;
    setFireballTargets((current) =>
      current.includes(target)
        ? current.filter((candidate) => candidate !== target)
        : [...current, target],
    );
  };
  const panelActions = useMemo(() => {
    if (!state) return [];
    const sourceActions = state.actions.filter(
      (action) => action.cardId === selectedCard && actionMatchesSelectedX(action),
    );
    const sourceHasTargets = sourceActions.some(
      (action) =>
        action.targetCardId != null ||
        action.targetPlayer != null ||
        action.targetStackId != null,
    );
    return state.actions.filter((action) => {
      if (action.kind === "danger") return false;
      if (!actionMatchesSelectedX(action)) return false;
      if (declaringBlockers && action.kind === "combat") return false;
      if (declaringBlockers && action.label === "Finish blocking") return false;
      if (action.kind === "pass" || action.cardId == null) return true;
      if (action.cardId !== selectedCard) return false;
      if (selectedTargetCard !== null) {
        return action.targetCardId === selectedTargetCard;
      }
      if (selectedTargetPlayer !== null) {
        return action.targetPlayer === selectedTargetPlayer;
      }
      if (selectedTargetStackId !== null) {
        return action.targetStackId === selectedTargetStackId;
      }
      return !sourceHasTargets && sourceActions.length > 1;
    });
  }, [actionMatchesSelectedX, declaringBlockers, selectedCard, selectedTargetCard, selectedTargetPlayer, selectedTargetStackId, state]);
  const dangerActions = state?.actions.filter((action) => action.kind === "danger") ?? [];
  const attackAllCount =
    state?.step === "Declare Attackers"
      ? state.actions.filter((action) => action.label.startsWith("Attack with ")).length
      : 0;

  const opponentPermanents =
    state?.battlefield.filter((card) => card.owner === "opponent") ?? [];
  const humanPermanents =
    state?.battlefield.filter((card) => card.owner === "human") ?? [];

  const cardActions = (id: number) =>
    watchingOpponent
      ? 0
      : state?.actions.filter(
          (action) =>
            action.cardId === id ||
            (action.cardId === selectedCard &&
              ((action.targetCardIds ?? []).includes(id) || action.targetCardId === id) &&
              actionMatchesSelectedX(action)) ||
            (action.cardId === selectedBlocker && action.targetCardId === id),
        ).length ?? 0;
  const cardIsDraggable = (id: number) => {
    if (watchingOpponent || !state?.human.hand.some((card) => card.id === id)) return false;
    const directActions = state.actions.filter(
      (action) =>
        action.cardId === id &&
        !action.manaAbility &&
        action.targetCardId == null &&
        action.targetPlayer == null &&
        action.targetStackId == null,
    );
    return directActions.length === 1;
  };

  const isTargetable = (id: number) =>
    !watchingOpponent &&
    ((choosingFireballTargets && canChooseFireballTarget(cardTargetKey(id))) ||
      (declaringBlockers &&
      selectedBlocker !== null &&
      (state?.actions.some(
        (action) =>
          action.cardId === selectedBlocker && action.targetCardId === id,
      ) ?? false)) ||
      (selectedCard !== null &&
        (state?.actions.some(
          (action) =>
            action.cardId === selectedCard &&
            action.targetCardId === id &&
            actionMatchesSelectedX(action),
        ) ?? false)));

  const isPlayerTargetable = (owner: Owner) =>
    !watchingOpponent &&
    selectedCard !== null &&
    ((choosingFireballTargets && canChooseFireballTarget(playerTargetKey(owner))) ||
      (state?.actions.some(
        (action) =>
          action.cardId === selectedCard &&
          action.targetPlayer === owner &&
          actionMatchesSelectedX(action),
      ) ?? false));

  const isStackTargetable = (id: number) =>
    !watchingOpponent &&
    selectedCard !== null &&
    (state?.actions.some(
      (action) =>
        action.cardId === selectedCard &&
        action.targetStackId === id &&
        actionMatchesSelectedX(action),
    ) ?? false);

  const selectedSource = state?.battlefield
    .concat(state.human.hand)
    .find((card) => card.id === selectedCard);
  const actionMenuSource = state?.battlefield
    .concat(state.human.hand)
    .find((card) => card.id === cardActionMenu);
  const xPickerSource = state?.human.hand.find((card) => card.id === xPickerCard);
  const xPickerValues = Array.from(
    new Set(
      state?.actions
        .filter((action) => action.cardId === xPickerCard && action.x != null)
        .map((action) => action.x as number) ?? [],
    ),
  ).sort((left, right) => left - right);
  const previewedPayment = fireballCastAction ?? pendingAction ?? hoverPaymentAction;
  const actionMenuActions =
    state?.actions.filter(
      (action) =>
        action.cardId === cardActionMenu &&
        action.targetCardId == null &&
        action.targetPlayer == null &&
        action.targetStackId == null,
    ) ?? [];
  const actionMenuHasTargets =
    cardActionMenu !== null &&
    (state?.actions.some(
      (action) =>
        action.cardId === cardActionMenu &&
        (action.targetCardId != null ||
          action.targetPlayer != null ||
          action.targetStackId != null),
    ) ?? false);
  const choosingTarget =
    !choosingFireballTargets &&
    selectedCard !== null &&
    selectedTargetCard === null &&
    selectedTargetPlayer === null &&
    selectedTargetStackId === null &&
    (state?.actions.some(
      (action) =>
        action.cardId === selectedCard &&
        actionMatchesSelectedX(action) &&
        (action.targetCardId != null ||
          action.targetPlayer != null ||
          action.targetStackId != null),
    ) ?? false);
  const selectedTarget = state?.battlefield.find(
    (card) => card.id === selectedTargetCard,
  );
  const choosingSacrifice = selectedCard !== null && selectedTargetCard !== null;

  const clearCardSelection = () => {
    setSelectedCard(null);
    setSelectedTargetCard(null);
    setSelectedTargetPlayer(null);
    setSelectedTargetStackId(null);
    setCardActionMenu(null);
    setSelectedX(null);
    setXPickerCard(null);
    setFireballTargets([]);
  };

  const selectPlayer = (owner: Owner) => {
    if (selectedCard === null) return;
    if (choosingFireballTargets) {
      toggleFireballTarget(playerTargetKey(owner));
      return;
    }
    const matching =
      state?.actions.filter(
        (action) =>
          action.cardId === selectedCard &&
          action.targetPlayer === owner &&
          actionMatchesSelectedX(action),
      ) ?? [];
    if (matching.length === 1) {
      prepareAction(matching[0]);
    } else if (matching.length > 1) {
      setSelectedTargetCard(null);
      setSelectedTargetPlayer(owner);
      setSelectedTargetStackId(null);
    }
  };

  const selectStackTarget = (id: number) => {
    if (selectedCard === null) return;
    const matching =
      state?.actions.filter(
        (action) =>
          action.cardId === selectedCard &&
          action.targetStackId === id &&
          actionMatchesSelectedX(action),
      ) ?? [];
    if (matching.length === 1) {
      prepareAction(matching[0]);
    } else if (matching.length > 1) {
      setSelectedTargetCard(null);
      setSelectedTargetPlayer(null);
      setSelectedTargetStackId(id);
    }
  };

  const selectCard = (id: number) => {
    if (declaringBlockers) {
      const blockerOptions =
        state?.actions.filter(
          (action) => action.cardId === id && action.targetCardId != null,
        ) ?? [];
      if (blockerOptions.length > 0) {
        if (selectedBlocker === id) {
          setSelectedBlocker(null);
          setBlockAssignments((current) => {
            const next = { ...current };
            delete next[id];
            return next;
          });
        } else {
          setSelectedBlocker(id);
        }
        return;
      }
      if (selectedBlocker !== null) {
        const assignment = state?.actions.find(
          (action) =>
            action.cardId === selectedBlocker && action.targetCardId === id,
        );
        if (assignment) {
          setBlockAssignments((current) => ({ ...current, [selectedBlocker]: id }));
          setSelectedBlocker(null);
          return;
        }
      }
    }
    if (choosingFireballTargets && canChooseFireballTarget(cardTargetKey(id))) {
      toggleFireballTarget(cardTargetKey(id));
      return;
    }
    if (selectedCard !== null) {
      const targeted =
        state?.actions.filter(
          (action) =>
            action.cardId === selectedCard &&
            action.targetCardId === id &&
            actionMatchesSelectedX(action),
        ) ?? [];
      if (targeted.length === 1) {
        prepareAction(targeted[0]);
        return;
      }
      if (targeted.length > 1) {
        setSelectedTargetCard(id);
        setSelectedTargetPlayer(null);
        setSelectedTargetStackId(null);
        return;
      }
      if (id === selectedCard) {
        clearCardSelection();
        return;
      }
    }

    const matching =
      state?.actions.filter((action) => action.cardId === id) ?? [];
    if (selectedCard !== null && selectedCard !== id) {
      setSelectedX(null);
      setXPickerCard(null);
      setFireballTargets([]);
    }
    const source = state?.human.hand.find((card) => card.id === id);
    const xValues = Array.from(
      new Set(
        matching
          .filter((action) => action.x != null)
          .map((action) => action.x as number),
      ),
    );
    if (source?.manaCost?.x && selectedCard !== id && xValues.length > 1) {
      setSelectedCard(id);
      setSelectedTargetCard(null);
      setSelectedTargetPlayer(null);
      setSelectedTargetStackId(null);
      setSelectedX(null);
      setXPickerCard(id);
      setFireballTargets([]);
      return;
    }
    const untargeted = matching.filter(
      (action) =>
        action.targetCardId == null &&
        action.targetPlayer == null &&
        action.targetStackId == null,
    );
    const hasTargetedAction = matching.some(
      (action) =>
        action.targetCardId != null ||
        action.targetPlayer != null ||
        action.targetStackId != null,
    );
    if (untargeted.length > 1 || (untargeted.length > 0 && hasTargetedAction)) {
      setSelectedCard(null);
      setSelectedTargetCard(null);
      setSelectedTargetPlayer(null);
      setSelectedTargetStackId(null);
      setCardActionMenu(id);
      return;
    }
    if (matching.length === 1 && matching[0].spellAction) {
      prepareAction(matching[0]);
      return;
    }
    if (hasTargetedAction) {
      setSelectedCard(id);
      setSelectedTargetCard(null);
      setSelectedTargetPlayer(null);
      setSelectedTargetStackId(null);
      return;
    }
    if (matching.length === 1) {
      prepareAction(matching[0]);
    } else if (matching.length > 1) {
      if (selectedCard === id) {
        clearCardSelection();
      } else {
        setSelectedCard(id);
        setSelectedTargetCard(null);
        setSelectedTargetPlayer(null);
        setSelectedTargetStackId(null);
      }
    }
  };

  const chooseRandomSeed = () => {
    newGame(randomSeed());
  };

  const openSetup = () => {
    setDraftHumanDeck(humanDeck);
    setDraftBotDeck(botDeck);
    setDraftPolicy(policy);
    setDraftHumanFirst(humanFirst);
    setSetupOpen(true);
  };

  const startConfiguredGame = () => {
    if (!wasmReady.current) return;
    setHumanDeck(draftHumanDeck);
    setBotDeck(draftBotDeck);
    setPolicy(draftPolicy);
    setHumanFirst(draftHumanFirst);
    newGame(
      setupDismissible ? randomSeed() : seed,
      draftHumanDeck,
      draftBotDeck,
      draftPolicy,
      draftHumanFirst,
    );
    setSetupDismissible(true);
    setSetupOpen(false);
  };

  const undoMana = () => {
    try {
      game.current?.undo_mana();
      refresh();
    } catch (cause) {
      setError(String(cause));
    }
  };

  return (
    <main className="arena">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">OS</span>
          <div>
            <strong>ARENA</strong>
            <small>OLD SCHOOL · 93/94</small>
          </div>
        </div>
        <div className="match-summary" aria-label="Current match">
          <span>You · {humanDeck}</span>
          <i>versus</i>
          <span>{policy} · {botDeck}</span>
        </div>
        <div className="topbar-actions">
          <span className="current-seed">Seed {seed}</span>
          <button className="new-game-link" onClick={openSetup}>
            New game
          </button>
        </div>
      </header>

      {setupOpen && (
        <div className="setup-backdrop">
          <section
            className="setup-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="setup-title"
          >
            <span className="setup-kicker">NEW MATCH</span>
            <h1 id="setup-title">Choose your deck</h1>
            <p>Pick your deck first, then choose what you want to play against.</p>
            {error && <div className="setup-error" role="alert">{error}</div>}
            <div className="setup-fields">
              <div className="setup-primary-choice">
                <label>
                  <span>Your deck</span>
                  <select
                    value={draftHumanDeck}
                    onChange={(event) => setDraftHumanDeck(event.target.value)}
                  >
                    {Object.keys(deckNotes).map((deck) => (
                      <option key={deck}>{deck}</option>
                    ))}
                  </select>
                  <small>{deckNotes[draftHumanDeck]}</small>
                </label>
                <label className="setup-seat">
                  <input
                    type="checkbox"
                    checked={draftHumanFirst}
                    onChange={(event) => setDraftHumanFirst(event.target.checked)}
                  />
                  <span>You play first</span>
                </label>
              </div>
              <div className="setup-opponent-choices">
                <label>
                  <span>Opponent deck</span>
                  <select
                    value={draftBotDeck}
                    onChange={(event) => setDraftBotDeck(event.target.value)}
                  >
                    {Object.keys(deckNotes).map((deck) => (
                      <option key={deck}>{deck}</option>
                    ))}
                  </select>
                  <small>{deckNotes[draftBotDeck]}</small>
                </label>
                <label className="setup-policy">
                  <span>Opponent style</span>
                  <select
                    value={draftPolicy}
                    onChange={(event) => setDraftPolicy(event.target.value)}
                  >
                    <option>Handcrafted</option>
                    <option>Random</option>
                  </select>
                  <small>
                    {draftPolicy === "Handcrafted"
                      ? "Purposeful Old School play."
                      : "Chooses randomly from legal actions."}
                  </small>
                </label>
              </div>
            </div>
            <div className="setup-actions">
              {setupDismissible && (
                <button className="setup-cancel" onClick={() => setSetupOpen(false)}>
                  Back to game
                </button>
              )}
              <button
                className="setup-start"
                onClick={startConfiguredGame}
                disabled={!engineReady}
              >
                {loading ? "Loading engine…" : "Deal game"}
              </button>
            </div>
          </section>
        </div>
      )}

      {xPickerCard !== null && xPickerSource && (
        <div className="card-action-backdrop">
          <section
            className="card-action-dialog x-picker-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="x-picker-title"
          >
            <span>CHOOSE X</span>
            <h2 id="x-picker-title">Cast {xPickerSource.name}</h2>
            <p>How much mana should X be?</p>
            <div className="x-picker-options">
              {xPickerValues.map((value) => (
                <button
                  key={value}
                  onClick={() => {
                    setSelectedX(value);
                    setFireballTargets([]);
                    setXPickerCard(null);
                  }}
                >
                  <strong>X = {value}</strong>
                </button>
              ))}
            </div>
            <button className="card-action-cancel" onClick={clearCardSelection}>
              Cancel
            </button>
          </section>
        </div>
      )}

      {cardActionMenu !== null && actionMenuSource && (
        <div className="card-action-backdrop">
          <section
            className="card-action-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="card-action-title"
          >
            <span>CHOOSE AN ACTION</span>
            <h2 id="card-action-title">{actionMenuSource.name}</h2>
            <div>
              {actionMenuActions.map((action) => (
                <button key={action.index} onClick={() => prepareAction(action)}>
                  <strong>{action.label}</strong>
                  <i aria-hidden="true">→</i>
                </button>
              ))}
              {actionMenuHasTargets && (
                <button
                  onClick={() => {
                    setSelectedCard(cardActionMenu);
                    setCardActionMenu(null);
                  }}
                >
                  <strong>Choose a target on the battlefield</strong>
                  <i aria-hidden="true">→</i>
                </button>
              )}
            </div>
            <button className="card-action-cancel" onClick={clearCardSelection}>
              Cancel
            </button>
          </section>
        </div>
      )}

      {loading && (
        <section className="engine-loading" role="status">
          <span className="loader-rune">R</span>
          <p>Waking the Rust engine…</p>
        </section>
      )}

      {error && (
        <div className="error-banner" role="alert">
          {error}
        </div>
      )}

      {state && (
        <div className="game-layout">
          <section
            ref={tableRef}
            className={`table ${pendingAction ? "is-payment-drop-target" : ""}`}
            aria-label="Game table"
            onDragOver={(event) => {
              if (pendingActionRef.current) event.preventDefault();
            }}
            onDrop={(event) => {
              if (!pendingActionRef.current) return;
              event.preventDefault();
              dragDropped.current = true;
              confirmPendingAction();
            }}
          >
            {declaringBlockers && (
              <BlockArrows
                assignments={blockAssignments}
                tableRef={tableRef}
              />
            )}
            <PlayerBar
              name={`${policy} · ${botDeck}`}
              note={deckNotes[botDeck]}
              player={state.opponent}
              opponent
              targetable={isPlayerTargetable("opponent")}
              selected={fireballTargets.includes(playerTargetKey("opponent"))}
              onTarget={() => selectPlayer("opponent")}
            />
            <div className="opponent-hand" aria-label={`${state.opponent.handSize} hidden cards`}>
              {Array.from({ length: state.opponent.handSize }, (_, index) => (
                <span className="card-back" key={index}>
                  <i>93</i>
                </span>
              ))}
            </div>

            {currentOpponentAction && (
              <div
                className={`opponent-action opponent-action-${currentOpponentAction.kind}`}
                key={`${currentOpponentAction.label}-${opponentActionQueue.length}`}
                role="status"
                aria-live="polite"
              >
                <span className="opponent-action-icon" aria-hidden="true">
                  {opponentActionSymbol(currentOpponentAction.kind)}
                </span>
                <div>
                  <small>Opponent</small>
                  <strong>{currentOpponentAction.label}</strong>
                  {currentOpponentAction.manaSources &&
                    currentOpponentAction.manaSources.length > 0 && (
                      <span className="opponent-action-mana-used">
                        Tapped {currentOpponentAction.manaSources.join(", ")}
                      </span>
                    )}
                </div>
                {opponentActionQueue.length > 1 && (
                  <span className="opponent-action-count">
                    +{opponentActionQueue.length - 1}
                  </span>
                )}
                <button onClick={skipOpponentActions}>
                  Skip
                </button>
              </div>
            )}

            <Zone
              cards={opponentPermanents}
              empty="Opponent battlefield"
              actionCount={cardActions}
              isDraggable={cardIsDraggable}
              isTargetable={isTargetable}
              onSelect={selectCard}
              selectedCard={selectedTargetCard ?? selectedCard}
              selectedCardIds={fireballTargets
                .filter((target) => target.startsWith("card:"))
                .map((target) => Number(target.slice(5)))}
              animatedCardId={currentOpponentAction?.cardId ?? null}
              previewManaSourceIds={previewedPayment?.manaSourceIds ?? []}
              onDragStartCard={beginCardDrag}
              onDragEndCard={finishCardDrag}
              opponent
            />

            <div className="center-line">
              <div className="turn-status">
                <strong>{state.active === "You" ? "Your turn" : "Opponent’s turn"}</strong>
                <span>Turn {state.turn}</span>
              </div>
              <ol className="phase-track" aria-label={`Current step: ${state.step}`}>
                {turnPhases.map((phase) => {
                  const current = phase.steps.includes(state.step);
                  const stopped = state.phaseStops.includes(phase.label);
                  return (
                    <li
                      className={`${current ? "phase-current" : ""} ${stopped ? "phase-stopped" : ""}`}
                      key={phase.label}
                    >
                      <button
                        type="button"
                        aria-pressed={stopped}
                        title={`${stopped ? "Remove" : "Set"} stop on ${phase.label}`}
                        onClick={() => togglePhaseStop(phase.label, !stopped)}
                      >
                        <span>{phase.label}</span>
                        {current && <small>{state.step}</small>}
                        {stopped && <i aria-label="Stop set">●</i>}
                      </button>
                    </li>
                  );
                })}
              </ol>
              <button
                type="button"
                className={`autopass-toggle ${state.autopassEnabled ? "is-on" : ""}`}
                aria-pressed={state.autopassEnabled}
                title="Automatically yield routine priority windows"
                onClick={() => toggleAutopass(!state.autopassEnabled)}
              >
                <span>Auto-pass</span>
                <i>{state.autopassEnabled ? "On" : "Off"}</i>
              </button>
            </div>

            <div
              className={`stack-zone ${state.stack.length === 0 ? "stack-zone-empty" : ""}`}
              aria-label="Stack"
            >
              {state.stack.length > 0 && (
                <>
                  <span>STACK</span>
                  {state.stack.map((item) => (
                    <button
                      key={item.id}
                      className={`stack-card ${isStackTargetable(item.id) ? "is-targetable" : ""}`}
                      onClick={() => selectStackTarget(item.id)}
                      disabled={!isStackTargetable(item.id)}
                    >
                      {item.name}
                      <small>{item.owner === "human" ? "YOU" : "OPPONENT"}</small>
                    </button>
                  ))}
                </>
              )}
            </div>

            <Zone
              cards={humanPermanents}
              empty="Your battlefield"
              actionCount={cardActions}
              isDraggable={cardIsDraggable}
              isTargetable={isTargetable}
              onSelect={selectCard}
              selectedCard={selectedBlocker ?? selectedTargetCard ?? selectedCard}
              selectedCardIds={fireballTargets
                .filter((target) => target.startsWith("card:"))
                .map((target) => Number(target.slice(5)))}
              animatedCardId={currentOpponentAction?.cardId ?? null}
              previewManaSourceIds={previewedPayment?.manaSourceIds ?? []}
              onDragStartCard={beginCardDrag}
              onDragEndCard={finishCardDrag}
            />

            <PlayerBar
              name={`You · ${humanDeck}`}
              note={deckNotes[humanDeck]}
              player={state.human}
              targetable={isPlayerTargetable("human")}
              selected={fireballTargets.includes(playerTargetKey("human"))}
              onTarget={() => selectPlayer("human")}
            />

            <HandZone
              cards={state.human.hand}
              actionCount={cardActions}
              isDraggable={cardIsDraggable}
              isTargetable={isTargetable}
              selectedCard={selectedCard}
              previewManaSourceIds={previewedPayment?.manaSourceIds ?? []}
              onSelect={selectCard}
              onDragStartCard={beginCardDrag}
              onDragEndCard={finishCardDrag}
              onPaymentPreviewStart={previewPaymentForCard}
              onPaymentPreviewEnd={clearPaymentPreview}
            />
          </section>

          <aside
            className={`decision-panel ${watchingOpponent ? "is-watching-opponent" : ""}`}
            aria-label="Legal actions"
            aria-busy={watchingOpponent}
          >
            <div className="decision-heading">
              <div>
                <span>{watchingOpponent ? "OPPONENT ACTING" : "YOUR DECISION"}</span>
                <strong>
                  {watchingOpponent
                    ? `${opponentActionQueue.length} action${opponentActionQueue.length === 1 ? "" : "s"}`
                    : declaringBlockers
                      ? "Assign your blockers"
                    : preparingBlockers
                      ? "Prepare your blockers"
                    : choosingFireballTargets
                      ? "Select Fireball targets"
                    : choosingTarget
                      ? "Choose a highlighted target"
                      : panelActions.some((action) => action.kind === "primary")
                        ? "Choose an option"
                        : "Choose a card or pass"}
                </strong>
              </div>
              {selectedCard !== null && (
                <button onClick={clearCardSelection}>
                  {choosingTarget || choosingFireballTargets || choosingSacrifice
                    ? "Cancel"
                    : "Clear filter"}
                </button>
              )}
            </div>
            <div className="action-list">
              {attackAllCount > 0 && (
                <button className="attack-all" onClick={attackAll} disabled={watchingOpponent}>
                  <span aria-hidden="true">⚔</span>
                  <strong>Attack all</strong>
                  <small>{attackAllCount} creature{attackAllCount === 1 ? "" : "s"}</small>
                </button>
              )}
              {choosingFireballTargets && (
                <div className="fireball-target-controls">
                  <div className="target-prompt" role="status">
                    <strong>Fireball · X = {selectedX}</strong>
                    <span>
                      Choose one or more highlighted creatures or players. Each target
                      beyond the first costs 1 additional mana.
                    </span>
                  </div>
                  <div className="fireball-target-summary">
                    {fireballTargets.length === 0 ? (
                      <span>No targets selected</span>
                    ) : (
                      fireballTargets.map((target) => {
                        const [kind, value] = target.split(":");
                        const label =
                          kind === "player"
                            ? value === "human"
                              ? "You"
                              : "Opponent"
                            : state.battlefield.find((card) => card.id === Number(value))
                                ?.name ?? `Card ${value}`;
                        return <span key={target}>{label}</span>;
                      })
                    )}
                  </div>
                  <div className="fireball-cost-summary">
                    <span>X</span>
                    <strong>{selectedX}</strong>
                    <i>+</i>
                    <span>Extra targets</span>
                    <strong>{Math.max(0, fireballTargets.length - 1)}</strong>
                    <i>+</i>
                    <span>Red</span>
                    <strong>R</strong>
                  </div>
                  <button
                    className="cast-fireball"
                    disabled={!fireballCastAction}
                    onClick={() => fireballCastAction && prepareAction(fireballCastAction)}
                  >
                    <strong>Cast Fireball</strong>
                    <small>
                      {fireballTargets.length === 0
                        ? "Choose at least one target"
                        : `${fireballTargets.length} target${fireballTargets.length === 1 ? "" : "s"} · ${Math.max(0, fireballTargets.length - 1)} extra mana`}
                    </small>
                  </button>
                </div>
              )}
              {declaringBlockers && (
                <div className="block-controls">
                  <div className="target-prompt" role="status">
                    <strong>
                      {selectedBlocker === null
                        ? "Choose a blocker"
                        : "Choose an attacker"}
                    </strong>
                    <span>
                      Click one of your highlighted creatures, then the attacker it
                      should block. Click a selected blocker again to remove its block.
                    </span>
                  </div>
                  <button className="finalize-blocks" onClick={finalizeBlocks}>
                    <strong>Finalize blocks</strong>
                    <small>
                      {Object.keys(blockAssignments).length === 0
                        ? "No blocks"
                        : `${Object.keys(blockAssignments).length} assigned`}
                    </small>
                  </button>
                </div>
              )}
              {state.canUndoMana && (
                <button className="undo-mana" onClick={undoMana}>
                  <span>↶</span>
                  <strong>Undo last mana tap</strong>
                </button>
              )}
              {choosingTarget && (
                <div className="target-prompt" role="status">
                  <strong>Choose a highlighted target</strong>
                  <span>
                    Click a highlighted card, player, or spell for{" "}
                    {selectedSource?.name ?? "this action"}
                    {selectedX !== null ? ` with X = ${selectedX}` : ""}.
                  </span>
                </div>
              )}
              {choosingSacrifice && (
                <div className="target-prompt" role="status">
                  <strong>Choose an artifact to sacrifice</strong>
                  <span>
                    Select a cost below to deal 2 damage to{" "}
                    {selectedTarget?.name ?? "that creature"}.
                  </span>
                </div>
              )}
              {!choosingFireballTargets && panelActions.map((action) => (
                <button
                  className={`action action-${action.kind}`}
                  key={action.index}
                  onClick={() => prepareAction(action)}
                  disabled={watchingOpponent}
                >
                  <span>{displayActionLabel(action, state)}</span>
                  <i aria-hidden="true">→</i>
                </button>
              ))}
            </div>
            {dangerActions.length > 0 && (
              <details className="game-menu">
                <summary>Game menu</summary>
                {dangerActions.map((action) => (
                  <button key={action.index} onClick={() => act(action)} disabled={watchingOpponent}>
                    {action.label}
                  </button>
                ))}
              </details>
            )}
            <details className="game-log">
              <summary>Game log</summary>
              <ol>
                {state.events.map((event, index) => (
                  <li key={`${event}-${index}`}>{cleanEvent(event)}</li>
                ))}
              </ol>
            </details>
          </aside>
        </div>
      )}

      {state?.result && (
        <div className="result-backdrop">
          <section className={`result-card result-${state.result.outcome}`}>
            <span>GAME OVER</span>
            <h1>{state.result.message}</h1>
            <p>
              Turn {state.turn} · {humanDeck} vs {botDeck}
            </p>
            <div>
              <button onClick={() => newGame(seed)}>Replay seed</button>
              <button className="result-primary" onClick={chooseRandomSeed}>
                New shuffled game
              </button>
            </div>
          </section>
        </div>
      )}
    </main>
  );
}

function opponentActionSymbol(kind: OpponentAction["kind"]) {
  switch (kind) {
    case "land":
      return "▲";
    case "spell":
      return "✦";
    case "ability":
      return "◇";
    case "combat":
      return "⚔";
    case "choice":
      return "…";
  }
}

function displayActionLabel(action: Action, state: GameState) {
  if (action.label !== "Pass priority") return action.label;
  if (state.stack.length > 0) return `Pass to resolve ${state.stack[0].name}`;
  switch (state.step) {
    case "Upkeep":
    case "Draw":
      return "Pass to main";
    case "Precombat Main":
      return "Pass to combat";
    case "Beginning Of Combat":
      return "Pass to attackers";
    case "Combat Damage":
    case "End Of Combat":
      return "Pass to main 2";
    case "Postcombat Main":
      return "Pass to end step";
    case "End":
    case "Cleanup":
      return "Pass to next turn";
    default:
      return "Pass priority";
  }
}

function PlayerBar({
  name,
  note,
  player,
  opponent = false,
  targetable = false,
  selected = false,
  onTarget,
}: {
  name: string;
  note: string;
  player: PlayerState;
  opponent?: boolean;
  targetable?: boolean;
  selected?: boolean;
  onTarget?(): void;
}) {
  return (
    <div
      className={`player-bar ${opponent ? "player-opponent" : ""} ${targetable ? "is-targetable" : ""} ${selected ? "is-selected-target" : ""}`}
    >
      <div className="avatar">{opponent ? "B" : "Y"}</div>
      <div className="player-name">
        <strong>{name}</strong>
        <span>{note}</span>
      </div>
      <div className="zone-counts">
        <span title="Library">LIB {player.library}</span>
        <span title="Graveyard">GY {player.graveyard.length}</span>
      </div>
      <div className="mana-pool" title="Mana pool">
        {player.mana.white > 0 && <span className="mana-white">W{player.mana.white}</span>}
        {player.mana.blue > 0 && <span className="mana-blue">U{player.mana.blue}</span>}
        {player.mana.black > 0 && <span className="mana-black">B{player.mana.black}</span>}
        <span className="mana-red">{player.mana.red}</span>
        {player.mana.green > 0 && <span className="mana-green">G{player.mana.green}</span>}
        <span className="mana-colorless">{player.mana.colorless}</span>
      </div>
      <div className="life-total">
        <small>LIFE</small>
        <strong>{player.life}</strong>
      </div>
      {targetable && (
        <button
          type="button"
          className="player-target-hitbox"
          aria-label={`Target ${opponent ? "opponent" : "yourself"}`}
          onClick={onTarget}
        />
      )}
    </div>
  );
}

function BlockArrows({
  assignments,
  tableRef,
}: {
  assignments: Record<number, number>;
  tableRef: { current: HTMLElement | null };
}) {
  const [lines, setLines] = useState<
    Array<{ blocker: number; attacker: number; x1: number; y1: number; x2: number; y2: number }>
  >([]);

  useEffect(() => {
    const table = tableRef.current;
    if (!table) return;
    const update = () => {
      const tableBounds = table.getBoundingClientRect();
      const next = Object.entries(assignments).flatMap(([blockerText, attacker]) => {
        const blocker = Number(blockerText);
        const blockerCard = table.querySelector<HTMLElement>(`[data-card-id="${blocker}"]`);
        const attackerCard = table.querySelector<HTMLElement>(`[data-card-id="${attacker}"]`);
        if (!blockerCard || !attackerCard) return [];
        const blockerBounds = blockerCard.getBoundingClientRect();
        const attackerBounds = attackerCard.getBoundingClientRect();
        return [{
          blocker,
          attacker,
          x1: blockerBounds.left + blockerBounds.width / 2 - tableBounds.left,
          y1: blockerBounds.top - tableBounds.top,
          x2: attackerBounds.left + attackerBounds.width / 2 - tableBounds.left,
          y2: attackerBounds.bottom - tableBounds.top,
        }];
      });
      setLines(next);
    };
    const frame = window.requestAnimationFrame(update);
    window.addEventListener("resize", update);
    table.addEventListener("scroll", update, true);
    return () => {
      window.cancelAnimationFrame(frame);
      window.removeEventListener("resize", update);
      table.removeEventListener("scroll", update, true);
    };
  }, [assignments, tableRef]);

  return (
    <svg className="block-arrows" aria-label="Current block assignments">
      <defs>
        <marker
          id="block-arrow-head"
          markerWidth="8"
          markerHeight="8"
          refX="7"
          refY="4"
          orient="auto"
          markerUnits="strokeWidth"
        >
          <path d="M 0 0 L 8 4 L 0 8 z" />
        </marker>
      </defs>
      {lines.map((line) => (
        <line
          key={line.blocker}
          x1={line.x1}
          y1={line.y1}
          x2={line.x2}
          y2={line.y2}
          markerEnd="url(#block-arrow-head)"
        />
      ))}
    </svg>
  );
}

function Zone({
  cards,
  empty,
  actionCount,
  isDraggable,
  isTargetable,
  onSelect,
  selectedCard,
  selectedCardIds = [],
  animatedCardId,
  previewManaSourceIds,
  onDragStartCard,
  onDragEndCard,
  opponent = false,
}: {
  cards: Card[];
  empty: string;
  actionCount(id: number): number;
  isDraggable(id: number): boolean;
  isTargetable(id: number): boolean;
  onSelect(id: number): void;
  selectedCard: number | null;
  selectedCardIds?: number[];
  animatedCardId: number | null;
  previewManaSourceIds: number[];
  onDragStartCard(id: number): void;
  onDragEndCard(): void;
  opponent?: boolean;
}) {
  const lands = cards.filter((card) => card.kind === "land");
  const nonlands = cards.filter((card) => card.kind !== "land");
  const renderCards = (laneCards: Card[]) =>
    groupCardsIntoPiles(laneCards).map((pile) => (
      <CardPile
        key={pile.key}
        cards={pile.cards}
        actionCount={actionCount}
        isDraggable={isDraggable}
        isTargetable={isTargetable}
        onSelect={onSelect}
        selectedCard={selectedCard}
        selectedCardIds={selectedCardIds}
        animatedCardId={animatedCardId}
        previewManaSourceIds={previewManaSourceIds}
        onDragStartCard={onDragStartCard}
        onDragEndCard={onDragEndCard}
      />
    ));

  return (
    <div
      className={`battlefield-zone ${opponent ? "battlefield-opponent" : "battlefield-human"}`}
    >
      <div className="battlefield-row battlefield-nonlands" aria-label="Nonland permanents">
        {renderCards(nonlands)}
      </div>
      <div className="battlefield-row battlefield-lands" aria-label="Lands">
        {renderCards(lands)}
      </div>
      {cards.length === 0 && <span className="zone-empty">{empty}</span>}
    </div>
  );
}

function groupCardsIntoPiles(cards: Card[]) {
  const piles = new Map<string, Card[]>();
  for (const card of cards) {
    // Visually different game states must remain in separate piles. Identical
    // cards in the same state can safely collapse while retaining each card id.
    const key = [
      card.name,
      card.kind,
      card.tapped ? "tapped" : "ready",
      card.attacking ? "attacking" : "idle",
      card.blocking ?? "",
      card.damage ?? 0,
    ].join(":");
    const pile = piles.get(key) ?? [];
    pile.push(card);
    piles.set(key, pile);
  }
  return Array.from(piles, ([key, pileCards]) => ({ key, cards: pileCards }));
}

function CardPile({
  cards,
  actionCount,
  isDraggable,
  isTargetable,
  onSelect,
  selectedCard,
  selectedCardIds,
  animatedCardId,
  previewManaSourceIds,
  onDragStartCard,
  onDragEndCard,
}: {
  cards: Card[];
  actionCount(id: number): number;
  isDraggable(id: number): boolean;
  isTargetable(id: number): boolean;
  onSelect(id: number): void;
  selectedCard: number | null;
  selectedCardIds: number[];
  animatedCardId: number | null;
  previewManaSourceIds: number[];
  onDragStartCard(id: number): void;
  onDragEndCard(): void;
}) {
  const [expanded, setExpanded] = useState(false);
  const spacing = expanded ? 72 : 7;
  const visibleOffsetCount = expanded ? cards.length - 1 : Math.min(cards.length - 1, 3);
  const width = 96 + visibleOffsetCount * spacing;

  return (
    <div
      className={`card-pile ${expanded ? "is-expanded" : ""}`}
      style={{ width }}
      onMouseEnter={() => setExpanded(true)}
      onMouseLeave={() => setExpanded(false)}
      onFocusCapture={() => setExpanded(true)}
      onBlurCapture={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setExpanded(false);
      }}
      aria-label={cards.length > 1 ? `${cards.length} ${cards[0].name} cards` : undefined}
    >
      {cards.map((card, index) => {
        const offset = expanded ? index * spacing : Math.min(index, 3) * spacing;
        return (
          <div
            className="battlefield-card-slot"
            key={card.id}
            style={{ transform: `translateX(${offset}px)`, zIndex: index + 1 }}
          >
            <GameCard
              card={card}
              actionable={actionCount(card.id) > 0}
              draggableAction={isDraggable(card.id)}
              targetable={isTargetable(card.id)}
              selected={selectedCard === card.id || selectedCardIds.includes(card.id)}
              animating={animatedCardId === card.id}
              previewMana={previewManaSourceIds.includes(card.id)}
              onSelect={onSelect}
              onDragStartCard={onDragStartCard}
              onDragEndCard={onDragEndCard}
              compact
            />
          </div>
        );
      })}
      {cards.length > 1 && !expanded && (
        <span className="card-pile-count" aria-hidden="true">{cards.length}</span>
      )}
    </div>
  );
}

function HandZone({
  cards,
  actionCount,
  isDraggable,
  isTargetable,
  selectedCard,
  previewManaSourceIds,
  onSelect,
  onDragStartCard,
  onDragEndCard,
  onPaymentPreviewStart,
  onPaymentPreviewEnd,
}: {
  cards: Card[];
  actionCount(id: number): number;
  isDraggable(id: number): boolean;
  isTargetable(id: number): boolean;
  selectedCard: number | null;
  previewManaSourceIds: number[];
  onSelect(id: number): void;
  onDragStartCard(id: number): void;
  onDragEndCard(): void;
  onPaymentPreviewStart(id: number): void;
  onPaymentPreviewEnd(): void;
}) {
  const [hoveredCard, setHoveredCard] = useState<number | null>(null);
  const center = (cards.length - 1) / 2;
  const spacing = Math.min(78, Math.max(42, 700 / Math.max(cards.length - 1, 1)));

  return (
    <div className="hand-zone" aria-label="Your hand">
      {cards.map((card, index) => {
          const relative = index - center;
          const isHovered = hoveredCard === card.id;
          const hoveredIndex = cards.findIndex((candidate) => candidate.id === hoveredCard);
          const neighborSpread = hoveredIndex < 0 || isHovered
            ? 0
            : index < hoveredIndex ? -18 : 18;
          return (
            <div
              className="hand-card-slot"
              key={card.id}
              style={{
                transform: `translateX(${relative * spacing + neighborSpread}px) translateY(${isHovered ? -52 : Math.abs(relative) * 3}px) rotate(${isHovered ? 0 : relative * 1.8}deg) scale(${isHovered ? 1.16 : 1})`,
                zIndex: isHovered ? 100 : index + 1,
              }}
            >
              <GameCard
                card={card}
                actionable={actionCount(card.id) > 0}
                draggableAction={isDraggable(card.id)}
                targetable={isTargetable(card.id)}
                selected={selectedCard === card.id}
                onSelect={onSelect}
                previewMana={previewManaSourceIds.includes(card.id)}
                onDragStartCard={onDragStartCard}
                onDragEndCard={onDragEndCard}
                onPaymentPreviewStart={onPaymentPreviewStart}
                onPaymentPreviewEnd={onPaymentPreviewEnd}
                onHoverChange={(hovered) => setHoveredCard(hovered ? card.id : null)}
              />
            </div>
          );
        })}
      {cards.length === 0 && <span className="zone-empty">Your hand is empty</span>}
    </div>
  );
}

function GameCard({
  card,
  actionable,
  draggableAction = false,
  targetable = false,
  selected,
  animating = false,
  previewMana = false,
  onSelect,
  onDragStartCard,
  onDragEndCard,
  onPaymentPreviewStart,
  onPaymentPreviewEnd,
  onHoverChange,
  compact = false,
}: {
  card: Card;
  actionable: boolean;
  draggableAction?: boolean;
  targetable?: boolean;
  selected: boolean;
  animating?: boolean;
  previewMana?: boolean;
  onSelect(id: number): void;
  onDragStartCard?(id: number): void;
  onDragEndCard?(): void;
  onPaymentPreviewStart?(id: number): void;
  onPaymentPreviewEnd?(): void;
  onHoverChange?(hovered: boolean): void;
  compact?: boolean;
}) {
  const [previewPosition, setPreviewPosition] = useState<{
    left: number;
    top: number;
  } | null>(null);
  const currentType = card.kind.replace("artifactcreature", "artifact creature");
  const type = card.isLand && card.kind !== "land" ? `land · ${currentType}` : currentType;
  const isRed =
    !card.kind.includes("artifact") &&
    !card.isLand &&
    (card.manaCost?.red ?? 0) > 0;
  const showZeroCost =
    !card.isLand &&
    card.manaCost?.generic === 0 &&
    card.manaCost.white === 0 &&
    card.manaCost.blue === 0 &&
    card.manaCost.black === 0 &&
    card.manaCost.red === 0 &&
    card.manaCost.green === 0 &&
    !card.manaCost.x;
  const manaSymbolCount = card.manaCost
    ? (card.manaCost.x ? 1 : 0) +
      (card.manaCost.generic > 0 || showZeroCost ? 1 : 0) +
      card.manaCost.white +
      card.manaCost.blue +
      card.manaCost.black +
      card.manaCost.red +
      card.manaCost.green
    : 0;
  const manaCost = formatManaCost(card);
  const battlefieldState = [
    card.owner ? (card.tapped ? "Tapped" : "Untapped") : null,
    card.attacking ? "Attacking" : null,
    card.flying ? "Flying" : null,
    card.damage ? `${card.damage} damage marked` : null,
  ].filter(Boolean);

  const showPreview = (element: HTMLButtonElement) => {
    const bounds = element.getBoundingClientRect();
    const previewWidth = 260;
    const previewHeight = 220;
    const gutter = 10;
    let left = bounds.right + gutter;
    if (left + previewWidth > window.innerWidth - gutter) {
      left = bounds.left - previewWidth - gutter;
    }
    left = Math.max(gutter, Math.min(left, window.innerWidth - previewWidth - gutter));
    const centeredTop = bounds.top + (bounds.height - previewHeight) / 2;
    const battlefieldTop =
      bounds.top + bounds.height / 2 < window.innerHeight / 2
        ? bounds.top - previewHeight - gutter
        : bounds.bottom + gutter;
    const top = Math.max(
      gutter,
      Math.min(
        compact ? battlefieldTop : centeredTop,
        window.innerHeight - previewHeight - gutter,
      ),
    );
    setPreviewPosition({ left, top });
  };

  const previewId = `card-rules-${card.id}`;
  return (
    <>
      <button
        data-card-id={card.id}
        className={[
          "game-card",
          compact ? "game-card-compact" : "",
          `card-${card.kind}`,
          isRed ? "card-red" : "",
          card.tapped ? "is-tapped" : "",
          card.attacking ? "is-attacking" : "",
          actionable ? "is-actionable" : "",
          targetable ? "is-targetable" : "",
          selected ? "is-selected" : "",
          animating ? "is-opponent-action-card" : "",
          previewMana ? "is-autotap-preview" : "",
        ].join(" ")}
        aria-label={
          targetable
            ? `Target ${card.name}`
            : actionable
              ? `Choose an action for ${card.name}`
              : `Inspect ${card.name}`
        }
        aria-describedby={previewPosition ? previewId : undefined}
        draggable={draggableAction && !targetable}
        onDragStart={(event) => {
          event.dataTransfer.effectAllowed = "move";
          event.dataTransfer.setData("text/plain", String(card.id));
          onDragStartCard?.(card.id);
        }}
        onDragEnd={() => onDragEndCard?.()}
        onMouseEnter={(event) => {
          showPreview(event.currentTarget);
          onPaymentPreviewStart?.(card.id);
          onHoverChange?.(true);
        }}
        onMouseLeave={() => {
          setPreviewPosition(null);
          onPaymentPreviewEnd?.();
          onHoverChange?.(false);
        }}
        onFocus={(event) => {
          showPreview(event.currentTarget);
          onPaymentPreviewStart?.(card.id);
          onHoverChange?.(true);
        }}
        onBlur={() => {
          setPreviewPosition(null);
          onPaymentPreviewEnd?.();
          onHoverChange?.(false);
        }}
        onClick={(event) => {
          event.currentTarget.blur();
          if (actionable) onSelect(card.id);
        }}
      >
        <span className={`card-header ${manaSymbolCount >= 3 ? "card-header-dense" : ""}`}>
          <span className="card-title">{card.name}</span>
          {card.manaCost && !card.isLand && (
            <span className="card-cost" aria-label={`Casting cost for ${card.name}`}>
              {card.manaCost.x && <i className="mana-generic">X</i>}
              {card.manaCost.generic > 0 && (
                <i className="mana-generic">{card.manaCost.generic}</i>
              )}
              {showZeroCost && <i className="mana-generic">0</i>}
              {Array.from({ length: card.manaCost.white }, (_, index) => (
                <i className="mana-white-symbol" key={`w${index}`}>W</i>
              ))}
              {Array.from({ length: card.manaCost.blue }, (_, index) => (
                <i className="mana-blue-symbol" key={`u${index}`}>U</i>
              ))}
              {Array.from({ length: card.manaCost.black }, (_, index) => (
                <i className="mana-black-symbol" key={`b${index}`}>B</i>
              ))}
              {Array.from({ length: card.manaCost.red }, (_, index) => (
                <i className="mana-red-symbol" key={`r${index}`}>R</i>
              ))}
              {Array.from({ length: card.manaCost.green }, (_, index) => (
                <i className="mana-green-symbol" key={`g${index}`}>G</i>
              ))}
            </span>
          )}
        </span>
        <span className="card-art" aria-hidden="true">
          <i>{card.kind.includes("land") ? "▲" : card.kind.includes("artifact") ? "◇" : "●"}</i>
        </span>
        <span className="card-type">{type}</span>
        <span className="card-text">
          {card.attacking ? "Attacking" : card.flying ? "Flying" : "Old School 93/94"}
        </span>
        {card.power !== null && card.power !== undefined && (
          <strong className="card-stats">
            {card.power}/{card.toughness}
            {card.damage ? <small> · {card.damage} marked</small> : null}
          </strong>
        )}
      </button>
      {previewPosition &&
        createPortal(
          <span
            className="card-hover-stats"
            id={previewId}
            role="tooltip"
            style={previewPosition}
          >
            <strong>{card.name}</strong>
            <span className="card-hover-type">{type}</span>
            <span className="card-hover-rules">{card.rulesText}</span>
            <span className="card-hover-details">
              <span><b>Cost</b> {manaCost}</span>
              {card.power !== null && card.power !== undefined && (
                <span><b>Power / toughness</b> {card.power} / {card.toughness}</span>
              )}
            </span>
            {battlefieldState.length > 0 && (
              <span className="card-hover-state">{battlefieldState.join(" · ")}</span>
            )}
          </span>,
          document.body,
        )}
    </>
  );
}

function formatManaCost(card: Card) {
  if (card.isLand) return "—";
  if (!card.manaCost) return "Unknown";
  const parts = [
    card.manaCost.x ? "X" : "",
    card.manaCost.generic > 0 ? String(card.manaCost.generic) : "",
    "W".repeat(card.manaCost.white),
    "U".repeat(card.manaCost.blue),
    "B".repeat(card.manaCost.black),
    "R".repeat(card.manaCost.red),
    "G".repeat(card.manaCost.green),
  ].filter(Boolean);
  return parts.length > 0 ? parts.join(" ") : "0";
}
