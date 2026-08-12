import assert from "node:assert/strict";
import test from "node:test";

import {
  INVITE_MS,
  PRESENCE_MS,
  isOnline,
  liveInvites,
  publicBot,
} from "../worker/bot-presence.mjs";

const NOW = 1_000_000;

function bot(overrides = {}) {
  return {
    id: "abc",
    name: "Fizzbot",
    deck: "Sligh",
    lastSeen: NOW,
    invites: [],
    ...overrides,
  };
}

test("a bot is online for exactly as long as its heartbeat lease", () => {
  assert.equal(isOnline(NOW, NOW), true);
  assert.equal(isOnline(NOW - PRESENCE_MS, NOW), true, "the boundary is inclusive");
  assert.equal(isOnline(NOW - PRESENCE_MS - 1, NOW), false);
});

test("a bot that never heartbeated is offline, so registering is not being online", () => {
  assert.equal(isOnline(0, NOW), false);
  assert.equal(publicBot(bot({ lastSeen: 0 }), NOW).online, false);
});

test("an invitation nobody picked up expires, freeing the bot for the next challenger", () => {
  const fresh = { room: "r1", reason: "challenge", at: NOW - 1_000 };
  const stale = { room: "r2", reason: "challenge", at: NOW - INVITE_MS };
  assert.deepEqual(liveInvites([fresh, stale], NOW), [fresh]);
  assert.equal(publicBot(bot({ invites: [stale] }), NOW).busy, false);
  assert.equal(publicBot(bot({ invites: [fresh] }), NOW).busy, true);
});

test("the public view carries no token, whatever else the record holds", () => {
  const view = publicBot(bot({ token: "secret" }), NOW);
  assert.deepEqual(Object.keys(view).sort(), ["busy", "deck", "id", "name", "online"]);
  assert.equal(JSON.stringify(view).includes("secret"), false);
});
