// JSON object member order is not semantic; array order and all values are.
function semanticKey(value) {
  if (Array.isArray(value)) return `[${value.map(semanticKey).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map(key => `${JSON.stringify(key)}:${semanticKey(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

/** Printed knowledge belongs only to this live session and successful writes. */
export class SessionRules {
  #printed = new Map();

  reset() { this.#printed.clear(); }

  prepare(packet) {
    const collection = packet?.presentation === "pilot-v2" ? packet.position?.rules : null;
    if (!Array.isArray(collection?.items)) return { packet, commit() {} };
    const updates = new Map();
    const items = collection.items.map(rule => {
      if (!rule.available || rule.printed == null) {
        // Unavailable or omitted rules do not teach the session their contents.
        // Forget unavailable definitions so recovery explicitly prints them.
        if (!rule.available) updates.set(rule.definition, null);
        return rule;
      }
      const key = semanticKey(rule.printed);
      updates.set(rule.definition, key);
      if (this.#printed.get(rule.definition) !== key) return rule;
      return { ...rule, printed: null, printedPreviouslySent: true };
    });
    return {
      packet: { ...packet, position: { ...packet.position, rules: { ...collection, items } } },
      // Call only after stdout accepted the complete response. Merely preparing
      // output (including a response whose write failed) establishes no knowledge.
      commit: () => {
        for (const [definition, key] of updates) {
          if (key === null) this.#printed.delete(definition);
          else this.#printed.set(definition, key);
        }
      },
    };
  }
}
