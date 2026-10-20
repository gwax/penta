#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { nativeCommand } from "./native-client.mjs";

const help = `Usage: node tools/penta-agent/native.mjs METHOD [JSON | @FILE | -]
Each input includes {server, secret, ...args}. Prefer stdin for credentials.
Seat methods: observe, wait, choose, play, concede, receipt, transcript, note, catalog.
Organizer methods: seats, status, export, close.
Adapter method: inspect_ref {reference, offset?, limit?}.
choose/play/concede require a caller-retained requestId. Retry the identical
method and arguments after uncertain outcomes. No credentials or requests are saved.
Views default to decision-v1; presentation: "exact" returns the raw host result.
See docs/native-matches.md for startup, reconnect and lifecycle limits.`;

try {
  const [method, input, ...extra] = process.argv.slice(2);
  if (!method || method === "--help" || method === "help") console.log(help);
  else {
    if (extra.length) throw new Error("pass one JSON object, @FILE, or - for stdin");
    const text = input === "-" ? readFileSync(0, "utf8") : input?.startsWith("@") ? readFileSync(input.slice(1), "utf8") : input ?? "{}";
    console.log(JSON.stringify(await nativeCommand(method, JSON.parse(text))));
  }
} catch (error) {
  console.error(JSON.stringify({ error: error.message }));
  process.exitCode = 1;
}
