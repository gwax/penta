import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { Readable, Writable } from "node:stream";
import test from "node:test";
import { MAX_SESSION_LINE_BYTES, runNativeSession } from "./native-session.mjs";

const connection = { server: "http://127.0.0.1:41234", secret: "private-seat-secret", presentation: "exact" };
async function session(chunks, command) {
  let text = "";
  const output = new Writable({ write(chunk, _encoding, done) { text += chunk; done(); } });
  await runNativeSession(Readable.from(chunks), output, command);
  return { text, replies: text.trim().split("\n").filter(Boolean).map(line => JSON.parse(line)) };
}
const initialization = `${JSON.stringify(connection)}\n`;

test("session initializes without network access, dispatches serially, and drains final line on EOF", async () => {
  const calls = [];
  let active = false;
  const result = await session([initialization, '{"method":"observe"}\n{"method":"wait","waitMs":1}'], async (method, args) => {
    assert.equal(active, false);
    active = true;
    await new Promise(resolve => setImmediate(resolve));
    calls.push({ method, args });
    active = false;
    return { method };
  });
  assert.deepEqual(result.replies, [{ status: "initialized" }, { method: "observe" }, { method: "wait" }]);
  assert.deepEqual(calls, [{ method: "observe", args: connection }, { method: "wait", args: { ...connection, waitMs: 1 } }]);
  assert.ok(!result.text.includes(connection.secret));
});

test("session keeps running after parse, validation and command errors without printing secrets", async () => {
  const result = await session([initialization,
    `bad JSON ${connection.secret}\n{}\n{"method":"wait","secret":"replacement"}\n{"method":"choose"}\n{"method":"observe"}\n`,
  ], async method => {
    if (method === "choose") throw new Error(`failed for ${connection.secret}`);
    return { status: "ready", nested: [connection.secret] };
  });
  assert.deepEqual(result.replies, [
    { status: "initialized" }, { error: "invalid JSON line" },
    { error: "method must be a nonempty string" }, { error: "session requests must omit server and secret" },
    { error: "failed for [redacted]" }, { status: "ready", nested: ["[redacted]"] },
  ]);
  assert.ok(!result.text.includes(connection.secret));
});

test("session rejects invalid startup once and empty EOF exits without requests", async () => {
  const command = () => assert.fail("must not dispatch");
  assert.deepEqual((await session([], command)).replies, []);
  assert.deepEqual((await session(['{"server":"bad","secret":"sensitive"}\n{}\n'], command)).replies,
    [{ error: "server must be an HTTP origin without credentials" }]);
});

test("oversized lines are bounded, discarded through newline, and do not kill initialized sessions", async () => {
  const result = await session([initialization, "x".repeat(MAX_SESSION_LINE_BYTES), "x\n", '{"method":"observe"}\n'],
    async () => ({ status: "ready" }));
  assert.deepEqual(result.replies, [{ status: "initialized" },
    { error: `JSON line exceeds ${MAX_SESSION_LINE_BYTES} bytes` }, { status: "ready" }]);
});

test("sessionStats measures dispatches, errors and actual output bytes without host requests", async () => {
  const calls = [];
  const result = await session([initialization, '{"method":"observe"}\n{"method":"choose"}\nnot json\n{"method":"sessionStats"}\n'],
    async method => {
      calls.push(method);
      if (method === "choose") throw new Error("missing requestId");
      return { status: "ready", text: "é" };
    });
  assert.deepEqual(calls, ["observe", "choose"]);
  const stats = result.replies.at(-1);
  assert.deepEqual(stats.callsByMethod, { observe: 1, choose: 1 });
  assert.equal(stats.errors, 2);
  assert.equal(stats.outputBytes, Buffer.byteLength(result.text.split("\n").slice(0, -2).join("\n") + "\n"));
  assert.ok(Number.isInteger(stats.elapsedMs) && stats.elapsedMs >= 0);
});

test("full is local, resets printed knowledge across waiting, and actual compressed bytes count in stats", async () => {
  const printed = { rulesText: "A sufficiently long printed rule repeated across observations." };
  const value = { presentation: "pilot-v2", status: "ready", position: {
    rules: { items: [{ definition: "d", available: true, printed }], total: 1, reference: null },
  } };
  const calls = [];
  const result = await session([initialization,
    '{"method":"observe"}\n{"method":"observe","full":false}\n{"method":"wait","full":true}\n{"method":"observe"}\n{"method":"sessionStats"}\n',
  ], async (method, args) => {
    calls.push({ method, args });
    return method === "wait" ? { status: "waiting", position: null } : value;
  });
  assert.deepEqual(result.replies[1], value);
  assert.deepEqual(result.replies[2].position.rules.items[0], { definition: "d", available: true, printed: null, printedPreviouslySent: true });
  assert.deepEqual(result.replies[4], value);
  assert.ok(calls.every(call => !Object.hasOwn(call.args, "full")));
  const lines = result.text.trimEnd().split("\n");
  assert.equal(result.replies[5].outputBytes, Buffer.byteLength(lines.slice(0, -1).join("\n") + "\n"));
});

test("invalid full is rejected locally and the next request still runs", async () => {
  const calls = [];
  const result = await session([initialization, '{"method":"observe","full":"yes"}\n{"method":"observe"}\n'],
    async method => { calls.push(method); return { status: "ready" }; });
  assert.deepEqual(calls, ["observe"]);
  assert.equal(result.replies[1].error, "full must be a boolean");
  assert.equal(result.replies[2].status, "ready");
});

test("CLI session reads stdin and exits cleanly at EOF with JSON-only stdout", async () => {
  const child = spawn(process.execPath, [new URL("./native.mjs", import.meta.url).pathname, "session"], { stdio: "pipe" });
  let stdout = "";
  let stderr = "";
  child.stdout.on("data", chunk => { stdout += chunk; });
  child.stderr.on("data", chunk => { stderr += chunk; });
  const exited = new Promise((resolve, reject) => {
    child.on("error", reject);
    child.on("exit", code => resolve(code));
  });
  child.stdin.end(`${initialization}{"method":"choose"}\n{"method":"unknown"}\n`);
  assert.equal(await exited, 0);
  assert.equal(stderr, "");
  const replies = stdout.trim().split("\n").map(line => JSON.parse(line));
  assert.equal(replies[0].status, "initialized");
  assert.match(replies[1].error, /requestId/);
  assert.match(replies[2].error, /unknown native command/);
  assert.ok(!stdout.includes(connection.secret));
});
