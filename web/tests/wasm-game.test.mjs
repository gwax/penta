import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import init, { WebGame } from "../app/wasm/penta_wasm.js";

test("The Deck exposes colored costs and control rules to the browser", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("The Deck", "The Deck", "Handcrafted", true, 3);
  const opening = JSON.parse(game.state_json());
  const swords = opening.human.hand.find(
    (card) => card.name === "Swords to Plowshares",
  );
  const serra = opening.human.hand.find((card) => card.name === "Serra Angel");
  assert.ok(swords);
  assert.equal(swords.manaCost.white, 1);
  assert.match(swords.rulesText, /exile target creature/i);
  assert.ok(serra);
  assert.equal(serra.manaCost.white, 2);
  assert.equal(serra.power, 4);
  assert.equal(serra.toughness, 4);

  game.free();
});

test("staged engine decisions are serialized as generic private choices", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("The Deck", "Goblins", "Random", true, 214);
  let state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Keep this hand").index);
  state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Cast Black Lotus").index);
  state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Cast Demonic Tutor").index);
  state = JSON.parse(game.state_json());

  assert.equal(state.decision.visibility, "Private");
  assert.equal(state.decision.minimum, 1);
  assert.equal(state.decision.maximum, 1);
  assert.ok(state.decision.options.length > 40);
  const choice = state.decision.options[0];
  game.choose_decision(state.decision.id, JSON.stringify([choice.id]));
  assert.equal(JSON.parse(game.state_json()).decision, null);

  game.free();
});

test("opponent pregame choices do not block the game with animations", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("The Deck", "Sligh", "Handcrafted", true, 0);
  const opening = JSON.parse(game.state_json());
  const keep = opening.actions.find((action) => action.label === "Keep this hand");
  game.act(keep.index);

  const afterKeep = JSON.parse(game.state_json());
  assert.ok(
    afterKeep.opponentActions.every(
      (action) =>
        action.label !== "Keep this hand" &&
        action.label !== "Take a mulligan" &&
        !action.label.startsWith("Bottom "),
    ),
    "keep, mulligan, and bottom choices stay out of the opponent animation queue",
  );

  game.free();
});

test("the Robots deck and its new card rules are packaged for the browser", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame(
    "Robots",
    "Robots",
    "Handcrafted",
    true,
    823380616,
  );
  const opening = JSON.parse(game.state_json());
  const juggernaut = opening.human.hand.find(
    (card) => card.name === "Juggernaut",
  );
  assert.ok(juggernaut, "the deterministic Robots hand includes Juggernaut");
  assert.equal(juggernaut.power, 5);
  assert.equal(juggernaut.toughness, 3);
  assert.match(juggernaut.rulesText, /attacks each combat if able/i);

  game.free();
});

test("the packaged Rust engine plays through browser actions", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
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
    opening.human.hand.every(
      (card) => typeof card.rulesText === "string" && card.rulesText.length > 0,
    ),
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
  assert.equal(
    afterKeep.step,
    "Precombat Main",
    "the web facade passes through an uneventful opening upkeep",
  );
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
    afterKeep.opponentActions.every(
      (action) =>
        action.state &&
        Array.isArray(action.state.battlefield) &&
        action.state.opponentActions.length === 0,
    ),
    "each opponent animation carries a non-recursive board snapshot",
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
  assert.ok(
    afterKeep.events.every(
      (event) =>
        !event.includes("CardInstanceId") &&
        !event.includes("active_player") &&
        !event.includes("card #"),
    ),
    "the game log contains player-facing descriptions rather than engine diagnostics",
  );

  game.free();
});

test("auto-pass declines an unavailable Chain Lightning copy", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("Goblins", "Goblins", "Handcrafted", true, 5);
  let state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Keep this hand").index);
  state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Play Mountain").index);
  state = JSON.parse(game.state_json());
  game.act(
    state.actions.find(
      (action) => action.label === "Cast Goblins of the Flarg",
    ).index,
  );
  state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Pass priority").index);
  state = JSON.parse(game.state_json());

  assert.equal(state.turn, 2);
  assert.equal(state.step, "Precombat Main");
  assert.ok(
    !state.actions.some((action) => action.label === "Don't copy Chain Lightning"),
    "an impossible copy choice does not interrupt the player",
  );
  assert.ok(
    state.events.some((event) => event.includes("Opponent cast Chain Lightning")),
  );
  assert.ok(state.events.some((event) => event === "Turn 2 · your turn"));

  game.free();
});

