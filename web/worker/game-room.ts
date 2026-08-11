/**
 * One game, owned by one Durable Object.
 *
 * The engine is authoritative here and nowhere else. A seat sends an action
 * index and gets back its own redacted view; it never sees the other hand,
 * the library order, or the seed, because it is never sent them.
 *
 * Nothing stores engine state. A game at rest is the format, the decks, the
 * seed and the action indices taken, which is a few hundred bytes, and the
 * engine is deterministic enough to rebuild from it -- see
 * `the_same_seed_produces_the_same_bytes` in the protocol tests. That buys
 * persistence, replays and spectating from one artifact, and it means no
 * stored state has to be migrated when the engine changes.
 *
 * It does mean a recorded action can stop being legal when the engine
 * changes, so the versions are stored alongside and checked before a rebuild.
 * A mismatch refuses loudly rather than silently producing a different game.
 */

type EngineModule = typeof import("../app/wasm/penta_wasm.js");
type HostedGame = InstanceType<EngineModule["HostedGame"]>;

let engineReady: Promise<EngineModule> | null = null;
function engine(): Promise<EngineModule> {
  engineReady ??= (async () => {
    const [module, wasm] = await Promise.all([
      import("../app/wasm/penta_wasm.js"),
      import("../app/wasm/penta_wasm_bg.wasm"),
    ]);
    await module.default({ module_or_path: wasm.default });
    return module;
  })();
  return engineReady;
}

/** What is written down. Everything else is rebuilt from it. */
interface StoredGame {
  format: string;
  p1Deck: string;
  p2Deck: string;
  /** Text, not a number: a JS number cannot hold the u64 range exactly. */
  seed: string;
  actions: number[];
  engineVersion: string;
  protocolVersion: number;
}

interface DurableStorage {
  get<T>(key: string): Promise<T | undefined>;
  put<T>(key: string, value: T): Promise<void>;
}
interface DurableState {
  storage: DurableStorage;
  blockConcurrencyWhile<T>(callback: () => Promise<T>): Promise<T>;
}

/** Either the live game, a reason it cannot be rebuilt, or neither. */
interface LoadOutcome {
  game?: HostedGame;
  refused?: string;
}

const STORED = "game";

export class GameRoom {
  readonly #state: DurableState;
  #game: HostedGame | null = null;
  #stored: StoredGame | null = null;

  constructor(state: DurableState) {
    this.#state = state;
  }

  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);
    const action = url.pathname.split("/").pop();
    try {
      if (action === "start") {
        return await this.#start(await request.json());
      }
      const game = await this.#load();
      if (!game) {
        return Response.json({ error: "no game here yet" }, { status: 404 });
      }
      if (action === "observe") {
        return this.#observe(url.searchParams.get("seat") ?? "");
      }
      if (action === "act") {
        return await this.#act(await request.json());
      }
      if (action === "record") {
        return Response.json(this.#stored);
      }
      return Response.json({ error: `unknown action ${action}` }, { status: 404 });
    } catch (cause) {
      return Response.json({ error: String(cause) }, { status: 400 });
    }
  }

  async #start(body: unknown): Promise<Response> {
    const { format, p1Deck, p2Deck, seed } = body as Omit<
      StoredGame,
      "actions" | "engineVersion" | "protocolVersion"
    >;
    const { HostedGame } = await engine();
    // Constructing before storing means an unknown deck or format is rejected
    // rather than written down as an unopenable room.
    const game = new HostedGame(format, p1Deck, p2Deck, seed);
    const stored: StoredGame = {
      format,
      p1Deck,
      p2Deck,
      seed,
      actions: [],
      engineVersion: HostedGame.engineVersion(),
      protocolVersion: HostedGame.protocolVersion(),
    };
    await this.#state.storage.put(STORED, stored);
    this.#game = game;
    this.#stored = stored;
    return Response.json({ started: true, seat: game.decisionSeat() });
  }

  /**
   * The live game, rebuilt from the record if this object was evicted since
   * the last request. `blockConcurrencyWhile` keeps two requests from both
   * deciding to rebuild.
   */
  async #load(): Promise<HostedGame | null> {
    if (this.#game) return this.#game;
    // Throwing inside `blockConcurrencyWhile` resets the Durable Object, so
    // the caller never sees the reason -- it arrives as an opaque 500. A
    // refusal is reported as a value instead and turned into a response
    // outside.
    const outcome: LoadOutcome = await this.#state.blockConcurrencyWhile(async () => {
      if (this.#game) return { game: this.#game };
      const stored = await this.#state.storage.get<StoredGame>(STORED);
      if (!stored) return {};
      const { HostedGame } = await engine();
      const engineVersion = HostedGame.engineVersion();
      const protocolVersion = HostedGame.protocolVersion();
      if (
        stored.engineVersion !== engineVersion ||
        stored.protocolVersion !== protocolVersion
      ) {
        // Replaying across an engine change can produce a different game, or
        // an action that is no longer legal. Neither is worth guessing at.
        return {
          refused:
            `game was recorded on engine ${stored.engineVersion} protocol ` +
            `${stored.protocolVersion}, this is ${engineVersion} ` +
            `protocol ${protocolVersion}`,
        };
      }
      this.#game = HostedGame.replay(
        stored.format,
        stored.p1Deck,
        stored.p2Deck,
        stored.seed,
        new Uint32Array(stored.actions),
      );
      this.#stored = stored;
      return { game: this.#game };
    });
    if (outcome.refused) throw new Error(outcome.refused);
    return outcome.game ?? null;
  }

  #observe(seat: string): Response {
    const game = this.#game;
    if (!game) return Response.json({ error: "no game" }, { status: 404 });
    // `observeJson` is the redaction boundary. Everything a seat learns comes
    // through it, so there is one thing to audit rather than a policy spread
    // across handlers.
    return new Response(game.observeJson(seat), {
      headers: { "content-type": "application/json" },
    });
  }

  async #act(body: unknown): Promise<Response> {
    const game = this.#game;
    const stored = this.#stored;
    if (!game || !stored) return Response.json({ error: "no game" }, { status: 404 });
    const { seat, index } = body as { seat: string; index: number };
    // A seat may only act when the engine is waiting on it. Without this a
    // client could play the other side's turn.
    const waiting = game.decisionSeat();
    if (waiting !== seat) {
      return Response.json(
        { error: `not ${seat}'s decision; waiting on ${waiting ?? "nobody"}` },
        { status: 409 },
      );
    }
    game.act(index);
    stored.actions.push(index);
    await this.#state.storage.put(STORED, stored);
    return Response.json({
      seat: game.decisionSeat(),
      result: game.resultJson() ? JSON.parse(game.resultJson()!) : null,
      actions: stored.actions.length,
    });
  }
}
