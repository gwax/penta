import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import init, { WebGame } from "../app/wasm/osarena_wasm.js";

test("the packaged Rust engine plays through browser actions", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/osarena_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("Goblins", "Sligh", "Handcrafted", true, 9394);
  const opening = JSON.parse(game.state_json());
  assert.equal(opening.human.hand.length, 7);
  assert.equal(opening.opponent.handSize, 7);
  assert.ok(opening.actions.some((action) => action.label === "Keep this hand"));

  const keep = opening.actions.find((action) => action.label === "Keep this hand");
  game.act(keep.index);
  const afterKeep = JSON.parse(game.state_json());
  assert.equal(afterKeep.turn, 1);
  assert.ok(afterKeep.actions.length > 0);
  assert.ok(
    !afterKeep.actions.some((action) => action.label === "Keep this hand"),
  );

  game.free();
});