test("player-targeted spells identify a clickable player target", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("Goblins", "Sligh", "Handcrafted", true, 5);
  let state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Keep this hand").index);
  state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Play Mountain").index);
  state = JSON.parse(game.state_json());

  const bolt = state.actions.find(
    (action) =>
      action.label.startsWith("Cast Lightning Bolt") &&
      action.targetPlayer === "opponent",
  );
  assert.ok(bolt, "Lightning Bolt exposes the opponent as its board target");
  assert.equal(bolt.targetCardId, null);
  assert.equal(bolt.targetStackId, null);

  game.free();
});

test("the web facade skips combat when no attackers exist", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("Goblins", "Sligh", "Handcrafted", true, 5);
  let state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Keep this hand").index);
  state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Play Mountain").index);
  state = JSON.parse(game.state_json());
  game.set_phase_stop("Combat", true);
  state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.kind === "pass").index);
  state = JSON.parse(game.state_json());

  assert.equal(state.step, "Beginning Of Combat");
  game.act(state.actions.find((action) => action.kind === "pass").index);
  state = JSON.parse(game.state_json());

  assert.equal(state.step, "Postcombat Main");

  game.free();
});

test("attack all declares every currently legal attacker", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("Goblins", "Goblins", "Random", true, 5);
  let state;
  for (let step = 0; step < 20; step += 1) {
    state = JSON.parse(game.state_json());
    if (
      state.step === "Declare Attackers" &&
      state.actions.some((action) => action.label.startsWith("Attack with "))
    ) {
      break;
    }
    const next =
      state.actions.find((action) => action.label === "Keep this hand") ??
      state.actions.find((action) => action.label === "Play Mountain") ??
      state.actions.find((action) => action.label.startsWith("Cast Goblins of the Flarg")) ??
      state.actions.find((action) => action.kind === "pass") ??
      state.actions.find((action) => /^(Don't|Leave) /.test(action.label));
    assert.ok(next, `the attack-all fixture can advance from ${state.step}`);
    game.act(next.index);
  }

  const attackOptions = state.actions.filter((action) =>
    action.label.startsWith("Attack with "),
  );
  assert.ok(attackOptions.length > 0);
  game.set_phase_stop("Combat", true);
  game.attack_all();
  state = JSON.parse(game.state_json());
  assert.equal(
    state.battlefield.filter((card) => card.owner === "human" && card.attacking).length,
    attackOptions.length,
  );
  assert.ok(
    !state.actions.some((action) => action.label.startsWith("Attack with ")),
    "attacker declaration is finished by the bulk action",
  );
  game.free();
});

test("opponent mana taps are grouped with the spell they pay for", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
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
  assert.ok(
    afterKeep.opponentActions.length > 1,
    "the deterministic turn provides a multi-action animation sequence",
  );
  assert.notDeepEqual(
    afterKeep.opponentActions[0].state.battlefield,
    afterKeep.battlefield,
    "the first animation does not expose the final battlefield",
  );
  for (const source of paidAction.manaSources) {
    assert.equal(
      paidAction.state.battlefield.find(
        (card) => card.owner === "opponent" && card.name === source,
      )?.tapped,
      true,
      `${source} taps in the same snapshot as the paid action`,
    );
  }

  game.free();
});

test("casting a spell automatically taps available mana sources", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
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
  assert.equal(castVise.paymentAction, true);
  assert.deepEqual(
    castVise.manaSourceIds,
    [state.battlefield.find((card) => card.name === "Mox Ruby").id],
    "the browser can preview the exact automatic mana tap before committing",
  );
  game.act(castVise.index);

  state = JSON.parse(game.state_json());
  const mox = state.battlefield.find(
    (card) => card.owner === "human" && card.name === "Mox Ruby",
  );
  assert.equal(mox?.tapped, true);
  assert.equal(state.human.mana.red, 0);
  assert.equal(state.autopassEnabled, true);
  assert.equal(state.stack.length, 0, "your spell resolves without another UI priority prompt");

  game.free();
});

test("turning auto-pass off exposes priority over your own spell", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("Artifacts", "Goblins", "Handcrafted", true, 16);
  let state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Keep this hand").index);
  state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Cast Mox Ruby").index);
  state = JSON.parse(game.state_json());
  game.set_autopass(false);
  state = JSON.parse(game.state_json());
  game.act(
    state.actions.find((action) => action.label.startsWith("Cast Black Vise")).index,
  );

  state = JSON.parse(game.state_json());
  assert.equal(state.autopassEnabled, false);
  assert.equal(state.stack[0]?.name, "Black Vise");
  assert.ok(state.actions.some((action) => action.label === "Pass priority"));

  game.set_autopass(true);
  state = JSON.parse(game.state_json());
  assert.equal(state.autopassEnabled, true);
  assert.equal(state.stack.length, 0);
  assert.ok(state.battlefield.some((card) => card.name === "Black Vise"));
  game.free();
});

