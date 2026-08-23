import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test, { after } from "node:test";

import ts from "typescript";

/**
 * What the room knows about its bot, and what it says about the ending.
 *
 * Issue 95: the registry ended games against bots that were sitting right
 * there playing them, because the only evidence it weighed was a heartbeat
 * the bot's own play loop had stopped sending. The room is where the
 * counter-evidence lives -- a bot polling and moving is a bot present -- and
 * `bot-activity` is how the registry asks. The same issue's second half is
 * here too: the reason a game ended has always travelled with the command
 * and been thrown away one step before the player, so a bot that vanished
 * and a bot that was slow read identically.
 *
 * Like the other room suites, `GameRoom` is transpiled from source with its
 * worker-only imports swapped for injected globals.
 */

class MemoryStorage {
  values = new Map();
  alarm = null;

  async get(key) {
    const value = this.values.get(key);
    return value === undefined ? undefined : structuredClone(value);
  }

  async put(key, value) {
    this.values.set(key, structuredClone(value));
  }

  async delete(key) {
    return this.values.delete(key);
  }

  async setAlarm(time) {
    this.alarm = time;
  }

  async deleteAlarm() {
    this.alarm = null;
  }
}

class TestResponse {
  constructor(body = null, init = {}) {
    this.status = init.status ?? 200;
    this.body = body === null ? "" : String(body);
  }

  static json(value, init = {}) {
    return new TestResponse(JSON.stringify(value), init);
  }

  async json() {
    return JSON.parse(this.body);
  }
}

/** Every timeout the room has handed the engine, newest last. */
const timeouts = [];

/**
 * Just enough of `WebGame` for the bot seat to hold a decision until a
 * timeout ends the game -- and to record what reason it was given, which is
 * the whole thing under test on that side.
 */
class TestWebGame {
  result = null;

  opponentObserveJson() {
    return JSON.stringify({ decision: { actor: "bot", kind: "Priority" } });
  }

  opponentIsDeciding() {
    return this.result === null;
  }

  loseOnTime(seat, reason) {
    timeouts.push({ seat, reason });
    this.result = { loser: seat, reason };
  }

  state_json() {
    return JSON.stringify({ result: this.result });
  }
}

class TestHostedGame {
  static replayVersion() {
    return 1;
  }

  static simulationFingerprint() {
    return "test-fingerprint";
  }

  static engineVersion() {
    return "test-engine";
  }

  static protocolVersion() {
    return 1;
  }
}

const originalResponse = globalThis.Response;
globalThis.Response = TestResponse;
globalThis.__gameRoomEngine = async () => ({
  WebGame: TestWebGame,
  HostedGame: TestHostedGame,
});
globalThis.__replayCompatibilityError = () => null;
globalThis.__botPresence = {
  FINISHED_ROOM_MS: 60_000,
  // A zero budget for the bot, so `alarm()` finds the clock already expired
  // rather than rearming -- these tests are about what the alarm does when it
  // fires, not about waiting for one.
  moveBudgetMs: (seat) => (seat === "bot" ? 0 : 10_000),
};

after(() => {
  globalThis.Response = originalResponse;
  delete globalThis.__gameRoomEngine;
  delete globalThis.__replayCompatibilityError;
  delete globalThis.__botPresence;
});

async function loadGameRoom() {
  let source = await readFile(
    new URL("../worker/game-room.ts", import.meta.url),
    "utf8",
  );
  const replacements = [
    [
      'import { type EngineModule, engine } from "./engine";',
      "const engine = globalThis.__gameRoomEngine;",
    ],
    [
      'import { replayCompatibilityError } from "./replay-compatibility.mjs";',
      "const replayCompatibilityError = globalThis.__replayCompatibilityError;",
    ],
    [
      'import { FINISHED_ROOM_MS, moveBudgetMs } from "./bot-presence.mjs";',
      "const { FINISHED_ROOM_MS, moveBudgetMs } = globalThis.__botPresence;",
    ],
  ];
  for (const [from, to] of replacements) {
    assert.ok(source.includes(from), `test loader expected ${from}`);
    source = source.replace(from, to);
  }
  const javascript = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
    },
  }).outputText;
  const encoded = Buffer.from(javascript).toString("base64");
  return import(`data:text/javascript;base64,${encoded}`);
}

function durableState(storage) {
  return {
    storage,
    blockConcurrencyWhile: (callback) => callback(),
  };
}

function request(route, { token, body } = {}) {
  const headers = token ? { "x-penta-token": token } : undefined;
  return new Request(`https://room.test/${route}`, {
    method: body === undefined ? "GET" : "POST",
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
}

async function startRoom() {
  const room = new GameRoom(durableState(new MemoryStorage()));
  const response = await room.fetch(
    request("start", {
      body: {
        humanDeck: "The Deck",
        botDeck: "Sligh",
        botPolicy: "external",
        humanFirst: true,
        seed: 7,
      },
    }),
  );
  return { room, ...(await response.json()) };
}

async function botActivity(room) {
  const response = await room.fetch(request("bot-activity"));
  return (await response.json()).lastSeen;
}

let GameRoom;
test.before(async () => {
  ({ GameRoom } = await loadGameRoom());
});
test.beforeEach(() => {
  timeouts.length = 0;
});

test("a room that has not heard from its bot says so", async () => {
  const { room } = await startRoom();
  assert.equal(await botActivity(room), null);
});

test("a bot polling its room counts as the bot being there", async () => {
  const { room, botToken } = await startRoom();
  const before = Date.now();
  await room.fetch(request("opponent", { token: botToken }));
  const lastSeen = await botActivity(room);
  assert.ok(
    typeof lastSeen === "number" && lastSeen >= before,
    `polling is presence, got ${lastSeen}`,
  );
});

test("the human's own traffic is not evidence about the bot", async () => {
  const { room, humanToken } = await startRoom();
  await room.fetch(request("state", { token: humanToken }));
  assert.equal(await botActivity(room), null);
});

test("a request with no token is not evidence about the bot", async () => {
  const { room } = await startRoom();
  await room.fetch(request("state"));
  assert.equal(await botActivity(room), null);
});

test("a host's reason for ending the game reaches the engine", async () => {
  const { room } = await startRoom();
  await room.fetch(
    request("lose-on-time", {
      body: { seat: "bot", reason: "Fizzbot stopped answering" },
    }),
  );
  assert.deepEqual(timeouts, [
    { seat: "bot", reason: "Fizzbot stopped answering" },
  ]);
});

test("the move clock's own timeout still reads as an expired clock", async () => {
  const { room } = await startRoom();
  await room.alarm();
  assert.deepEqual(timeouts, [{ seat: "bot", reason: "ran out of time" }]);
});

test("a timeout is journalled with its reason, so a replay ends the same way", async () => {
  const { room, humanToken } = await startRoom();
  await room.fetch(
    request("lose-on-time", {
      body: { seat: "bot", reason: "Fizzbot stopped answering" },
    }),
  );
  const record = await (await room.fetch(request("record", { token: humanToken }))).json();
  assert.deepEqual(record.commands, [
    { t: "loseOnTime", seat: "bot", reason: "Fizzbot stopped answering" },
  ]);
});
