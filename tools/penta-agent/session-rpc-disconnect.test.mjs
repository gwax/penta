import assert from "node:assert/strict";
import { request as httpRequest } from "node:http";
import { setTimeout as delay } from "node:timers/promises";
import test from "node:test";
import { serveNativeSession, callNativeSession } from "./session-rpc.mjs";

const connection = {
  server: "http://127.0.0.1:41234",
  secret: "disconnect-test-seat",
  presentation: "pilot-v2",
};

function deferred() {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
}

test("RPC disconnect recovery survives an intervening invalid request", { timeout: 10_000 }, async () => {
  const entered = deferred();
  const release = deferred();
  const cursors = [];
  const printed = { rulesText: "This rule must be printed again after context delivery fails." };
  const session = await serveNativeSession(connection, async (_method, args) => {
    cursors.push(args.afterEvent);
    if (cursors.length === 2) {
      entered.resolve();
      await release.promise;
    }
    return {
      presentation: "pilot-v2", status: "ready",
      host: { eventDelivery: { after: args.afterEvent, next: cursors.length } },
      position: { rules: { items: [{ definition: "card", available: true, printed }], total: 1, reference: null } },
    };
  });
  const call = request => callNativeSession({ clientServer: session.clientServer, secret: connection.secret, request });
  let abandoned;
  try {
    const initial = await call({ method: "observe" });
    assert.deepEqual(initial.position.rules.items[0].printed, printed);
    assert.equal(initial.host.eventDelivery.next, 1);

    const body = JSON.stringify({ method: "observe" });
    abandoned = httpRequest(session.clientServer, {
      method: "POST",
      headers: { authorization: `Bearer ${connection.secret}`, "content-type": "application/json", "content-length": Buffer.byteLength(body) },
    });
    abandoned.on("error", () => {}); // The deliberate disconnect reports ECONNRESET.
    const disconnected = new Promise(resolve => abandoned.once("close", resolve));
    abandoned.end(body);
    await entered.promise;
    abandoned.destroy();
    await disconnected;
    // Let the server process the socket close before the held reply becomes available.
    await delay(20);
    release.resolve();

    let invalid;
    for (let attempt = 0; attempt < 100; attempt++) {
      try { invalid = await call({}); break; }
      catch (error) {
        if (!/in progress/.test(error.message)) throw error;
        await delay(5);
      }
    }
    assert.equal(invalid?.error, "method must be a nonempty string");
    const recovered = await call({ method: "observe" });
    assert.deepEqual(cursors, [0, 1, 0]);
    assert.deepEqual(recovered.position.rules.items[0].printed, printed);
    assert.equal(recovered.position.rules.items[0].printedPreviouslySent, undefined);
    assert.equal(recovered.host.eventDelivery.after, 0);
  } finally {
    abandoned?.destroy();
    release.resolve();
    await session.close();
  }
});