test("targeted permanent actions identify their clickable battlefield target", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
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
      state.actions.find((action) =>
        /^(Don't|Leave) /.test(action.label),
      );
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
  const stripMana = state.actions.find(
    (action) => action.label === "Tap Strip Mine for Colorless mana",
  );
  assert.ok(stripMana, "Strip Mine remains available as a colorless mana source");
  assert.equal(stripMana.manaAbility, true);
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

test("Mishra's Factory offers both modes and manual mana can be undone", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("Artifacts", "Sligh", "Handcrafted", true, 0);
  let state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Keep this hand").index);
  state = JSON.parse(game.state_json());
  game.act(
    state.actions.find((action) => action.label === "Cast Mox Sapphire").index,
  );
  state = JSON.parse(game.state_json());
  game.act(
    state.actions.find((action) => action.label.startsWith("Play Mishra's Factory"))
      .index,
  );
  state = JSON.parse(game.state_json());

  const factory = state.battlefield.find(
    (card) => card.owner === "human" && card.name === "Mishra's Factory",
  );
  const factoryActions = state.actions.filter(
    (action) => action.cardId === factory.id,
  );
  const mox = state.battlefield.find(
    (card) => card.owner === "human" && card.name === "Mox Sapphire",
  );
  assert.deepEqual(
    factoryActions.map((action) => action.label),
    [
      "Tap Mishra's Factory for Colorless mana",
      "Make Mishra's Factory a 2/2 creature",
    ],
  );
  assert.deepEqual(
    factoryActions.find((action) => !action.manaAbility).manaSourceIds,
    [mox.id],
    "auto-pay preserves the Factory when another source can animate it",
  );

  game.act(factoryActions.find((action) => action.manaAbility).index);
  state = JSON.parse(game.state_json());
  assert.equal(state.canUndoMana, true);
  assert.equal(
    state.battlefield.find((card) => card.id === factory.id).tapped,
    true,
  );
  assert.equal(state.human.mana.colorless, 1);

  game.undo_mana();
  state = JSON.parse(game.state_json());
  assert.equal(state.canUndoMana, false);
  assert.equal(
    state.battlefield.find((card) => card.id === factory.id).tapped,
    false,
  );
  assert.equal(state.human.mana.colorless, 0);

  const animate = state.actions.find(
    (action) => action.label === "Make Mishra's Factory a 2/2 creature",
  );
  game.set_phase_stop("Main 1", true);
  game.act(animate.index);
  state = JSON.parse(game.state_json());
  const animatedFactory = state.battlefield.find(
    (card) => card.id === factory.id,
  );
  assert.equal(animatedFactory.kind, "artifactcreature");
  assert.equal(animatedFactory.isLand, true);
  assert.equal(animatedFactory.power, 2);
  assert.equal(animatedFactory.toughness, 2);

  game.free();
});

test("X spells expose explicit affordable values to the browser", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("The Deck", "Goblins", "Random", true, 654);
  let state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Keep this hand").index);
  state = JSON.parse(game.state_json());
  game.act(
    state.actions.find((action) => action.label === "Play Mishra's Factory").index,
  );
  state = JSON.parse(game.state_json());
  game.act(state.actions.find((action) => action.label === "Cast Black Lotus").index);
  state = JSON.parse(game.state_json());

  const fireballs = state.actions.filter(
    (action) => action.spellAction && action.label.startsWith("Cast Fireball"),
  );
  assert.deepEqual(
    [...new Set(fireballs.map((action) => action.x))],
    [0, 1, 2, 3],
    "the UI can present every affordable value of X",
  );
  const twoTargetFireball = fireballs.find(
    (action) =>
      action.x === 2 &&
      action.targetCount === 2 &&
      action.targetPlayers.includes("human") &&
      action.targetPlayers.includes("opponent"),
  );
  assert.ok(twoTargetFireball, "the UI receives complete multi-target Fireball actions");
  assert.deepEqual(twoTargetFireball.targetCardIds, []);
  const fireballForThree = fireballs.find(
    (action) => action.x === 3 && action.targetPlayer === "opponent",
  );
  assert.ok(fireballForThree);
  game.act(fireballForThree.index);
  state = JSON.parse(game.state_json());
  assert.equal(state.opponent.life, 17);

  game.free();
});

