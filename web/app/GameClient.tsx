"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import initWasm, { WebGame as RustWebGame } from "./wasm/osarena_wasm.js";

type Owner = "human" | "opponent";
type Card = {
  id: number;
  name: string;
  kind: string;
  rulesText: string;
  manaCost?: {
    generic: number;
    red: number;
    x: boolean;
  } | null;
  owner?: Owner;
  tapped?: boolean;
  power?: number | null;
  toughness?: number | null;
  damage?: number;
  attacking?: boolean;
  flying?: boolean;
};
type Action = {
  index: number;
  label: string;
  kind: "primary" | "combat" | "pass" | "danger";
  cardId?: number | null;
  targetCardId?: number | null;
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
  mana: { red: number; colorless: number };
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
  result: null | { outcome: "win" | "loss" | "draw"; message: string };
  events: string[];
};
const deckNotes: Record<string, string> = {
  Goblins: "Tribal pressure · Grenade finish",
  Sligh: "Clean curve · Burn reach",
  Artifacts: "Fast mana · Atog engine",
};

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
  return requested && requested in deckNotes ? requested : "Goblins";
};

const initialHumanFirst = () =>
  new URLSearchParams(window.location.search).get("first") !== "false";

export function GameClient() {
  const game = useRef<RustWebGame | null>(null);
  const wasmReady = useRef(false);
  const finalStateAfterOpponentActions = useRef<GameState | null>(null);
  const [state, setState] = useState<GameState | null>(null);
  const [humanDeck, setHumanDeck] = useState("Goblins");
  const [botDeck, setBotDeck] = useState("Sligh");
  const [policy, setPolicy] = useState("Handcrafted");
  const [humanFirst, setHumanFirst] = useState(true);
  const [seed, setSeed] = useState(9394);
  const [selectedCard, setSelectedCard] = useState<number | null>(null);
  const [selectedTargetCard, setSelectedTargetCard] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
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
        setHumanFirst(startingHumanFirst);
        wasmReady.current = true;
        game.current = new RustWebGame(
          startingHumanDeck,
          "Sligh",
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
    }, 1200);
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

  const filteredActions = useMemo(() => {
    if (!state) return [];
    const actions = (
      selectedCard === null
        ? state.actions
        : state.actions.filter(
            (action) =>
              action.cardId === selectedCard ||
              action.kind === "pass" ||
              action.kind === "danger",
          )
    ).filter((action) => {
      if (action.kind === "pass" || action.kind === "danger") return true;
      return selectedTargetCard === null
        ? action.targetCardId == null
        : action.targetCardId === selectedTargetCard;
    });
    const order = { combat: 0, primary: 1, pass: 2, danger: 3 };
    return [...actions].sort((a, b) => order[a.kind] - order[b.kind]);
  }, [selectedCard, selectedTargetCard, state]);
  const moveCount = filteredActions.filter(
    (action) => action.kind !== "danger",
  ).length;

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
            (action.cardId === selectedCard && action.targetCardId === id),
        ).length ?? 0;

  const isTargetable = (id: number) =>
    !watchingOpponent &&
    selectedCard !== null &&
    (state?.actions.some(
      (action) =>
        action.cardId === selectedCard && action.targetCardId === id,
    ) ?? false);

  const selectedSource = state?.battlefield
    .concat(state.human.hand)
    .find((card) => card.id === selectedCard);
  const choosingTarget =
    selectedCard !== null &&
    selectedTargetCard === null &&
    (state?.actions.some(
      (action) =>
        action.cardId === selectedCard && action.targetCardId != null,
    ) ?? false);
  const selectedTarget = state?.battlefield.find(
    (card) => card.id === selectedTargetCard,
  );
  const choosingSacrifice = selectedCard !== null && selectedTargetCard !== null;

  const clearCardSelection = () => {
    setSelectedCard(null);
    setSelectedTargetCard(null);
  };

  const selectCard = (id: number) => {
    if (selectedCard !== null) {
      const targeted =
        state?.actions.filter(
          (action) =>
            action.cardId === selectedCard && action.targetCardId === id,
        ) ?? [];
      if (targeted.length === 1) {
        act(targeted[0]);
        return;
      }
      if (targeted.length > 1) {
        setSelectedTargetCard(id);
        return;
      }
      if (id === selectedCard) {
        clearCardSelection();
        return;
      }
    }

    const matching =
      state?.actions.filter((action) => action.cardId === id) ?? [];
    if (matching.some((action) => action.targetCardId != null)) {
      setSelectedCard(id);
      setSelectedTargetCard(null);
      return;
    }
    if (matching.length === 1) {
      act(matching[0]);
    } else if (matching.length > 1) {
      if (selectedCard === id) {
        clearCardSelection();
      } else {
        setSelectedCard(id);
        setSelectedTargetCard(null);
      }
    }
  };

  const chooseRandomSeed = () => {
    newGame(randomSeed());
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
        <div className="match-controls" aria-label="Match settings">
          <label>
            <span>Your deck</span>
            <select
              value={humanDeck}
              onChange={(event) => setHumanDeck(event.target.value)}
            >
              {Object.keys(deckNotes).map((deck) => (
                <option key={deck}>{deck}</option>
              ))}
            </select>
          </label>
          <span className="versus">VS</span>
          <label>
            <span>Opponent deck</span>
            <select
              value={botDeck}
              onChange={(event) => setBotDeck(event.target.value)}
            >
              {Object.keys(deckNotes).map((deck) => (
                <option key={deck}>{deck}</option>
              ))}
            </select>
          </label>
          <label>
            <span>Opponent</span>
            <select
              value={policy}
              onChange={(event) => setPolicy(event.target.value)}
            >
              <option>Handcrafted</option>
              <option>Random</option>
            </select>
          </label>
          <label className="seat-toggle">
            <input
              type="checkbox"
              checked={humanFirst}
              onChange={(event) => setHumanFirst(event.target.checked)}
            />
            <span>You play first</span>
          </label>
          <button className="new-game" onClick={() => newGame()}>
            Deal new game
          </button>
        </div>
        <div className="seed-control">
          <span>SEED</span>
          <button onClick={chooseRandomSeed} title="Randomize seed">
            {seed} ↻
          </button>
        </div>
      </header>

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
          <section className="table" aria-label="Game table">
            <PlayerBar
              name={`${policy} · ${botDeck}`}
              note={deckNotes[botDeck]}
              player={state.opponent}
              opponent
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
              isTargetable={isTargetable}
              onSelect={selectCard}
              selectedCard={selectedTargetCard ?? selectedCard}
              animatedCardId={currentOpponentAction?.cardId ?? null}
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
                  return (
                    <li className={current ? "phase-current" : ""} key={phase.label}>
                      <span>{phase.label}</span>
                      {current && <small>{state.step}</small>}
                    </li>
                  );
                })}
              </ol>
              <span className="priority-status">
                {state.priority === "You" ? "You have priority" : "Opponent has priority"}
              </span>
            </div>

            <div
              className={`stack-zone ${state.stack.length === 0 ? "stack-zone-empty" : ""}`}
              aria-label="Stack"
            >
              {state.stack.length > 0 && (
                <>
                  <span>STACK</span>
                  {state.stack.map((item) => (
                    <div key={item.id} className="stack-card">
                      {item.name}
                      <small>{item.owner === "human" ? "YOU" : "OPPONENT"}</small>
                    </div>
                  ))}
                </>
              )}
            </div>

            <Zone
              cards={humanPermanents}
              empty="Your battlefield"
              actionCount={cardActions}
              isTargetable={isTargetable}
              onSelect={selectCard}
              selectedCard={selectedTargetCard ?? selectedCard}
              animatedCardId={currentOpponentAction?.cardId ?? null}
            />

            <PlayerBar
              name={`You · ${humanDeck}`}
              note={deckNotes[humanDeck]}
              player={state.human}
            />

            <div className="hand-zone">
              {state.human.hand.map((card) => (
                <GameCard
                  key={card.id}
                  card={card}
                  actionable={cardActions(card.id) > 0}
                  targetable={isTargetable(card.id)}
                  selected={selectedCard === card.id}
                  onSelect={selectCard}
                />
              ))}
              {state.human.hand.length === 0 && (
                <span className="zone-empty">Your hand is empty</span>
              )}
            </div>
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
                    : `${moveCount} move${moveCount === 1 ? "" : "s"}`}
                </strong>
              </div>
              {selectedCard !== null && (
                <button onClick={clearCardSelection}>Clear filter</button>
              )}
            </div>
            <div className="action-list">
              {choosingTarget && (
                <div className="target-prompt" role="status">
                  <strong>Choose a battlefield target</strong>
                  <span>
                    Click a highlighted permanent for{" "}
                    {selectedSource?.name ?? "this action"}.
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
              {filteredActions.map((action) => (
                <button
                  className={`action action-${action.kind}`}
                  key={action.index}
                  onClick={() => act(action)}
                  disabled={watchingOpponent}
                >
                  <span>{action.label}</span>
                  <i aria-hidden="true">→</i>
                </button>
              ))}
            </div>
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

function PlayerBar({
  name,
  note,
  player,
  opponent = false,
}: {
  name: string;
  note: string;
  player: PlayerState;
  opponent?: boolean;
}) {
  return (
    <div className={`player-bar ${opponent ? "player-opponent" : ""}`}>
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
        <span className="mana-red">{player.mana.red}</span>
        <span className="mana-colorless">{player.mana.colorless}</span>
      </div>
      <div className="life-total">
        <small>LIFE</small>
        <strong>{player.life}</strong>
      </div>
    </div>
  );
}

function Zone({
  cards,
  empty,
  actionCount,
  isTargetable,
  onSelect,
  selectedCard,
  animatedCardId,
  opponent = false,
}: {
  cards: Card[];
  empty: string;
  actionCount(id: number): number;
  isTargetable(id: number): boolean;
  onSelect(id: number): void;
  selectedCard: number | null;
  animatedCardId: number | null;
  opponent?: boolean;
}) {
  const lands = cards.filter((card) => card.kind === "land");
  const nonlands = cards.filter((card) => card.kind !== "land");
  const renderCards = (laneCards: Card[]) =>
    laneCards.map((card) => (
      <GameCard
        key={card.id}
        card={card}
        actionable={actionCount(card.id) > 0}
        targetable={isTargetable(card.id)}
        selected={selectedCard === card.id}
        animating={animatedCardId === card.id}
        onSelect={onSelect}
        compact
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

function GameCard({
  card,
  actionable,
  targetable = false,
  selected,
  animating = false,
  onSelect,
  compact = false,
}: {
  card: Card;
  actionable: boolean;
  targetable?: boolean;
  selected: boolean;
  animating?: boolean;
  onSelect(id: number): void;
  compact?: boolean;
}) {
  const [previewPosition, setPreviewPosition] = useState<{
    left: number;
    top: number;
  } | null>(null);
  const type = card.kind.replace("artifactcreature", "artifact creature");
  const isRed = !card.kind.includes("artifact") && !card.kind.includes("land");
  const showZeroCost =
    !card.kind.includes("land") &&
    card.manaCost?.generic === 0 &&
    card.manaCost.red === 0 &&
    !card.manaCost.x;
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
    const top = Math.max(
      gutter,
      Math.min(
        bounds.top + (bounds.height - previewHeight) / 2,
        window.innerHeight - previewHeight - gutter,
      ),
    );
    setPreviewPosition({ left, top });
  };

  const previewId = `card-rules-${card.id}`;
  return (
    <>
      <button
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
        ].join(" ")}
        aria-label={
          targetable
            ? `Target ${card.name}`
            : actionable
              ? `Choose an action for ${card.name}`
              : `Inspect ${card.name}`
        }
        aria-describedby={previewPosition ? previewId : undefined}
        onMouseEnter={(event) => showPreview(event.currentTarget)}
        onMouseLeave={() => setPreviewPosition(null)}
        onFocus={(event) => showPreview(event.currentTarget)}
        onBlur={() => setPreviewPosition(null)}
        onClick={() => {
          if (actionable) onSelect(card.id);
        }}
      >
        <span className="card-header">
          <span className="card-title">{card.name}</span>
          {card.manaCost && !card.kind.includes("land") && (
            <span className="card-cost" aria-label={`Casting cost for ${card.name}`}>
              {card.manaCost.x && <i className="mana-generic">X</i>}
              {card.manaCost.generic > 0 && (
                <i className="mana-generic">{card.manaCost.generic}</i>
              )}
              {showZeroCost && <i className="mana-generic">0</i>}
              {Array.from({ length: card.manaCost.red }, (_, index) => (
                <i className="mana-red-symbol" key={index}>R</i>
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
  if (card.kind.includes("land")) return "—";
  if (!card.manaCost) return "Unknown";
  const parts = [
    card.manaCost.x ? "X" : "",
    card.manaCost.generic > 0 ? String(card.manaCost.generic) : "",
    "R".repeat(card.manaCost.red),
  ].filter(Boolean);
  return parts.length > 0 ? parts.join(" ") : "0";
}
