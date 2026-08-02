# Spell web client

This directory contains the local browser client for the Rust game engine. It
uses React, vinext, and a generated WebAssembly bridge; no account or database
is required to play a local game.

## Prerequisites

- Node.js `>=22.13.0`
- Rust and the `wasm32-unknown-unknown` target
- `wasm-bindgen` on `PATH`

## Quick start

```bash
pnpm install
pnpm run wasm:build
pnpm run dev
```

Then open `http://localhost:3000`. The client defaults to The Deck versus
Goblins, and all game state stays in the browser.

## Checks

From the repository root, run:

```bash
./scripts/check-all.sh
```

That formats, lints, and tests both Rust crates, rebuilds the WASM artifact,
builds the client, and runs the browser-facing tests. The shorter commands are
available from this directory as `pnpm lint`, `pnpm build`, and `pnpm test`.

## Hosting

`vite.config.ts` and `worker/index.ts` retain the small Sites adapter used for
deployment. Local development does not require Cloudflare bindings; the
checked-in `.openai/hosting.json` is only consulted by the deployment plugin.
