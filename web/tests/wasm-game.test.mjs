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
  assert.ok(
    opening.human.hand.every((card) => card.manaCost !== undefined),
    "cards expose their casting costs to the interface",
  );
  assert.ok(
    opening.human.hand.every((card) => typeof card.rulesText === "string" && card.rulesText.length > 0),
    "cards expose their rules text to the interface",
  );
  const openingCreature = opening.human.hand.find((card) =>
    card.kind.includes("creature"),
  );
  assert.ok(openingCreature, "the deterministic opening hand includes a creature");
  assert.equal(typeof openingCreature.power, "number");
  assert.equal(typeof openingCreature.toughness, "number");
  assert.ok(opening.actions.some((action) => action.label === "Keep this hand"));

  const keep = opening.actions.find((action) => action.label === "Keep this hand");
  game.act(keep.index);
  const afterKeep = JSON.parse(game.state_json());
  assert.equal(afterKeep.turn, 1);
  assert.ok(Array.isArray(afterKeep.opponentActions));
  assert.ok(
    afterKeep.opponentActions.every((action) => action.label !== "Pass priority"),
    "routine opponent priority passes stay out of the animation queue",
  );
  assert.ok(
    afterKeep.opponentActions.every((action) => action.kind !== "mana"),
    "mana taps stay out of the standalone animation queue",
  );
  assert.ok(
    afterKeep.actions.some(
      (action) => action.kind === "primary" || action.kind === "combat",
    ),
    "choice-free priority windows are passed automatically",
  );
  assert.ok(
    !afterKeep.actions.some((action) => action.label === "Keep this hand"),
  );

  game.free();
});

test("opponent mana taps are grouped with the spell they pay for", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/osarena_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("Goblins", "Sligh", "Handcrafted", false, 9394);
  const opening = JSON.parse(game.state_json());
  const keep = opening.actions.find((action) => action.label === "Keep this hand");
  game.act(keep.index);

  const afterKeep = JSON.parse(game.state_json());
  const paidAction = afterKeep.opponentActions.find(
    (action) => action.manaSources?.length > 0,
  );
  assert.ok(paidAction, "a paid spell or ability includes its tapped mana sources");
  assert.match(paidAction.label, /^(Cast|Activate) /);
  assert.ok(
    afterKeep.opponentActions.every((action) => action.kind !== "mana"),
    "there is no separate mana animation",
  );

  game.free();
});

test("casting a spell automatically taps available mana sources", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/osarena_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("Artifacts", "Sligh", "Handcrafted", true, 16);
  let state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Keep this hand").index);

  state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Cast Mox Ruby").index);

  state = JSON.parse(game.state_json());
  const castVise = state.actions.find((action) =>
    action.label.startsWith("Cast Black Vise"),
  );
  assert.ok(castVise, "Black Vise is castable before manually tapping Mox Ruby");
  game.act(castVise.index);

  state = JSON.parse(game.state_json());
  const mox = state.battlefield.find(
    (card) => card.owner === "human" && card.name === "Mox Ruby",
  );
  assert.equal(mox?.tapped, true);
  assert.equal(state.human.mana.red, 0);

  game.free();
});

test("targeted permanent actions identify their clickable battlefield target", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/osarena_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("Goblins", "Sligh", "Handcrafted", true, 1138831559);
  let state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Keep this hand").index);
  state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Play Mountain").index);

  for (let step = 0; step < 30; step += 1) {
    state = JSON.parse(game.state_json());
    if (state.actions.some((action) => action.label === "Play Strip Mine")) {
      break;
    }
    const pass =
      state.actions.find((action) => action.kind === "pass") ??
      state.actions.find((action) => action.label === "Decline");
    assert.ok(
      pass,
      `the human can yield each intervening priority window: ${JSON.stringify({
        turn: state.turn,
        step: state.step,
        actions: state.actions,
      })}`,
    );
    game.act(pass.index);
  }

  state = JSON.parse(game.state_json());
  const playStrip = state.actions.find((action) => action.label === "Play Strip Mine");
  assert.ok(playStrip, "the deterministic hand can play Strip Mine on turn two");
  game.act(playStrip.index);

  state = JSON.parse(game.state_json());
  const stripAction = state.actions.find((action) => {
    if (!action.label.startsWith("Activate Strip Mine →")) return false;
    return state.battlefield.some(
      (card) => card.id === action.targetCardId && card.owner === "opponent",
    );
  });
  assert.ok(stripAction, "Strip Mine exposes a targeted activation");
  const target = state.battlefield.find(
    (card) => card.id === stripAction.targetCardId,
  );
  assert.equal(target?.owner, "opponent");
  assert.equal(target?.kind, "land");

  game.free();
});
