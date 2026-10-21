import { createHash } from "node:crypto";
import { PilotView } from "./pilot-view.mjs";
import { validatePilotV3 } from "./pilot-v3-contract.mjs";

/** Proposal input is complete seat-visible state, never an action menu. */
export class PilotV3View extends PilotView {
  constructor(view, catalog) {
    if (view.apiVersion !== 2) throw new Error("pilot-v3 requires native API 2");
    if (view.observation && view.observation.actionInterface !== "proposal-v1") {
      throw new Error("pilot-v3 requires a proposal-v1 observation");
    }
    // Reuse the state partition and references without asking the engine to
    // enumerate actions. The temporary empty list exists only in this adapter.
    super(view.observation ? { ...view, apiVersion: 1, observation: { ...view.observation, legalActions: [] } } : { ...view, apiVersion: 1 }, catalog);
    const packet = this.packet;
    packet.presentation = "pilot-v3";
    packet.host.apiVersion = 2;
    if (Object.hasOwn(packet.host, "yield")) {
      packet.host.currentYield = packet.host.yield;
      delete packet.host.yield;
    }
    if (packet.host.receipt && Object.hasOwn(packet.host.receipt, "yield")) {
      packet.host.receipt.yieldAtAcceptance = packet.host.receipt.yield;
      delete packet.host.receipt.yield;
    }
    if (packet.position) {
      const position = packet.position;
      delete position.choices;
      delete position.facts.actionInterface;
      position.actionInterface = "proposal-v1";
      for (const field of ["actionContext", "actionAbilities"]) {
        position[field] = position.facts[field];
        delete position.facts[field];
      }
      position.reference = this.reference("proposal observation", view.observation);
      position.rules.items = position.rules.items.map(rule => ({ ...rule,
        printed: rule.reference ? inlinePrintedRules(this.references.get(rule.reference).value) : null }));
    }
    validatePilotV3(packet);
  }

  collection(_kind, items) {
    if (!Array.isArray(items)) throw new Error("collection must be an array");
    return { items, total: items.length, reference: null };
  }

  reference(kind, value) {
    const suffix = kind === "printed card reference" ? `:${value.definition}` : "";
    const reference = `p3r:${createHash("sha256").update(JSON.stringify([this.id, kind, suffix])).digest("base64url")}`;
    this.references.set(reference, { kind, value: structuredClone(value) });
    return reference;
  }
}

function inlinePrintedRules(card) {
  // Omit only catalog presentation metadata. Keep unknown fields and nested
  // gameplay records intact; the reference still resolves to the full card.
  const { printings: _printings, art: _art, ...rules } = card;
  return rules;
}
