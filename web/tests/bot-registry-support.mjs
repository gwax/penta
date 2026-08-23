/**
 * Runs `BotRegistry` under `node --test`.
 *
 * The registry is a Durable Object: it expects Workers globals, imports the
 * engine for its compatibility manifest, and reaches other objects through
 * bindings. None of that exists here, so the source is transpiled with its
 * worker-only imports swapped for injected globals -- the same approach
 * `game-room-cache.test.mjs` takes for the room. Loading the real source is
 * the point: a hand-copied registry would pass its tests while the deployed
 * one failed.
 */

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

import ts from "typescript";

import {
  MAX_BOTS,
  PRESENCE_MS,
  isOnline,
  liveInvites,
  publicBot,
  worthKeeping,
} from "../worker/bot-presence.mjs";
import {
  incompatibility,
  incompatibilityBody,
  parseBotCompatibility,
  parseServerCompatibility,
  publicServerCompatibility,
} from "../worker/bot-compatibility.mjs";

export class MemoryStorage {
  values = new Map();
  alarms = [];

  async get(key) {
    const value = this.values.get(key);
    return value === undefined ? undefined : structuredClone(value);
  }

  async put(key, value) {
    this.values.set(key, structuredClone(value));
  }

  async delete(key) {
    return this.values.delete(key);
  }

  async list({ prefix }) {
    const result = new Map();
    for (const [key, value] of this.values) {
      if (key.startsWith(prefix)) result.set(key, structuredClone(value));
    }
    return result;
  }

  async setAlarm(time) {
    this.alarms.push(time);
  }
}

export class TestResponse {
  constructor(body = null, init = {}) {
    this.status = init.status ?? 200;
    this.body = body === null ? "" : String(body);
  }

  static json(value, init = {}) {
    return new TestResponse(JSON.stringify(value), init);
  }

  async json() {
    return JSON.parse(this.body);
  }
}

const SERVER_COMPATIBILITY = {
  protocolVersion: 1,
  capabilities: [],
  requiredCapabilities: [],
  simulationFingerprint: "test-fingerprint",
  legacyUndeclaredProtocolVersion: 1,
};

class TestHostedGame {
  static botCompatibilityJson() {
    return JSON.stringify(SERVER_COMPATIBILITY);
  }
}

/**
 * A fake `GAME_ROOMS` room that records every call it receives.
 *
 * `answers` overrides the reply for a route by name, so a test can say what
 * a room reports without teaching this stub the room's real behaviour.
 */
export function fakeRoom(answers = {}) {
  const calls = [];
  return {
    calls,
    async fetch(request) {
      const url = new URL(request.url);
      const route = url.pathname.split("/").pop();
      const body = request.method === "POST" ? await request.json() : undefined;
      calls.push({ route, body });
      const answer = answers[route];
      const value = typeof answer === "function" ? answer(body) : answer;
      return { ok: true, json: async () => value ?? { ok: true } };
    },
  };
}

/** Routes every room id to `room`, or to whatever `rooms` names for that id. */
export function fakeEnv(room, rooms = {}) {
  return {
    GAME_ROOMS: {
      idFromName: (name) => name,
      get: (name) => rooms[name] ?? room,
    },
  };
}

export function durableState() {
  return { storage: new MemoryStorage() };
}

export function request(route, { body } = {}) {
  return new Request(`https://registry.test/_bots/${route}`, {
    method: body === undefined ? "GET" : "POST",
    headers: body === undefined ? undefined : { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
}

export async function register(registry, overrides = {}) {
  const response = await registry.fetch(
    request("register", { body: { name: "Fizzbot", deck: "Sligh", ...overrides } }),
  );
  return response.json();
}

export async function heartbeat(registry, id, overrides = {}) {
  const response = await registry.fetch(
    request(`${id}/heartbeat`, { body: { done: [], ...overrides } }),
  );
  return response.json();
}

const originalResponse = globalThis.Response;

/** Installs the injected globals the transpiled registry closes over. */
export function installRegistryGlobals() {
  globalThis.Response = TestResponse;
  globalThis.__botPresence = {
    MAX_BOTS,
    PRESENCE_MS,
    isOnline,
    liveInvites,
    publicBot,
    worthKeeping,
  };
  globalThis.__botCompatibility = {
    incompatibility,
    incompatibilityBody,
    parseBotCompatibility,
    parseServerCompatibility,
    publicServerCompatibility,
  };
  globalThis.__botRegistryEngine = async () => ({ HostedGame: TestHostedGame });
}

export function restoreRegistryGlobals() {
  globalThis.Response = originalResponse;
  delete globalThis.__botPresence;
  delete globalThis.__botCompatibility;
  delete globalThis.__botRegistryEngine;
}

export async function loadBotRegistry() {
  let source = await readFile(
    new URL("../worker/bot-registry.ts", import.meta.url),
    "utf8",
  );
  const replacements = [
    [
      'import {\n  MAX_BOTS,\n  PRESENCE_MS,\n  isOnline,\n  liveInvites,\n  publicBot,\n  worthKeeping,\n} from "./bot-presence.mjs";',
      "const {\n  MAX_BOTS,\n  PRESENCE_MS,\n  isOnline,\n  liveInvites,\n  publicBot,\n  worthKeeping,\n} = globalThis.__botPresence;",
    ],
    [
      'import {\n  incompatibility,\n  incompatibilityBody,\n  parseBotCompatibility,\n  parseServerCompatibility,\n  publicServerCompatibility,\n} from "./bot-compatibility.mjs";',
      "const {\n  incompatibility,\n  incompatibilityBody,\n  parseBotCompatibility,\n  parseServerCompatibility,\n  publicServerCompatibility,\n} = globalThis.__botCompatibility;",
    ],
    [
      'import { engine } from "./engine";',
      "const engine = globalThis.__botRegistryEngine;",
    ],
  ];
  for (const [from, to] of replacements) {
    assert.ok(source.includes(from), `test loader expected ${from}`);
    source = source.replace(from, to);
  }
  const javascript = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
    },
  }).outputText;
  const encoded = Buffer.from(javascript).toString("base64");
  return import(`data:text/javascript;base64,${encoded}`);
}
