/**
 * One hosted game, owned by one Durable Object.
 *
 * The engine is authoritative here, and so is the presentation layer.
 * `WebGame` -- the same Rust the browser runs for a local game -- lives in
 * this object, and the browser receives the `state_json()` it already knows
 * how to render. Nothing is ported to TypeScript and there is no second copy
 * of the beats, the undo, or the pass label to keep in step.
 *
 * There is deliberately no client-side copy of the game. A `Game` holds the
 * opponent's hand and the library order, so a client holding one would be
 * holding the answers. `state_json()` is a seat's view and is the only thing
 * that leaves.
 *
 * Nothing stores engine state. A game at rest is its configuration plus the
 * commands issued against it, and it is rebuilt by replaying them, which the
 * engine's determinism makes exact. Phase stops and auto-pass are in that log
 * as well as the plays: they change where the engine stops, so a replay that
 * dropped them would land somewhere else.
 */

type EngineModule = typeof import("../app/wasm/penta_wasm.js");
type WebGame = InstanceType<EngineModule["WebGame"]>;

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

interface GameConfig {
  humanDeck: string;
  botDeck: string;
  botPolicy: string;
  humanFirst: boolean;
  seed: number;
  format?: string;
}

/** Everything a seat can do, in a form that can be written down and redone. */
type Command =
  | { t: "act"; index: number }
  | { t: "choose"; decision: number; options: number[] }
  | { t: "attackAll" }
  | { t: "cancelAttackers" }
  | { t: "blocks"; assignments: string }
  | { t: "undoMana" }
  | { t: "phaseStop"; phase: string; enabled: boolean }
  | { t: "autopass"; enabled: boolean };

interface StoredGame {
  config: GameConfig;
  commands: Command[];
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
  game?: WebGame;
  refused?: string;
}

const STORED = "hosted-game";

function apply(game: WebGame, command: Command): void {
  switch (command.t) {
    case "act":
      game.act(command.index);
      return;
    case "choose":
      game.choose_decision(command.decision, JSON.stringify(command.options));
      return;
    case "attackAll":
      game.attack_all();
      return;
    case "cancelAttackers":
      game.cancel_attackers();
      return;
    case "blocks":
      game.finalize_blocks(command.assignments);
      return;
    case "undoMana":
      game.undo_mana();
      return;
    case "phaseStop":
      game.set_phase_stop(command.phase, command.enabled);
      return;
    case "autopass":
      game.set_autopass(command.enabled);
  }
}

export class GameRoom {
  readonly #state: DurableState;
  #game: WebGame | null = null;
  #stored: StoredGame | null = null;

  constructor(state: DurableState) {
    this.#state = state;
  }

  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);
    const route = url.pathname.split("/").pop();
    try {
      if (route === "start") {
        return await this.#start((await request.json()) as GameConfig);
      }
      const game = await this.#load();
      if (!game) return Response.json({ error: "no game here yet" }, { status: 404 });
      if (route === "state") return this.#snapshot(game);
      if (route === "command") {
        return await this.#command(game, (await request.json()) as Command);
      }
      if (route === "record") return Response.json(this.#stored);
      return Response.json({ error: `unknown route ${route}` }, { status: 404 });
    } catch (cause) {
      return Response.json({ error: String(cause) }, { status: 400 });
    }
  }

  #snapshot(game: WebGame): Response {
    // The same JSON a local game hands the React app, so the browser cannot
    // tell which side of the wire the engine is on.
    return new Response(game.state_json(), {
      headers: { "content-type": "application/json" },
    });
  }

  async #start(config: GameConfig): Promise<Response> {
    const { WebGame, HostedGame } = await engine();
    // Built before it is stored, so a bad deck is rejected rather than
    // written down as a room that can never open.
    const game = new WebGame(
      config.humanDeck,
      config.botDeck,
      config.botPolicy,
      config.humanFirst,
      config.seed,
      config.format,
    );
    const stored: StoredGame = {
      config,
      commands: [],
      engineVersion: HostedGame.engineVersion(),
      protocolVersion: HostedGame.protocolVersion(),
    };
    await this.#state.storage.put(STORED, stored);
    this.#game = game;
    this.#stored = stored;
    return this.#snapshot(game);
  }

  async #load(): Promise<WebGame | null> {
    if (this.#game) return this.#game;
    // A throw inside `blockConcurrencyWhile` resets the object and the caller
    // only sees an opaque 500, so a refusal comes back as a value.
    const outcome: LoadOutcome = await this.#state.blockConcurrencyWhile(async () => {
      if (this.#game) return { game: this.#game };
      const stored = await this.#state.storage.get<StoredGame>(STORED);
      if (!stored) return {};
      const { WebGame, HostedGame } = await engine();
      const game = new WebGame(
        stored.config.humanDeck,
        stored.config.botDeck,
        stored.config.botPolicy,
        stored.config.humanFirst,
        stored.config.seed,
        stored.config.format,
      );
      const engineVersion = HostedGame.engineVersion();
      const protocolVersion = HostedGame.protocolVersion();
      if (
        stored.protocolVersion !== protocolVersion ||
        stored.engineVersion !== engineVersion
      ) {
        // Replaying across an engine change can land somewhere else, or on a
        // command that is no longer legal. Neither is worth guessing at.
        return {
          refused:
            `game was recorded on engine ${stored.engineVersion} protocol ` +
            `${stored.protocolVersion}, this is ${engineVersion} ` +
            `protocol ${protocolVersion}`,
        };
      }
      for (const [position, command] of stored.commands.entries()) {
        try {
          apply(game, command);
        } catch (cause) {
          return {
            refused:
              `command ${position} of ${stored.commands.length} ` +
              `(${command.t}) no longer applies: ${String(cause)}`,
          };
        }
      }
      this.#game = game;
      this.#stored = stored;
      return { game };
    });
    if (outcome.refused) throw new Error(outcome.refused);
    return outcome.game ?? null;
  }

  async #command(game: WebGame, command: Command): Promise<Response> {
    const stored = this.#stored;
    if (!stored) return Response.json({ error: "no game" }, { status: 404 });
    apply(game, command);
    stored.commands.push(command);
    await this.#state.storage.put(STORED, stored);
    return this.#snapshot(game);
  }
}
