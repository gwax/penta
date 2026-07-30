"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import initWasm, { WebGame as RustWebGame } from "./wasm/osarena_wasm.js";

type Owner = "human" | "opponent";
type Card = {
  id: number;
  name: string;
  kind: string;
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
  result: null | { outcome: "win" | "loss" | "draw"; message: string };
  events: string[];
};
const deckNotes: Record<string, string> = {
  Goblins: "Tribal pressure · Grenade finish",
  Sligh: "Clean curve · Burn reach",
  Artifacts: "Fast mana · Atog engine",
};

const cleanEvent = (event: string) =>
  event
    .replaceAll(/CardInstanceId\((\d+)\)/g, "card #$1")
    .replaceAll(/CardDefinitionId\((\d+)\)/g, "definition #$1")
    .replaceAll(/[{}]/g, "")
    .replaceAll(/([a-z])([A-Z])/g, "$1 $2")
    .replaceAll(",", " ·");

export function GameClient() {
  const game = useRef<RustWebGame | null>(null);
  const wasmReady = useRef(false);
  const [state, setState] = useState<GameState | null>(null);
  const [humanDeck, setHumanDeck] = useState("Goblins");
  const [botDeck, setBotDeck] = useState("Sligh");
  const [policy, setPolicy] = useState("Handcrafted");
  const [humanFirst, setHumanFirst] = useState(true);
  const [seed, setSeed] = useState(9394);
  const [selectedCard, setSelectedCard] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(() => {
    if (!game.current) return;
    setState(JSON.parse(game.current.state_json()) as GameState);
    setSelectedCard(null);
  }, []);

  const newGame = useCallback(
    (
      nextSeed = seed,
      nextHumanDeck = humanDeck,
      nextBotDeck = botDeck,
      nextPolicy = policy,
      nextHumanFirst = humanFirst,
    ) => {
      if (!wasmReady.current) return;
      try {
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
    [botDeck, humanDeck, humanFirst, policy, refresh, seed],
  );

  useEffect(() => {
    let alive = true;
    const load = async () => {
      try {
        await initWasm();
        if (!alive) return;
        wasmReady.current = true;
        game.current = new RustWebGame(
          "Goblins",
          "Sligh",
          "Handcrafted",
          true,
          9394,
        );
        setState(JSON.parse(game.current.state_json()) as GameState);
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
  }, []);

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
    const actions =
      selectedCard === null
        ? state.actions
        : state.actions.filter(
            (action) =>
              action.cardId === selectedCard ||
              action.kind === "pass" ||
              action.kind === "danger",
          );
    const order = { combat: 0, primary: 1, pass: 2, danger: 3 };
    return [...actions].sort((a, b) => order[a.kind] - order[b.kind]);
  }, [selectedCard, state]);

  const opponentPermanents =
    state?.battlefield.filter((card) => card.owner === "opponent") ?? [];
  const humanPermanents =
    state?.battlefield.filter((card) => card.owner === "human") ?? [];

  const cardActions = (id: number) =>
    state?.actions.filter((action) => action.cardId === id).length ?? 0;

  const selectCard = (id: number) => {
    const matching = state?.actions.filter((action) => action.cardId === id) ?? [];
    if (matching.length === 1) {
      act(matching[0]);
    } else if (matching.length > 1) {
      setSelectedCard((current) => (current === id ? null : id));
    }
  };

  const chooseRandomSeed = () => {
    const next = crypto.getRandomValues(new Uint32Array(1))[0];
    setSeed(next);
    newGame(next);
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

            <Zone
              cards={opponentPermanents}
              empty="Opponent battlefield"
              actionCount={cardActions}
              onSelect={selectCard}
              selectedCard={selectedCard}
            />

            <div className="center-line">
              <span>
                TURN {state.turn} <b>·</b> {state.active.toUpperCase()} ACTIVE
              </span>
              <strong>{state.step}</strong>
              <span>{state.priority.toUpperCase()} HAS PRIORITY</span>
            </div>

            {state.stack.length > 0 && (
              <div className="stack-zone">
                <span>STACK</span>
                {state.stack.map((item) => (
                  <div key={item.id} className="stack-card">
                    {item.name}
                    <small>{item.owner === "human" ? "YOU" : "OPPONENT"}</small>
                  </div>
                ))}
              </div>
            )}

            <Zone
              cards={humanPermanents}
              empty="Your battlefield"
              actionCount={cardActions}
              onSelect={selectCard}
              selectedCard={selectedCard}
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
                  selected={selectedCard === card.id}
                  onSelect={selectCard}
                />
              ))}
              {state.human.hand.length === 0 && (
                <span className="zone-empty">Your hand is empty</span>
              )}
            </div>
          </section>

          <aside className="decision-panel" aria-label="Legal actions">
            <div className="decision-heading">
              <div>
                <span>YOUR DECISION</span>
                <strong>{filteredActions.length} legal moves</strong>
              </div>
              {selectedCard !== null && (
                <button onClick={() => setSelectedCard(null)}>Clear filter</button>
              )}
            </div>
            <div className="action-list">
              {filteredActions.map((action) => (
                <button
                  className={`action action-${action.kind}`}
                  key={action.index}
                  onClick={() => act(action)}
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
              <button onClick={() => newGame()}>Replay seed</button>
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
  onSelect,
  selectedCard,
}: {
  cards: Card[];
  empty: string;
  actionCount(id: number): number;
  onSelect(id: number): void;
  selectedCard: number | null;
}) {
  return (
    <div className="battlefield-zone">
      {cards.map((card) => (
        <GameCard
          key={card.id}
          card={card}
          actionable={actionCount(card.id) > 0}
          selected={selectedCard === card.id}
          onSelect={onSelect}
          compact
        />
      ))}
      {cards.length === 0 && <span className="zone-empty">{empty}</span>}
    </div>
  );
}

function GameCard({
  card,
  actionable,
  selected,
  onSelect,
  compact = false,
}: {
  card: Card;
  actionable: boolean;
  selected: boolean;
  onSelect(id: number): void;
  compact?: boolean;
}) {
  const type = card.kind.replace("artifactcreature", "artifact creature");
  const isRed = !card.kind.includes("artifact") && !card.kind.includes("land");
  return (
    <button
      className={[
        "game-card",
        compact ? "game-card-compact" : "",
        `card-${card.kind}`,
        isRed ? "card-red" : "",
        card.tapped ? "is-tapped" : "",
        card.attacking ? "is-attacking" : "",
        actionable ? "is-actionable" : "",
        selected ? "is-selected" : "",
      ].join(" ")}
      disabled={!actionable}
      onClick={() => onSelect(card.id)}
      title={actionable ? `Choose an action for ${card.name}` : card.name}
    >
      <span className="card-title">{card.name}</span>
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
  );
}
