import assert from "node:assert/strict";
import test, { after, before } from "node:test";

import { PRESENCE_MS } from "../worker/bot-presence.mjs";
import {
  durableState,
  fakeEnv,
  fakeRoom,
  heartbeat,
  installRegistryGlobals,
  loadBotRegistry,
  register,
  request,
  restoreRegistryGlobals,
} from "./bot-registry-support.mjs";

/**
 * When the registry decides a bot has abandoned a game.
 *
 * Issue 95: a bot whose play loop did not heartbeat -- which both shipped
 * examples encouraged -- went silent here for the whole length of every game
 * and had that game ended against it within about a presence window. The
 * player saw "you win, opponent ran out of time" while the bot was sitting
 * there playing. Heartbeating from the play loop is the bot's half of the
 * fix; this file covers the server's half, which is that the registry asks
 * the rooms before concluding anybody is gone.
 */

before(installRegistryGlobals);
after(restoreRegistryGlobals);

/** Registers a heartbeating bot and puts it in `room`. */
async function botInAGame(registry, room = "room-1") {
  const registered = await register(registry);
  await heartbeat(registry, registered.id, { token: registered.token });
  await registry.fetch(
    request(`${registered.id}/challenge`, {
      body: { room, token: "bot-seat-token" },
    }),
  );
  return registered;
}

/** Ages every stored registration past the presence window. */
async function goSilent(state) {
  for (const [key, bot] of state.storage.values) {
    await state.storage.put(key, { ...bot, lastSeen: Date.now() - PRESENCE_MS * 2 });
  }
}

function timeouts(room) {
  return room.calls.filter((call) => call.route === "lose-on-time");
}

test("a bot still moving in its room is not declared gone when its heartbeat lapses", async () => {
  const { BotRegistry } = await loadBotRegistry();
  const room = fakeRoom({ "bot-activity": { lastSeen: Date.now() } });
  const state = durableState();
  const registry = new BotRegistry(state, fakeEnv(room));
  await botInAGame(registry);
  await goSilent(state);

  await registry.alarm();

  assert.deepEqual(timeouts(room), [], "the bot is visibly playing the game it owes");
  assert.ok(
    state.storage.alarms.length > 0,
    "and the registry keeps watching it rather than forgetting about it",
  );
});

test("a bot silent in its room too loses the game it owes", async () => {
  const { BotRegistry } = await loadBotRegistry();
  const room = fakeRoom({
    "bot-activity": { lastSeen: Date.now() - PRESENCE_MS * 2 },
  });
  const state = durableState();
  const registry = new BotRegistry(state, fakeEnv(room));
  await botInAGame(registry);
  await goSilent(state);

  await registry.alarm();

  const [timeout] = timeouts(room);
  assert.ok(timeout, "a bot that is gone everywhere has abandoned its opponent");
  assert.equal(timeout.body.seat, "bot");
  assert.match(
    timeout.body.reason,
    /Fizzbot stopped answering/,
    "and the reason names the bot, so the player is not just told about a clock",
  );
});

test("a room that has never heard from its bot is not evidence the bot is there", async () => {
  const { BotRegistry } = await loadBotRegistry();
  const room = fakeRoom({ "bot-activity": { lastSeen: null } });
  const state = durableState();
  const registry = new BotRegistry(state, fakeEnv(room));
  await botInAGame(registry);
  await goSilent(state);

  await registry.alarm();

  assert.equal(timeouts(room).length, 1, "silence is not a vouch");
});

test("a room that cannot answer falls back to the heartbeat rather than to mercy", async () => {
  const { BotRegistry } = await loadBotRegistry();
  const room = {
    calls: [],
    async fetch(request) {
      const route = new URL(request.url).pathname.split("/").pop();
      if (route === "bot-activity") throw new Error("room unreachable");
      room.calls.push({ route, body: await request.json() });
      return { ok: true, json: async () => ({ ok: true }) };
    },
  };
  const state = durableState();
  const registry = new BotRegistry(state, fakeEnv(room));
  await botInAGame(registry);
  await goSilent(state);

  await registry.alarm();

  assert.equal(timeouts(room).length, 1, "an unreachable room vouches for nobody");
});

test("a heartbeating bot is never asked about at its rooms", async () => {
  const { BotRegistry } = await loadBotRegistry();
  const room = fakeRoom();
  const registry = new BotRegistry(durableState(), fakeEnv(room));
  await botInAGame(registry);

  await registry.alarm();

  assert.deepEqual(timeouts(room), []);
  assert.deepEqual(
    room.calls.filter((call) => call.route === "bot-activity"),
    [],
    "the heartbeat already answered the question; do not pay for a second answer",
  );
});

test("one abandoned room does not cost a bot the rooms it is still playing", async () => {
  const { BotRegistry } = await loadBotRegistry();
  const live = fakeRoom({ "bot-activity": { lastSeen: Date.now() } });
  const dead = fakeRoom({
    "bot-activity": { lastSeen: Date.now() - PRESENCE_MS * 2 },
  });
  const state = durableState();
  const registry = new BotRegistry(state, fakeEnv(live, { "room-2": dead }));
  const registered = await botInAGame(registry, "room-1");
  // A second invitation reaches a bot through an event pairing rather than a
  // challenge, which refuses a bot that already owes somebody a game.
  const stored = await state.storage.get(`bot:${registered.id}`);
  await state.storage.put(`bot:${registered.id}`, {
    ...stored,
    invites: [
      ...stored.invites,
      { room: "room-2", reason: "event", at: Date.now(), token: "bot-seat-token" },
    ],
  });
  await goSilent(state);

  await registry.alarm();

  assert.deepEqual(timeouts(live), [], "the room it is playing is left alone");
  assert.equal(timeouts(dead).length, 1, "the room it walked away from is not");
  const after = await state.storage.get(`bot:${registered.id}`);
  assert.deepEqual(
    after.invites.map((invite) => invite.room),
    ["room-1"],
    "and it still owes the game it is playing",
  );
});
