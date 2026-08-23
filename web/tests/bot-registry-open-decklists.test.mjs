import assert from "node:assert/strict";
import test, { after, before } from "node:test";

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
 * Open decklists on the registry side: `discloseDeck` is stored on a bot's
 * registration, echoed back by register/heartbeat, and -- only once a
 * challenge actually succeeds -- forwarded to the room that bot just
 * claimed, via `disclose-bot-deck`. A bot that never opts in behaves exactly
 * as before, and the room is still told, but with `discloseDeck: false`,
 * which `game-room.ts` treats the same as never having been told at all.
 */

before(installRegistryGlobals);
after(restoreRegistryGlobals);

test("a bot that never opts in registers and heartbeats exactly as before", async () => {
  const { BotRegistry } = await loadBotRegistry();
  const registry = new BotRegistry(durableState(), fakeEnv(fakeRoom()));
  const registered = await register(registry);
  assert.equal(registered.discloseDeck, false);
  const beat = await heartbeat(registry, registered.id, { token: registered.token });
  assert.equal(beat.discloseDeck, false);
});

test("a bot can opt in at registration, and it is echoed back", async () => {
  const { BotRegistry } = await loadBotRegistry();
  const registry = new BotRegistry(durableState(), fakeEnv(fakeRoom()));
  const registered = await register(registry, { discloseDeck: true });
  assert.equal(registered.discloseDeck, true);
});

test("a heartbeat that omits discloseDeck leaves the prior declaration alone", async () => {
  const { BotRegistry } = await loadBotRegistry();
  const registry = new BotRegistry(durableState(), fakeEnv(fakeRoom()));
  const registered = await register(registry, { discloseDeck: true });
  const beat = await heartbeat(registry, registered.id, { token: registered.token });
  assert.equal(beat.discloseDeck, true, "omitting the field is not the same as turning it off");
});

test("a heartbeat can turn the opt-in on or back off", async () => {
  const { BotRegistry } = await loadBotRegistry();
  const registry = new BotRegistry(durableState(), fakeEnv(fakeRoom()));
  const registered = await register(registry);
  const on = await heartbeat(registry, registered.id, {
    token: registered.token,
    discloseDeck: true,
  });
  assert.equal(on.discloseDeck, true);
  const off = await heartbeat(registry, registered.id, {
    token: registered.token,
    discloseDeck: false,
  });
  assert.equal(off.discloseDeck, false);
});

test("a successful challenge tells the room the claiming bot's opt-in", async () => {
  const { BotRegistry } = await loadBotRegistry();
  const room = fakeRoom();
  const registry = new BotRegistry(durableState(), fakeEnv(room));
  const registered = await register(registry, { discloseDeck: true });
  await heartbeat(registry, registered.id, { token: registered.token });
  const response = await registry.fetch(
    request(`${registered.id}/challenge`, {
      body: { room: "room-1", token: "bot-seat-token" },
    }),
  );
  const challenge = await response.json();
  assert.equal(challenge.discloseDeck, true);
  const disclosure = room.calls.find((call) => call.route === "disclose-bot-deck");
  assert.ok(disclosure, "the registry must tell the room about the opt-in");
  assert.deepEqual(disclosure.body, { discloseDeck: true });
});

test("a bot that never opted in still gets a disclose-bot-deck call, declaring false", async () => {
  const { BotRegistry } = await loadBotRegistry();
  const room = fakeRoom();
  const registry = new BotRegistry(durableState(), fakeEnv(room));
  const registered = await register(registry);
  await heartbeat(registry, registered.id, { token: registered.token });
  await registry.fetch(
    request(`${registered.id}/challenge`, {
      body: { room: "room-1", token: "bot-seat-token" },
    }),
  );
  const disclosure = room.calls.find((call) => call.route === "disclose-bot-deck");
  assert.ok(disclosure);
  assert.deepEqual(disclosure.body, { discloseDeck: false });
});
