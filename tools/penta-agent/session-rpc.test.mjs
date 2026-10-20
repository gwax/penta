import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import test from "node:test";
import { serveNativeSession, callNativeSession } from "./session-rpc.mjs";

const connection = { server: "http://127.0.0.1:41234", secret: "seat-capability" };

test("RPC session rejects other capabilities and preserves one session across finite calls", async () => {
  let calls = 0;
  const session = await serveNativeSession(connection, async () => { calls++; return { status: "ready" }; });
  const call = request => callNativeSession({ clientServer: session.clientServer, secret: connection.secret, request });
  try {
    await assert.rejects(callNativeSession({ clientServer: session.clientServer, secret: "other", request: { method: "observe" } }), /capability/);
    assert.equal(calls, 0);
    assert.equal((await call({ method: "next" })).status, "ready");
    assert.equal((await call({ method: "next" })).status, "ready");
    assert.deepEqual((await call({ method: "sessionStats" })).callsByMethod, { next: 2 });
    assert.equal((await call({ method: "shutdown" })).status, "closed");
    await session.closed;
    assert.equal(calls, 2);
  } finally { await session.close(); }
});

test("RPC session rejects concurrent commands without dispatching another mutation", async () => {
  let release;
  let started;
  const active = new Promise(resolve => { started = resolve; });
  const session = await serveNativeSession(connection, async () => {
    started();
    await new Promise(resolve => { release = resolve; });
    return { status: "ready" };
  });
  const call = request => callNativeSession({ clientServer: session.clientServer, secret: connection.secret, request });
  try {
    const first = call({ method: "observe" });
    await active;
    await assert.rejects(call({ method: "observe" }), /in progress/);
    release();
    await first;
  } finally { await session.close(); }
});

test("RPC finite CLI exits after a ready reply while the memory session stays alive", async () => {
  const session = await serveNativeSession(connection, async () => ({ status: "ready" }));
  try {
    const script = `import {callNativeSession} from ${JSON.stringify(new URL("./session-rpc.mjs", import.meta.url).href)}; console.log(JSON.stringify(await callNativeSession(${JSON.stringify({ clientServer: session.clientServer, secret: connection.secret, request: { method: "next" } })})));`;
    const result = await promisify(execFile)(process.execPath, ["--input-type=module", "-e", script], { timeout: 5000 });
    assert.deepEqual(JSON.parse(result.stdout), { status: "ready" });
    const stats = await callNativeSession({ clientServer: session.clientServer, secret: connection.secret, request: { method: "sessionStats" } });
    assert.equal(stats.callsByMethod.next, 1);
  } finally { await session.close(); }
});
