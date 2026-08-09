import assert from "node:assert/strict";
import test from "node:test";

import {
  abilityOriginKey,
  actionHasTargets,
  buildAbilityActionGroups,
} from "../app/ability-actions.mjs";

const action = (index, abilityId, targets = {}) => ({
  index,
  label: "Activate Jace, Memory Adept",
  kind: "primary",
  cardId: 41,
  ability: { kind: "printed", definition: 700, partId: 0, abilityId },
  abilitySummary: [
    "+1: Draw a card. Target player mills a card.",
    "0: Target player mills ten cards.",
    "−7: Any number of target players each draw twenty cards.",
  ][abilityId],
  targetCount: 0,
  targetCardIds: [],
  targetPlayers: [],
  targetStackIds: [],
  targetSelections: [],
  sacrificeCardIds: [],
  bottomCardIds: [],
  ...targets,
});

test("Jace optional targets stay grouped with the exact -7 ability", () => {
  const zeroTargets = action(20, 2);
  const oneTarget = action(21, 2, {
    targetPlayer: "human",
    targetCount: 1,
    targetPlayers: ["human"],
  });
  const twoTargets = action(22, 2, {
    targetPlayer: "human",
    targetCount: 2,
    targetPlayers: ["human", "opponent"],
  });
  const actions = [
    action(10, 0, { targetPlayer: "opponent", targetCount: 1 }),
    action(11, 1, { targetPlayer: "opponent", targetCount: 1 }),
    zeroTargets,
    oneTarget,
    twoTargets,
  ];

  const groups = buildAbilityActionGroups(actions);
  assert.equal(groups.length, 3);
  const ultimate = groups.find((group) => group.key.endsWith(":2"));
  assert.ok(ultimate);
  assert.deepEqual(ultimate.targetless, [zeroTargets]);
  assert.deepEqual(ultimate.targeted, [oneTarget, twoTargets]);
  assert.equal(actionHasTargets(zeroTargets), false);
  assert.equal(actionHasTargets(twoTargets), true);
});

test("ability origin keys preserve definition, part, and grant provenance", () => {
  const printed = { kind: "printed", definition: 10, partId: 2, abilityId: 0 };
  assert.notEqual(
    abilityOriginKey(printed),
    abilityOriginKey({ ...printed, definition: 11 }),
  );
  assert.notEqual(
    abilityOriginKey(printed),
    abilityOriginKey({ ...printed, partId: 3 }),
  );
  assert.notEqual(
    abilityOriginKey({
      kind: "granted",
      source: 8,
      sourceDefinition: 10,
      sourcePartId: 0,
      sourceAbilityId: 1,
      grantId: 0,
    }),
    abilityOriginKey({
      kind: "granted",
      source: 8,
      sourceDefinition: 10,
      sourcePartId: 0,
      sourceAbilityId: 1,
      grantId: 1,
    }),
  );
});
