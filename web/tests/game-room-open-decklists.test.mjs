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
 * Open decklists is a mutual, opt-in-only disclosure: the bot seat's
 * observation gains `opponentDeck` -- naming the human seat's deck -- only
 * once both the human seat (declared at `start`) and the bot seat (declared
 * by the registry, via `disclose-bot-deck`, once a challenge actually
 * succeeds) have opted in. Neither side opting in, by itself, is enough.
 *
 * This harness mirrors `game-room-cache.test.mjs`: `GameRoom` is transpiled
 * from source with its worker-only imports swapped for injected globals, so
 * these tests run under plain `node --test` without a Workers runtime.
 */

/** Just enough of `WebGame` for the bot seat to hold a pending decision. */
class TestWebGame {
  opponentObserveJson() {
    return JSON.stringify({ decision: { actor: "bot", kind: "Priority" } });
  }

  opponentIsDeciding() {
    return true;
  }

  isFinished() {
    return false;
  }

  resultJson() {
    return undefined;
  }

  state_json() {
    return JSON.stringify({ result: null });
  }
}

installRoomGlobals({ WebGame: TestWebGame });
after(restoreRoomGlobals);

let GameRoom;
test.before(async () => {
  ({ GameRoom } = await loadGameRoom());
});

/** Starts a room and returns its tokens, optionally declaring the human seat's opt-in. */
async function startRoom(room, { humanDiscloseDeck } = {}) {
  const response = await room.fetch(
    request("start", {
      body: {
        humanDeck: "The Deck",
        botDeck: "Sligh",
        botPolicy: "external",
        humanFirst: true,
        seed: 7,
        ...(humanDiscloseDeck === undefined ? {} : { humanDiscloseDeck }),
      },
    }),
  );
  return response.json();
}

async function opponentObservation(room, botToken) {
  const response = await room.fetch(request("opponent", { token: botToken }));
  const view = await response.json();
  assert.equal(view.deciding, true, "the bot seat should hold the decision in this fixture");
  return view.observation;
}

test("neither side opting in: the bot's observation is unchanged", async () => {
  const room = new GameRoom(durableState());
  const { botToken } = await startRoom(room);
  const observation = await opponentObservation(room, botToken);
  assert.equal("opponentDeck" in observation, false);
});

test("both sides opted in: the bot's observation names the human seat's deck", async () => {
  const room = new GameRoom(durableState());
  const { botToken } = await startRoom(room, { humanDiscloseDeck: true });
  await room.fetch(
    request("disclose-bot-deck", { body: { discloseDeck: true } }),
  );
  const observation = await opponentObservation(room, botToken);
  assert.equal(observation.opponentDeck, "The Deck");
});

test("only the human seat opted in: nothing is disclosed", async () => {
  const room = new GameRoom(durableState());
  const { botToken } = await startRoom(room, { humanDiscloseDeck: true });
  // No disclose-bot-deck call: the bot that filled the seat never opted in.
  const observation = await opponentObservation(room, botToken);
  assert.equal("opponentDeck" in observation, false);
});

test("only the bot seat opted in: nothing is disclosed", async () => {
  const room = new GameRoom(durableState());
  const { botToken } = await startRoom(room);
  await room.fetch(
    request("disclose-bot-deck", { body: { discloseDeck: true } }),
  );
  const observation = await opponentObservation(room, botToken);
  assert.equal("opponentDeck" in observation, false);
});

test("an explicit false from either side still withholds disclosure", async () => {
  const room = new GameRoom(durableState());
  const { botToken } = await startRoom(room, { humanDiscloseDeck: true });
  await room.fetch(
    request("disclose-bot-deck", { body: { discloseDeck: false } }),
  );
  const observation = await opponentObservation(room, botToken);
  assert.equal("opponentDeck" in observation, false);
});
