import { nativeCommand, validateNativeConnection } from "./native-client.mjs";
import { SessionRules } from "./session-rules.mjs";

export const MAX_SESSION_LINE_BYTES = 1024 * 1024;

/** Bounded input buffering; dispatch and output finish before reading more input. */
async function* lines(input) {
  let pending = Buffer.alloc(0);
  let oversized = false;
  for await (const chunk of input) {
    const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
    let start = 0;
    while (start < bytes.length) {
      const newline = bytes.indexOf(10, start);
      const end = newline < 0 ? bytes.length : newline;
      const part = bytes.subarray(start, end);
      if (!oversized && pending.length + part.length <= MAX_SESSION_LINE_BYTES) {
        pending = Buffer.concat([pending, part]);
      } else {
        oversized = true;
        pending = Buffer.alloc(0);
      }
      if (newline < 0) break;
      yield oversized ? null : pending.toString("utf8");
      pending = Buffer.alloc(0);
      oversized = false;
      start = newline + 1;
    }
  }
  if (oversized || pending.length) yield oversized ? null : pending.toString("utf8");
}

function redact(value, secret) {
  if (typeof value === "string") return secret ? value.split(secret).join("[redacted]") : value;
  if (Array.isArray(value)) return value.map(item => redact(item, secret));
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [redact(key, secret), redact(item, secret)]));
  }
  return value;
}

/** First line initializes credentials; subsequent lines are explicit commands. */
export async function runNativeSession(input, output, command = nativeCommand) {
  let connection;
  let first = true;
  let secret;
  let startedAt;
  let outputBytes = 0;
  let errors = 0;
  const callsByMethod = {};
  const printedRules = new SessionRules();
  const countedMethods = new Set(["seats", "status", "export", "close", "observe", "wait", "choose", "play", "concede", "yield", "cancelYield", "receipt", "transcript", "note", "catalog", "inspect_ref", "schema"]);
  const reply = value => new Promise((resolve, reject) => {
    const line = `${JSON.stringify(redact(value, secret))}\n`;
    output.write(line, error => {
      if (error) reject(error);
      else { outputBytes += Buffer.byteLength(line); resolve(); }
    });
  });
  for await (const line of lines(input)) {
    try {
      if (line === null) throw new Error(`JSON line exceeds ${MAX_SESSION_LINE_BYTES} bytes`);
      let request;
      try { request = JSON.parse(line); } catch { throw new Error("invalid JSON line"); }
      if (!request || typeof request !== "object" || Array.isArray(request)) throw new Error("request must be a JSON object");
      if (first) {
        secret = typeof request.secret === "string" ? request.secret : undefined;
        if (Object.keys(request).some(key => !["server", "secret", "presentation"].includes(key))) throw new Error("initialization accepts only server, secret, and presentation");
        validateNativeConnection(request);
        connection = { ...request };
        first = false;
        startedAt = performance.now();
        await reply({ status: "initialized" });
        continue;
      }
      if (Object.hasOwn(request, "server") || Object.hasOwn(request, "secret")) throw new Error("session requests must omit server and secret");
      const { method, full, ...args } = request;
      if (typeof method !== "string" || !method) throw new Error("method must be a nonempty string");
      if (full !== undefined && typeof full !== "boolean") throw new Error("full must be a boolean");
      // Reset before dispatch even if it returns waiting/no position or fails.
      // The next inline rules are then self-contained after context loss.
      if (full === true) printedRules.reset();
      if (method === "sessionStats") {
        if (Object.keys(args).length) throw new Error("sessionStats accepts no arguments");
        await reply({ callsByMethod: { ...callsByMethod }, errors, outputBytes, elapsedMs: Math.floor(performance.now() - startedAt) });
        continue;
      }
      const counter = countedMethods.has(method) ? method : "unknown";
      callsByMethod[counter] = (callsByMethod[counter] ?? 0) + 1;
      const prepared = printedRules.prepare(await command(method, { ...connection, ...args }));
      await reply(prepared.packet);
      prepared.commit();
    } catch (error) {
      errors++;
      await reply({ error: error.message });
      // Invalid startup cannot establish a usable session.
      if (first) return;
    }
  }
}
