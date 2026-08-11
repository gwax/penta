/** Cloudflare Worker entry point for the vinext-starter template. */
type EngineModule = typeof import("../app/wasm/penta_wasm.js");

// Loaded on first use, not at module scope: a request that never touches the
// engine should not pay for it, and the server-render test evaluates this
// module under Node, where a `.wasm` import is not a compiled module.
let engineReady: Promise<EngineModule> | null = null;
function engine(): Promise<EngineModule> {
  engineReady ??= (async () => {
    const [module, wasm] = await Promise.all([
      import("../app/wasm/penta_wasm.js"),
      // Workers have no `fetch` for local files, so the compiled module is
      // handed to wasm-bindgen rather than fetched by URL.
      import("../app/wasm/penta_wasm_bg.wasm"),
    ]);
    await module.default({ module_or_path: wasm.default });
    return module;
  })();
  return engineReady;
}

/** Plays a game to its end inside the Worker and reports what happened. */
async function selfCheck(): Promise<Response> {
  const { HostedGame } = await engine();
  const game = new HostedGame("old-school-93-94", "Sligh", "The Deck", "4242");
  for (let step = 0; step < 20000; step += 1) {
    const seat = game.decisionSeat();
    if (!seat) break;
    const observation = JSON.parse(game.observeJson(seat)) as {
      legalActions: { type: string }[];
    };
    if (observation.legalActions.length === 0) break;
    const index = observation.legalActions.findIndex(
      (action) => action.type !== "PassPriority" && action.type !== "Concede",
    );
    game.act(index === -1 ? 0 : index);
  }
  const history = JSON.parse(game.historyJson()) as number[];
  // The whole point of storing actions: rebuild and check it lands identically.
  const rebuilt = HostedGame.replay(
    "old-school-93-94",
    "Sligh",
    "The Deck",
    "4242",
    new Uint32Array(history),
  );
  return Response.json({
    engineVersion: HostedGame.engineVersion(),
    protocolVersion: HostedGame.protocolVersion(),
    actions: history.length,
    result: game.resultJson() ? JSON.parse(game.resultJson()!) : null,
    replayMatches: rebuilt.historyJson() === game.historyJson()
      && rebuilt.resultJson() === game.resultJson(),
  });
}
import { handleImageOptimization, DEFAULT_DEVICE_SIZES, DEFAULT_IMAGE_SIZES } from "vinext/server/image-optimization";
import handler from "vinext/server/app-router-entry";

interface AssetBinding {
  fetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response>;
}

interface DurableObjectNamespace {
  idFromName(name: string): unknown;
  get(id: unknown): { fetch(request: Request): Promise<Response> };
}

interface Env {
  ASSETS: AssetBinding;
  GAME_ROOMS: DurableObjectNamespace;
  BUGS: DurableObjectNamespace;
  /**
   * Set to `enabled` to serve the server-side game routes. They are off by
   * default and should stay off in anything public: there is no auth and no
   * rate limit, so a caller can open Durable Objects without bound and make
   * the Worker play whole games on demand.
   */
  HOSTED_GAMES?: string;
  // Reserved for a future Sites database binding; this project currently has none.
  DB: unknown;
  IMAGES: {
    input(stream: ReadableStream): {
      transform(options: Record<string, unknown>): {
        output(options: { format: string; quality: number }): Promise<{ response(): Response }>;
      };
    };
  };
}

interface ExecutionContext {
  waitUntil(promise: Promise<unknown>): void;
  passThroughOnException(): void;
}

// Image security config. SVG sources with .svg extension auto-skip the
// optimization endpoint on the client side (served directly, no proxy).
// To route SVGs through the optimizer (with security headers), set
// dangerouslyAllowSVG: true in next.config.js and uncomment below:
// const imageConfig: ImageConfig = { dangerouslyAllowSVG: true };

const worker = {
  async fetch(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(request.url);

    const hostedGames = env.HOSTED_GAMES === "enabled";

    // Proof that the engine runs server-side, not a product endpoint.
    if (url.pathname === "/_engine/self-check") {
      return hostedGames ? selfCheck() : new Response("not found", { status: 404 });
    }

    // /_game/<id>/<start|observe|act|record>. The id names the Durable
    // Object, so every request for one game reaches the same instance and the
    // engine has a single writer.
    const room = hostedGames ? url.pathname.match(/^\/_game\/([^/]+)\/[^/]+$/) : null;
    if (room) {
      const stub = env.GAME_ROOMS.get(env.GAME_ROOMS.idFromName(room[1]));
      return stub.fetch(request);
    }

    // The bug ledger: one object for the whole deployment.
    if (hostedGames && url.pathname.startsWith("/_bugs/")) {
      const stub = env.BUGS.get(env.BUGS.idFromName("bugs"));
      return stub.fetch(request);
    }

    if (url.pathname === "/_vinext/image") {
      const allowedWidths = [...DEFAULT_DEVICE_SIZES, ...DEFAULT_IMAGE_SIZES];
      return handleImageOptimization(request, {
        fetchAsset: (path) => env.ASSETS.fetch(new Request(new URL(path, request.url))),
        transformImage: async (body, { width, format, quality }) => {
          const result = await env.IMAGES.input(body).transform(width > 0 ? { width } : {}).output({ format, quality });
          return result.response();
        },
      }, allowedWidths);
    }

    return handler.fetch(request, env, ctx);
  },
};

export { GameRoom } from "./game-room";
export { BugTracker } from "./bug-tracker";

export default worker;
