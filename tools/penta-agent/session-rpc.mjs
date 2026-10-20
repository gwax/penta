import { createServer } from "node:http";
import { timingSafeEqual } from "node:crypto";
import { PassThrough, Writable } from "node:stream";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { runNativeSession, MAX_SESSION_LINE_BYTES } from "./native-session.mjs";
import { validateNativeConnection } from "./native-client.mjs";

/** Memory-only, seat-bound transport for finite per-request CLI processes. */
export async function serveNativeSession(connection, command) {
  validateNativeConnection(connection);
  const input = new PassThrough();
  let pending, busy = false, needsFull = false;
  const output = new Writable({ write(chunk, _encoding, done) {
    try { pending?.resolve({ packet: JSON.parse(chunk.toString()), acknowledge: done }); pending = undefined; }
    catch (error) { pending?.reject(error); pending = undefined; done(error); }
  } });
  const dispatch = request => new Promise((resolve, reject) => {
    pending = { resolve, reject };
    input.write(`${JSON.stringify(request)}\n`);
  });
  const running = runNativeSession(input, output, command);
  running.catch(error => { pending?.reject(error); pending = undefined; });
  const initialized = await dispatch(connection);
  initialized.acknowledge();
  if (initialized.packet.error) { input.end(); throw new Error(initialized.packet.error); }
  const authorization = Buffer.from(`Bearer ${connection.secret}`);
  const server = createServer(async (request, response) => {
    const reply = (status, body) => new Promise(resolve => {
      if (response.destroyed) { resolve(false); return; }
      response.once("finish", () => resolve(true));
      response.once("close", () => resolve(response.writableFinished));
      response.writeHead(status, { "content-type": "application/json", "cache-control": "no-store" });
      response.end(JSON.stringify(body));
    });
    const supplied = Buffer.from(request.headers.authorization ?? "");
    if (supplied.length !== authorization.length || !timingSafeEqual(supplied, authorization)) {
      reply(401, { error: "invalid session capability" }); return;
    }
    if (request.method !== "POST" || request.url !== "/") {
      reply(404, { error: "POST the session origin" }); return;
    }
    if (busy) { reply(409, { error: "session has a request in progress; wait for its result" }); return; }
    busy = true;
    try {
      let length = 0;
      const chunks = [];
      for await (const chunk of request) {
        length += chunk.length;
        if (length > MAX_SESSION_LINE_BYTES) {
          reply(413, { error: "session request exceeds 1 MiB" }); return;
        }
        chunks.push(chunk);
      }
      let body;
      try { body = JSON.parse(Buffer.concat(chunks).toString()); }
      catch { reply(400, { error: "invalid JSON request" }); return; }
      if (body?.method === "shutdown") {
        if (Object.keys(body).length !== 1) { reply(400, { error: "shutdown accepts no arguments" }); return; }
        input.end();
        await running;
        await reply(200, { status: "closed" });
        server.close();
        return;
      }
      const result = await dispatch(needsFull && body && typeof body === "object"
        ? { ...body, full: true } : body);
      let delivered = false;
      try { delivered = await reply(200, result.packet); }
      finally {
        // A disconnected caller needs a complete refresh before its next command.
        needsFull = !delivered || (needsFull && Boolean(result.packet?.error));
        result.acknowledge();
      }
    } catch {
      if (!response.headersSent) reply(500, { error: "session transport failed; recover any uncertain request before playing again" });
    } finally { busy = false; }
  });
  server.requestTimeout = 10_000;
  server.headersTimeout = 5_000;
  server.keepAliveTimeout = 1_000;
  const closed = new Promise(resolve => server.once("close", resolve));
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  return { clientServer: `http://127.0.0.1:${server.address().port}`, closed,
    async close() { input.end(); await running; server.close(); await closed; } };
}

/** A finite call exits on its reply, avoiding fixed PTY collection delays. */
export async function callNativeSession({ clientServer, secret, request }) {
  const url = validateNativeConnection({ server: clientServer, secret });
  if (url.hostname !== "127.0.0.1") throw new Error("clientServer must be the local session origin");
  const response = await fetch(url, {
    method: "POST", headers: { authorization: `Bearer ${secret}`, "content-type": "application/json" },
    body: JSON.stringify(request), signal: AbortSignal.timeout(70_000),
  });
  const result = await response.json();
  if (!response.ok) throw new Error(result.error ?? `session HTTP ${response.status}`);
  return result;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const [method, source] = process.argv.slice(2);
    if (source !== "-" || !["serve", "call"].includes(method)) {
      throw new Error("Usage: node tools/penta-agent/session-rpc.mjs serve|call - (JSON on stdin)");
    }
    const args = JSON.parse(readFileSync(0, "utf8"));
    if (method === "serve") {
      const session = await serveNativeSession(args);
      console.log(JSON.stringify({ clientServer: session.clientServer }));
      await session.closed;
    } else console.log(JSON.stringify(await callNativeSession(args)));
  } catch (error) {
    console.error(JSON.stringify({ error: error.message }));
    process.exitCode = 1;
  }
}
