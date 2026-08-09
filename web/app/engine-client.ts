import initWasm, { WebGame as RustWebGame } from "./wasm/penta_wasm.js";
// wasm-bindgen's loader defaults to new URL(..., import.meta.url), which Vite 8
// resolves to a file: URL the browser refuses to load. Ask Vite for the asset
// URL instead.
import wasmUrl from "./wasm/penta_wasm_bg.wasm?url";
import type { GameState } from "./game-types";
import type { FormatId } from "./game-config";

export type EngineGame = RustWebGame;

export type EngineConfig = {
  format: FormatId;
  humanDeck: string;
  botDeck: string;
  policy: string;
  humanFirst: boolean;
  seed: number;
};

/** Loads the generated WASM module exactly once per browser session. */
export async function initializeEngine(): Promise<void> {
  await initWasm({ module_or_path: wasmUrl });
}

export function createEngineGame(config: EngineConfig): EngineGame {
  return new RustWebGame(
    config.humanDeck,
    config.botDeck,
    config.policy,
    config.humanFirst,
    config.seed,
    config.format,
  );
}

/**
 * Publishes a console handle for putting cards onto the battlefield, so a
 * board state can be reached directly instead of played toward.
 *
 * The underlying entry point exists only in a WASM build compiled with the
 * `dev-cheats` feature, which the production build never enables, so this is a
 * no-op in a deployed client rather than something to be trusted or guarded
 * against there.
 *
 *   penta.put("human", "Thragtusk")
 */
export function publishDevHandle(
  currentGame: () => EngineGame | null,
  refresh: () => void,
): void {
  if (typeof window === "undefined") {
    return;
  }
  const cheatOf = (game: EngineGame) =>
    (game as { dev_put_onto_battlefield?: (seat: string, card: string) => void })
      .dev_put_onto_battlefield;
  const game = currentGame();
  if (!game || typeof cheatOf(game) !== "function") {
    return;
  }
  (window as unknown as { penta: unknown }).penta = {
    put(seat: "human" | "bot", cardName: string) {
      // Resolve the game on each call: dealing a new one frees the old
      // WASM object, and a handle holding it would fault.
      const live = currentGame();
      const cheat = live && cheatOf(live);
      if (!live || typeof cheat !== "function") {
        return "no game in play";
      }
      cheat.call(live, seat, cardName);
      refresh();
      return `put ${cardName} onto ${seat}'s battlefield`;
    },
  };
}

/** Decodes the single JSON boundary exposed by the Rust facade. */
export function readEngineState(game: EngineGame): GameState {
  return JSON.parse(game.state_json()) as GameState;
}
