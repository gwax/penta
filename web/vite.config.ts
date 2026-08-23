import vinext from "vinext";
import { type Plugin, defineConfig } from "vite";
import { getWorktreeDevPort } from "./worktree-port.js";

// macOS Seatbelt blocks FSEvents, so sandboxed previews need polling for HMR.
const isSeatbeltSandbox = process.env.CODEX_SANDBOX === "seatbelt";

// The Worker serves the app, and hosts games the engine runs server-side.
// One Durable Object per game keeps the engine authoritative and gives each
// game a single writer, so two actions cannot race.
const workerConfig = {
  main: "./worker/index.ts",
  compatibility_flags: ["nodejs_compat"],
  durable_objects: {
    bindings: [
      { name: "GAME_ROOMS", class_name: "GameRoom" },
      { name: "BUGS", class_name: "BugTracker" },
      { name: "BOTS", class_name: "BotRegistry" },
    ],
  },
  // Hosted games and the bot registry are served everywhere, including the
  // public deployment: seats are held by tokens and the creating routes are
  // rate limited. The engine self-check is not -- it plays a whole game per
  // request -- so it stays off unless a deployment sets it.
  vars: { HOSTED_GAMES: "enabled" },
  // Ten creations a minute per address: a person deals a few games an hour,
  // and a bot registers once. Reads and moves are not counted here.
  ratelimits: [
    {
      name: "CREATE_LIMIT",
      namespace_id: "1001",
      simple: { limit: 10, period: 60 as const },
    },
  ],
  migrations: [
    { tag: "v1", new_sqlite_classes: ["GameRoom"] },
    { tag: "v2", new_sqlite_classes: ["BugTracker"] },
    { tag: "v3", new_sqlite_classes: ["BotRegistry"] },
  ],
};

/**
 * Drops asset files the ssr environment emits but never loads.
 *
 * The rsc and ssr environments both build into the single uploaded Worker
 * script, and each emits its own copy of every asset it imports. The engine
 * WASM is a genuine module import only in rsc, where the Worker instantiates
 * it. In ssr it arrives through the client's `?url` import, which needs the
 * hashed path but never the bytes: the browser fetches those from the client
 * assets directory, which is served separately and does not count against the
 * script size limit. Shipping them twice put a second full copy of the engine
 * into the script for nothing.
 *
 * Nothing serves the ssr output directory, so an emitted asset no chunk
 * imports is unreachable there by construction; only its URL matters, and that
 * URL resolves against the client assets.
 */
function dropUnimportedSsrAssets(): Plugin {
  return {
    name: "penta:drop-unimported-ssr-assets",
    generateBundle(_options, bundle) {
      if (this.environment.name !== "ssr") return;

      const imported = new Set<string>();
      for (const output of Object.values(bundle)) {
        if (output.type !== "chunk") continue;
        for (const file of output.imports) imported.add(file);
        for (const file of output.dynamicImports) imported.add(file);
      }

      for (const [fileName, output] of Object.entries(bundle)) {
        if (output.type === "asset" && !imported.has(fileName)) {
          delete bundle[fileName];
        }
      }
    },
  };
}

export default defineConfig(async () => {
  const devPort = getWorktreeDevPort();

  // Keep Wrangler and Miniflare state project-local. These are non-secret tool
  // settings; application environment belongs in ignored `.env*` files.
  process.env.WRANGLER_WRITE_LOGS ??= "false";
  process.env.WRANGLER_LOG_PATH ??= ".wrangler/logs";
  process.env.MINIFLARE_REGISTRY_PATH ??= ".wrangler/registry";

  // Wrangler snapshots its log path while the Cloudflare plugin is imported.
  const { cloudflare } = await import("@cloudflare/vite-plugin");

  return {
    server: {
      port: devPort,
      strictPort: true,
      ...(isSeatbeltSandbox
        ? { watch: { useFsEvents: false, usePolling: true } }
        : {}),
    },
    // The generated WASM declarations are TypeScript-only and are not valid
    // dependency-scan entry points. This app has a small, fixed dependency
    // graph, so discovery adds noise without improving startup.
    optimizeDeps: { noDiscovery: true },
    plugins: [
      vinext(),
      cloudflare({
        // The local client does not need the Miniflare inspector. Disabling
        // its extra listener keeps `pnpm dev` usable in locked-down sandboxes
        // and leaves the worktree's app port as the only server endpoint.
        inspectorPort: false,
        viteEnvironment: { name: "rsc", childEnvironments: ["ssr"] },
        config: workerConfig,
      }),
      dropUnimportedSsrAssets(),
    ],
  };
});
