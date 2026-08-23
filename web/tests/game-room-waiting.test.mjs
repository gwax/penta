import assert from "node:assert/strict";
import test, { after } from "node:test";

import { MAX_WAIT_MS, waitBudgetMs } from "../worker/bot-presence.mjs";
import {
  durableState,
  installRoomGlobals,
  loadGameRoom,
  request,
  restoreRoomGlobals,
} from "./game-room-support.mjs";

/**
 * Waiting on `opponent` instead of asking again.
 *
 * A bot is asked for a decision at every priority pass, which is many times
 * per turn, and a poller pays a whole poll interval for each one. Measured on
 * a development server, handing a decision to a bot polling at 250ms took
 * 221ms on average and 496ms at the 90th percentile; parking the request took
 * 9ms. This is where that comes from, and what it must not break: a bot that
 * asks for no wait is answered exactly as before, and a bot that asks for one
 * cannot park so long that the registry mistakes waiting for absence.
 *
 * Like the other room suites, `GameRoom` is transpiled from source with its
 * worker-only imports swapped for injected globals.
 */

/**
 * A `WebGame` whose decision seat the test moves by hand, so a wait can be
 * started, observed as still waiting, and then satisfied on purpose.
 */
class TestWebGame {
  deciding = false;
  finished = false;

  opponentObserveJson() {
    return JSON.stringify({ decision: { actor: "bot", kind: "Priority" } });
  }

  opponentIsDeciding() {
    return this.deciding;
  }

  isFinished() {
    return this.finished;
  }

  resultJson() {
    return this.finished ? JSON.stringify({ outcome: "win" }) : undefined;
  }

  act() {}
  opponentAct() {}
  loseOnTime() {
    this.finished = true;
    this.deciding = false;
  }

  state_json() {
    return JSON.stringify({ result: this.finished ? { outcome: "win" } : null });
  }
}

let built = null;

installRoomGlobals({
  WebGame: function TrackedWebGame() {
    built = new TestWebGame();
    return built;
  },
  presence: { moveBudgetMs: () => 60_000 },
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
  return { room, game: built, ...(await response.json()) };
}

/** Whether `promise` is still unsettled after the microtask queue drains. */
async function stillWaiting(promise) {
  const marker = Symbol("waiting");
  const raced = await Promise.race([
    promise.then(() => "settled"),
    new Promise((resolve) => setTimeout(() => resolve(marker), 25)),
  ]);
  return raced === marker;
}

test("a bot that asks for no wait is answered immediately, as it always was", async () => {
  const { room, botToken } = await startRoom();
  const answer = await (await room.fetch(request("opponent", { token: botToken }))).json();
  assert.deepEqual(answer, { deciding: false, result: null });
});

test("a bot that asks to wait is held until the decision is actually its own", async () => {
  const { room, game, botToken } = await startRoom();
  const pending = room.fetch(request("opponent?wait=5000", { token: botToken }));
  assert.ok(await stillWaiting(pending), "nothing has happened yet; do not send it away");

  // The room only reconsiders when a command lands, which is exactly when the
  // seat can have changed hands.
  game.deciding = true;
  await room.fetch(
    request("command", { token: botToken, body: { t: "botAct", index: 0 } }),
  );

  const answer = await (await pending).json();
  assert.equal(answer.deciding, true);
  assert.ok(answer.observation, "and it arrives holding the decision it waited for");
});

test("a wait ends when the game does, rather than outliving it", async () => {
  const { room, game, botToken } = await startRoom();
  const pending = room.fetch(request("opponent?wait=5000", { token: botToken }));
  assert.ok(await stillWaiting(pending));

  game.finished = true;
  await room.fetch(
    request("lose-on-time", { body: { seat: "bot", reason: "ran out of time" } }),
  );

  const answer = await (await pending).json();
  assert.equal(answer.deciding, false);
  assert.deepEqual(answer.result, { outcome: "win" });
});

test("a wait that nothing satisfies gives up on its own", async () => {
  const { room, botToken } = await startRoom();
  const started = Date.now();
  const answer = await (await room.fetch(
    request("opponent?wait=60", { token: botToken }),
  )).json();
  assert.equal(answer.deciding, false, "the honest answer, just later");
  assert.ok(Date.now() - started >= 50, "and it really did wait for it");
});

test("waiting counts as the bot being present the whole time", async () => {
  const { room, botToken } = await startRoom();
  const before = await (await room.fetch(request("bot-activity"))).json();
  assert.equal(before.lastSeen, null);

  await (await room.fetch(request("opponent?wait=60", { token: botToken }))).json();

  const after = await (await room.fetch(request("bot-activity"))).json();
  assert.ok(
    after.lastSeen !== null && Date.now() - after.lastSeen < 25,
    "a bot parked politely must not read as one that walked away",
  );
});

test("only the bot seat may wait", async () => {
  const { room, humanToken } = await startRoom();
  const response = await room.fetch(request("opponent?wait=5000", { token: humanToken }));
  assert.equal(response.status, 403);
});

test("a wait cannot outlast the presence window that judges it", () => {
  assert.ok(
    MAX_WAIT_MS < 45_000,
    "parking longer than a presence lease would make waiting look like leaving",
  );
  assert.equal(waitBudgetMs(String(MAX_WAIT_MS * 10)), MAX_WAIT_MS);
});

test("an unusable wait is not a wait", () => {
  for (const nonsense of [null, "", "soon", "-1", "0", "NaN", "Infinity"]) {
    assert.equal(waitBudgetMs(nonsense), 0, `${JSON.stringify(nonsense)} asks for nothing`);
  }
  assert.equal(waitBudgetMs("250"), 250);
  assert.equal(waitBudgetMs("250.7"), 250, "milliseconds are whole");
});
