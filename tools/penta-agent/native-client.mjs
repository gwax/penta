import { DecisionView } from "./decision-view.mjs";

const methods = new Set(["seats", "status", "export", "close", "observe", "wait", "choose", "play", "concede", "receipt", "transcript", "note", "catalog"]);
const mutations = new Set(["choose", "play", "concede"]);

/** One invocation, one credential. No connection registry or recovery files. */
export async function nativeCommand(method, input, fetcher = fetch) {
  if (!input || typeof input !== "object" || Array.isArray(input)) throw new Error("arguments must be a JSON object");
  const { server, secret, presentation = "decision-v1", ...args } = input;
  const url = new URL(server);
  if (!["http:", "https:"].includes(url.protocol) || url.username || url.password || url.search || url.hash || url.pathname !== "/") {
    throw new Error("server must be an HTTP origin without credentials");
  }
  if (typeof secret !== "string" || !secret) throw new Error("secret must be a nonempty string");
  if (!["decision-v1", "exact"].includes(presentation)) throw new Error("unsupported presentation");
  if (!methods.has(method) && method !== "inspect_ref") throw new Error(`unknown native command: ${method}`);
  if (args.waitMs !== undefined && (!Number.isInteger(args.waitMs) || args.waitMs < 0 || args.waitMs > 25_000)) throw new Error("waitMs must be an integer from 0 to 25000");
  if ((mutations.has(method) || method === "receipt") && (typeof args.requestId !== "string" || !args.requestId)) throw new Error("requestId must be a nonempty string; retain it for retries");
  for (const key of ["offset", "limit"]) {
    if (args[key] !== undefined && (!Number.isInteger(args[key]) || args[key] < (key === "limit" ? 1 : 0) || (key === "limit" && args[key] > 100))) throw new Error(`invalid ${key}`);
  }
  const request = async (operation, body) => {
    const response = await fetcher(url.href, {
      method: "POST", headers: { authorization: `Bearer ${secret}`, "content-type": "application/json" },
      body: JSON.stringify({ method: operation, args: body }), signal: AbortSignal.timeout(35_000),
    });
    let result;
    try { result = await response.json(); } catch { throw new Error(`Penta HTTP ${response.status} without JSON; retry uncertain mutations with the identical request`); }
    if (!response.ok) throw new Error(`Penta HTTP ${response.status}: ${result.error ?? response.statusText}`);
    return result;
  };
  const build = async view => new DecisionView(view, await request("catalog", {}), new Set(), true, { id: view.revision });
  if (method === "inspect_ref") {
    const { reference, ...paging } = args;
    if (typeof reference !== "string" || !reference) throw new Error("reference must be a nonempty string");
    const view = await request("observe", {});
    if (!view.observation || !reference.startsWith(`${view.revision}:r`)) throw new Error("stale reference; observe again");
    return (await build(view)).inspect(reference, paging);
  }
  const result = await request(method, args);
  if (presentation === "exact" || !result.observation) return result;
  try {
    const view = await build(result);
    const { observation: _observation, ...envelope } = result;
    return { ...envelope, ...view.packet };
  } catch {
    // The mutation may already be committed. Presentation is never its outcome.
    return { ...result, presentation: "exact", presentationWarning: "Compact presentation unavailable; exact observation retained." };
  }
}
