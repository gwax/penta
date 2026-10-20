import assert from "node:assert/strict";
import test from "node:test";
import { SessionRules } from "./session-rules.mjs";

const rule = (definition = "d", text = "Draw a card.") =>
  ({ definition, available: true, name: "Card", printed: { rulesText: text }, reference: `r:${definition}` });
const packet = items => ({ presentation: "pilot-v2", status: "ready", host: { receipt: { updates: [1] } },
  position: { rules: { items, total: items.length, reference: null }, choices: { items: [2] }, updates: [3] } });
function deliver(cache, value) {
  const prepared = cache.prepare(value);
  prepared.commit();
  return prepared.packet;
}

test("only repeated semantic printed summaries are omitted, without changing other state or inputs", () => {
  const cache = new SessionRules();
  const original = packet([rule()]);
  const before = structuredClone(original);
  assert.deepEqual(deliver(cache, original), before);
  const repeated = deliver(cache, original);
  assert.deepEqual(repeated.position.rules.items, [{ ...rule(), printed: null, printedPreviouslySent: true }]);
  assert.deepEqual({ ...repeated.position, rules: undefined }, { ...original.position, rules: undefined });
  assert.deepEqual(repeated.host, original.host);
  assert.deepEqual(original, before);
  assert.deepEqual(deliver(cache, packet([rule("d", "Draw two cards."), rule("new")])), packet([rule("d", "Draw two cards."), rule("new")]));
  assert.deepEqual(deliver(cache, original), original, "changing back also reprints");
});

test("object order is ignored, but array order and changed values are meaningful", () => {
  const cache = new SessionRules();
  const first = { ...rule(), printed: { rulesText: "a", manaCost: { blue: 1, generic: 2 }, parts: [1, 2] } };
  deliver(cache, packet([first]));
  const reordered = { ...first, printed: { parts: [1, 2], manaCost: { generic: 2, blue: 1 }, rulesText: "a" } };
  assert.equal(deliver(cache, packet([reordered])).position.rules.items[0].printedPreviouslySent, true);
  const changed = { ...first, printed: { ...first.printed, parts: [2, 1] } };
  assert.deepEqual(deliver(cache, packet([changed])).position.rules.items[0], changed);
});

test("unavailable, paged and inspected rules never imply delivery of omitted printed contents", () => {
  const cache = new SessionRules();
  const paged = packet([]);
  paged.position.rules = { items: [], total: 1, reference: "rules:page" };
  deliver(cache, paged);
  deliver(cache, { items: [rule()], reference: "rules:page" });
  assert.deepEqual(deliver(cache, packet([rule()])), packet([rule()]));
  const unavailable = { ...rule(), available: false, printed: null };
  assert.deepEqual(deliver(cache, packet([unavailable])), packet([unavailable]));
  assert.deepEqual(deliver(cache, packet([rule()])), packet([rule()]));
});

test("uncommitted output, reset, independent sessions, exact and pilot-v1 never suppress initial details", () => {
  const cache = new SessionRules();
  const value = packet([rule()]);
  cache.prepare(value); // Represents a failed output write.
  assert.deepEqual(deliver(cache, value), value);
  cache.reset();
  assert.deepEqual(deliver(cache, value), value);
  assert.deepEqual(deliver(new SessionRules(), value), value);
  for (const presentation of ["exact", "pilot-v1"]) {
    const legacy = { ...value, presentation };
    assert.deepEqual(deliver(cache, legacy), legacy);
    assert.deepEqual(deliver(cache, legacy), legacy);
  }
});
