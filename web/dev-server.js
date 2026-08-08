import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { devPortOverrideVariable, getWorktreeDevPort } from "./worktree-port.js";

const forwardedArguments = process.argv.slice(2);
const portFlag = forwardedArguments.find(
  (argument) =>
    argument === "--port" ||
    argument.startsWith("--port=") ||
    argument === "-p" ||
    /^-p(?:=)?\d+$/.test(argument),
);

// A flag would pin the port for this one invocation while `dev:url` kept
// reporting the assigned one, so the two would disagree exactly when a tool
// is relying on them to match. The environment variable is the supported
// way to pin a port, because every path reads it.
if (portFlag) {
  throw new Error(
    `Development ports are assigned per worktree; set ${devPortOverrideVariable} to pin one, ` +
      `or run \`pnpm run dev:url\` to read this worktree's port, instead of ${portFlag}`,
  );
}

const vinextEntry = fileURLToPath(new URL("cli.js", import.meta.resolve("vinext")));
const result = spawnSync(
  process.execPath,
  [vinextEntry, "dev", "--port", String(getWorktreeDevPort()), ...forwardedArguments],
  {
    env: {
      ...process.env,
      WRANGLER_LOG_PATH: process.env.WRANGLER_LOG_PATH ?? ".wrangler/wrangler.log",
    },
    stdio: "inherit",
  },
);

if (result.error) throw result.error;
if (result.signal) process.kill(process.pid, result.signal);
process.exitCode = result.status ?? 1;
