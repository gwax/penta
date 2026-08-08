import assert from "node:assert/strict";
import test from "node:test";

import {
  buildMulliganBottomPicker,
  isMulliganBottomAction,
  resolveMulliganBottomAction,
  toggleMulliganBottomCard,
} from "../app/mulligan-actions.mjs";

const action = (index, bottomCardIds = []) => ({
  index,
  label: `Action ${index}`,
  kind: "primary",
  bottomCardIds,
});

test("mulligan actions become one bounded picker", () => {
  const ordinary = action(0);
  const pair12 = action(1, [1, 2]);
  const pair13 = action(2, [1, 3]);
  const pair23 = action(3, [2, 3]);
  const picker = buildMulliganBottomPicker([ordinary, pair12, pair13, pair23]);

  assert.ok(picker);
  assert.equal(picker.required, 2);
  assert.deepEqual(picker.candidateCardIds, [1, 2, 3]);
  assert.deepEqual(picker.actions, [pair12, pair13, pair23]);
  assert.equal(isMulliganBottomAction(ordinary), false);
  assert.equal(isMulliganBottomAction(pair12), true);
});

test("mulligan selection toggles, caps, and resolves without depending on order", () => {
  const pair12 = action(1, [1, 2]);
  const pair13 = action(2, [1, 3]);
  const pair23 = action(3, [2, 3]);
  const picker = buildMulliganBottomPicker([pair12, pair13, pair23]);

  let selection = [];
  assert.equal(
    toggleMulliganBottomCard(picker, selection, 99),
    selection,
    "a card outside the backend candidates is ignored",
  );
  selection = toggleMulliganBottomCard(picker, selection, 2);
  selection = toggleMulliganBottomCard(picker, selection, 1);
  assert.deepEqual(selection, [2, 1]);
  assert.equal(resolveMulliganBottomAction(picker, selection), pair12);

  assert.equal(
    toggleMulliganBottomCard(picker, selection, 3),
    selection,
    "a third card is ignored at the exact bound",
  );
  assert.deepEqual(toggleMulliganBottomCard(picker, selection, 2), [1]);
  assert.equal(resolveMulliganBottomAction(picker, [1]), undefined);
  assert.equal(resolveMulliganBottomAction(picker, [1, 1]), undefined);
  assert.equal(resolveMulliganBottomAction(picker, [1, 4]), undefined);
});

test("duplicate card names remain distinct because the picker uses object IDs", () => {
  const firstCopyWithLand = action(1, [41, 90]);
  const secondCopyWithLand = action(2, [42, 90]);
  const bothCopies = action(3, [41, 42]);
  const picker = buildMulliganBottomPicker([
    firstCopyWithLand,
    secondCopyWithLand,
    bothCopies,
  ]);

  assert.deepEqual(picker?.candidateCardIds, [41, 90, 42]);
  assert.equal(
    resolveMulliganBottomAction(picker, [42, 90]),
    secondCopyWithLand,
  );
});

test("missing or inconsistent backend metadata falls back cleanly", () => {
  assert.equal(buildMulliganBottomPicker([action(0)]), null);
  assert.equal(
    buildMulliganBottomPicker([action(1, [1]), action(2, [1, 2])]),
    null,
  );
  assert.equal(buildMulliganBottomPicker([action(1, [1, 1])]), null);
  assert.equal(
    buildMulliganBottomPicker([action(1, [1, 2]), action(2, [2, 1])]),
    null,
  );
});

test("partial selections cannot enter a dead end", () => {
  const picker = buildMulliganBottomPicker([
    action(1, [1, 2]),
    action(2, [3, 4]),
  ]);
  const selection = toggleMulliganBottomCard(picker, [], 1);

  assert.equal(
    toggleMulliganBottomCard(picker, selection, 3),
    selection,
    "both cards are candidates, but they do not form a legal pair",
  );
  assert.deepEqual(toggleMulliganBottomCard(picker, selection, 2), [1, 2]);
});
