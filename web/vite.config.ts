import vinext from "vinext";
import { defineConfig } from "vite";
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
    bindings: [{ name: "GAME_ROOMS", class_name: "GameRoom" }],
  },
  migrations: [{ tag: "v1", new_sqlite_classes: ["GameRoom"] }],
};

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
    ],
  };
});
