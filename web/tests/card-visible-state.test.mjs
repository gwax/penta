import assert from "node:assert/strict";
import test from "node:test";

import {
  cardChoiceLabel,
  cardChoiceStateKey,
} from "../app/card-visible-state.mjs";

test("a chosen card name is visible and separates otherwise identical piles", () => {
  const jace = { chosenCardName: "Jace, Memory Adept" };
  const domri = { chosenCardName: "Domri Rade" };

  assert.equal(cardChoiceLabel(jace), "Named card: Jace, Memory Adept");
  assert.notEqual(cardChoiceStateKey(jace), cardChoiceStateKey(domri));
});

test("chosen creature types remain visible and part of pile identity", () => {
  const human = { chosenCreatureType: "Human" };
  const angel = { chosenCreatureType: "Angel" };

  assert.equal(cardChoiceLabel(human), "Chosen type: Human");
  assert.notEqual(cardChoiceStateKey(human), cardChoiceStateKey(angel));
});