test("phase stops override smooth UI auto-passing without changing engine steps", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("Goblins", "Sligh", "Handcrafted", true, 5);
  game.set_phase_stop("Beginning", true);
  let state = JSON.parse(game.state_json());
  assert.deepEqual(state.phaseStops, ["Beginning"]);
  game.act(state.actions.find((action) => action.label === "Keep this hand").index);
  state = JSON.parse(game.state_json());
  assert.equal(state.step, "Upkeep");
  assert.ok(state.actions.some((action) => action.label === "Pass priority"));

  game.set_phase_stop("Beginning", false);
  state = JSON.parse(game.state_json());
  assert.deepEqual(state.phaseStops, []);
  game.free();
});

test("Orcish Mechanics exposes creature targets and distinct artifact costs", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("Artifacts", "Sligh", "Handcrafted", true, 7);
  let state;
  let mechanics;
  let creatureTargets;
  for (let step = 0; step < 160; step += 1) {
    state = JSON.parse(game.state_json());
    mechanics = state.battlefield.find(
      (card) =>
        card.owner === "human" &&
        card.name === "Orcish Mechanics" &&
        !card.tapped,
    );
    creatureTargets = mechanics
      ? state.actions.filter(
          (action) =>
            action.cardId === mechanics.id &&
            action.targetCardId != null &&
            state.battlefield.some(
              (card) =>
                card.id === action.targetCardId &&
                card.owner === "opponent" &&
                card.kind.includes("creature"),
            ),
        )
      : [];
    if (creatureTargets.length >= 2) break;

    const actions = state.actions;
    const next =
      actions.find((action) => action.label === "Keep this hand") ??
      actions.find((action) => action.label.startsWith("Cast Mox ")) ??
      actions.find((action) => action.label === "Cast Black Lotus") ??
      actions.find((action) => action.label === "Play Mountain") ??
      actions.find((action) => action.label.startsWith("Play Mishra")) ??
      actions.find((action) => action.label.startsWith("Play Strip")) ??
      actions.find((action) => action.label.startsWith("Cast Orcish Mechanics")) ??
      actions.find((action) => action.label.startsWith("Cast Sol Ring")) ??
      actions.find((action) => action.label.startsWith("Cast Black Vise")) ??
      actions.find((action) => action.label.startsWith("Cast Copper Tablet")) ??
      actions.find((action) => action.label.startsWith("Cast Ankh")) ??
      actions.find((action) => /^(Don't|Leave) /.test(action.label)) ??
      actions.find((action) => action.kind === "pass");
    assert.ok(next, `seed 7 can advance from turn ${state.turn} ${state.step}`);
    game.act(next.index);
  }

  assert.ok(mechanics, "Orcish Mechanics reaches the battlefield");
  assert.equal(
    new Set(creatureTargets.map((action) => action.targetCardId)).size,
    1,
    "the opposing creature is a legal target",
  );
  assert.ok(
    new Set(creatureTargets.map((action) => action.label)).size >= 2,
    "each sacrifice choice has a distinct action label",
  );
  assert.ok(
    creatureTargets.every((action) => action.label.includes("sacrifice")),
    "the interface can name the artifact paid for each action",
  );

  game.free();
});

test("the pass button label reports the engine's real auto-pass destination", async () => {
  const bytes = await readFile(
    new URL("../app/wasm/penta_wasm_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: bytes });

  const game = new WebGame("Goblins", "Sligh", "Handcrafted", true, 9394);
  const currentState = () => JSON.parse(game.state_json());
  const pass = (state) =>
    game.act(state.actions.find((action) => action.label === "Pass priority").index);

  let state = currentState();
  game.act(state.actions.find((action) => action.label === "Keep this hand").index);
  state = currentState();
  assert.equal(state.step, "Precombat Main");
  assert.equal(
    state.passLabel,
    "Pass to main 2",
    "land plays waiting in Main 2 keep the pass from promising the whole turn",
  );
  pass(state);
  state = currentState();
  assert.equal(state.step, "Postcombat Main");

  game.set_phase_stop("Ending", true);
  state = currentState();
  assert.equal(state.passLabel, "Pass to end step");
  pass(state);
  state = currentState();
  assert.equal(state.step, "End");

  game.set_phase_stop("Ending", false);
  state = currentState();
  assert.equal(state.passLabel, "Pass the turn");
  pass(state);
  state = currentState();
  assert.equal(state.step, "Precombat Main");
  assert.equal(state.active, "You");
  assert.equal(state.turn, 2, "the promised pass really ends the turn");

  game.set_autopass(false);
  state = currentState();
  assert.equal(
    state.passLabel,
    "Pass to combat",
    "with auto-pass off the label only promises the next window",
  );
  pass(state);
  state = currentState();
  assert.equal(state.step, "Beginning Of Combat");

  game.free();
});
