import assert from "node:assert/strict";
import test from "node:test";
import { nativeCommand } from "./native-client.mjs";

const credentials = { server: "http://127.0.0.1:41234", secret: "seat-only" };
const observation = { protocolVersion: 33, seat: "p1", hand: [], battlefield: [], legalActions: [{ index: 7, type: "PassPriority" }], checkpoint: { marker: "private seat checkpoint" } };
const ready = { apiVersion: 1, status: "ready", revision: "revision-one", observation };
function mock(handler) {
  const calls = [];
  return {
    calls,
    fetch: async (url, options) => {
      const body = JSON.parse(options.body);
      calls.push({ url, headers: options.headers, ...body });
      return Response.json(await handler(body.method, body.args));
    },
  };
}

test("fresh invocations reproduce revision tickets and inspect references without a registry", async () => {
  const transport = mock(method => method === "catalog" ? { cards: [] } : ready);
  const first = await nativeCommand("observe", credentials, transport.fetch);
  const second = await nativeCommand("observe", credentials, transport.fetch);
  assert.deepEqual(first, second);
  assert.equal(first.presentation, "pilot-v1");
  assert.equal(first.position.choices.items[0].ticket, "revision-one:a7");
  const details = await nativeCommand("inspect_ref", { ...credentials, reference: first.position.provenance.reference }, transport.fetch);
  assert.deepEqual(details.items[0].checkpoint, observation.checkpoint);
  assert.ok(transport.calls.every(call => call.headers.authorization === "Bearer seat-only"));
  assert.ok(transport.calls.every(call => !Object.hasOwn(call.args, "secret")));
});

test("choose retries send the exact caller request without observing or generating credentials", async () => {
  const transport = mock(() => ({ apiVersion: 1, status: "waiting", receipt: { accepted: 1 } }));
  const input = { ...credentials, ticket: "revision-one:a7", requestId: "move-17" };
  await nativeCommand("choose", input, transport.fetch);
  await nativeCommand("choose", input, transport.fetch);
  assert.equal(transport.calls.length, 2);
  assert.deepEqual(transport.calls[0], transport.calls[1]);
  assert.deepEqual(transport.calls[0].args, { ticket: "revision-one:a7", requestId: "move-17" });
});

test("catalog failure preserves a playable typed packet and committed receipt", async () => {
  const committed = { ...ready, receipt: { requestId: "m", accepted: 1 } };
  const transport = mock(method => {
    if (method === "catalog") throw new Error("catalog unavailable");
    return committed;
  });
  const result = await nativeCommand("choose", { ...credentials, ticket: "old:a7", requestId: "m" }, transport.fetch);
  assert.deepEqual(result.host.receipt, committed.receipt);
  assert.equal(result.presentation, "pilot-v1");
  assert.match(result.host.presentationWarning, /catalog unavailable/);
  assert.deepEqual(result.position.choices.items[0].action, observation.legalActions[0]);
  assert.equal(result.position.choices.items[0].ticket, "revision-one:a7");
});

test("unsupported observations retain exact committed state as a last resort", async () => {
  const committed = { ...ready, observation: { ...observation, protocolVersion: 999 },
    receipt: { requestId: "m", accepted: 1 } };
  const transport = mock(method => method === "catalog" ? { cards: [] } : committed);
  const result = await nativeCommand("choose", { ...credentials, ticket: "old:a7", requestId: "m" }, transport.fetch);
  assert.equal(result.presentation, "exact");
  assert.deepEqual(result.observation, committed.observation);
  assert.deepEqual(result.receipt, committed.receipt);
});

test("exact mode bypasses presentation and catalog lookup", async () => {
  const transport = mock(() => ready);
  assert.deepEqual(await nativeCommand("observe", { ...credentials, presentation: "exact" }, transport.fetch), ready);
  assert.deepEqual(transport.calls.map(call => call.method), ["observe"]);
});

test("organizer status and exports remain raw without presentation warnings", async () => {
  for (const method of ["status", "export"]) {
    const result = { status: "complete", nativeReplayVersion: 1, future: {} };
    const transport = mock(() => result);
    assert.deepEqual(await nativeCommand(method, credentials, transport.fetch), result);
    assert.deepEqual(transport.calls.map(call => call.method), [method]);
  }
});

test("stale references never inspect a new revision", async () => {
  const transport = mock(() => ({ ...ready, revision: "revision-two" }));
  await assert.rejects(nativeCommand("inspect_ref", { ...credentials, reference: "revision-one:r0" }, transport.fetch), /stale reference/);
  assert.deepEqual(transport.calls.map(call => call.method), ["observe"]);
});

test("request IDs, bounded waits and origin-only servers are validated before sending", async () => {
  const transport = mock(() => ready);
  await assert.rejects(nativeCommand("choose", { ...credentials, ticket: "r:a0" }, transport.fetch), /requestId/);
  await assert.rejects(nativeCommand("wait", { ...credentials, waitMs: 25001 }, transport.fetch), /waitMs/);
  await assert.rejects(nativeCommand("observe", { ...credentials, server: "http://user:password@localhost/" }, transport.fetch), /origin/);
  assert.equal(transport.calls.length, 0);
});

test("transport errors propagate without replacing the logical request", async () => {
  const input = { ...credentials, requestId: "retained", ticket: "r:a7" };
  await assert.rejects(nativeCommand("choose", input, async () => { throw new Error("connection lost"); }), /connection lost/);
  const transport = mock(() => ({ status: "waiting", receipt: { requestId: "retained", accepted: 1 } }));
  await nativeCommand("choose", input, transport.fetch);
  assert.equal(transport.calls[0].args.requestId, "retained");
});
