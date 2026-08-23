import assert from "node:assert/strict";
import test, { after } from "node:test";

import {
  durableState,
  installRoomGlobals,
  loadGameRoom,
  request,
  restoreRoomGlobals,
} from "./game-room-support.mjs";

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

  isFinished() {
    return this.result !== null;
  }

  resultJson() {
    return this.result === null ? undefined : JSON.stringify(this.result);
  }

  loseOnTime(seat, reason) {
    timeouts.push({ seat, reason });
    this.result = { loser: seat, reason };
  }

  state_json() {
    return JSON.stringify({ result: this.result });
  }
}

installRoomGlobals({
  WebGame: TestWebGame,
  // A zero budget for the bot, so `alarm()` finds the clock already expired
  // rather than rearming -- these tests are about what the alarm does when it
  // fires, not about waiting for one.
  presence: { moveBudgetMs: (seat) => (seat === "bot" ? 0 : 10_000) },
});
after(restoreRoomGlobals);

let GameRoom;
test.before(async () => {
  ({ GameRoom } = await loadGameRoom());
});

async function startRoom() {
  const room = new GameRoom(durableState());
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
