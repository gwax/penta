import { objectNames, actionLabel } from "./decision-labels.mjs";
import { visibleDefinitions, printedReference } from "./decision-format.mjs";
import { validatePilot } from "./pilot-contract.mjs";
import { page } from "./views.mjs";

const SITUATION = ["seat", "activeSeat", "prioritySeat", "turn", "activeTurn", "phase", "step", "pregame", "match", "regularCombatDamagePending", "result"];
const PLAYERS = ["life", "manaPools", "poison", "energy", "playerCounters", "monarch", "opponentHandSize", "librarySizes", "faceDownExileSizes"];
const ZONES = ["hand", "battlefield", "stack", "graveyards", "exiles", "emblems", "companions", "cardCounters"];
const PROVENANCE = ["protocolVersion", "protocolCapabilities", "engineVersion", "simulationFingerprint", "format", "checkpoint"];

/** Native-only, self-contained presentation. Exact payloads never become labels. */
export class PilotView {
  references = new Map();
  tickets = new Map();
  active = true;

  constructor(view, catalog) {
    if (view.apiVersion !== 1) throw new Error("pilot-v1 requires native API 1");
    this.id = view.revision ?? null;
    this.revision = this.id;
    const { observation, ...host } = structuredClone(view);
    const packet = {
      presentation: "pilot-v1", status: view.status, revision: this.id,
      host, position: null,
    };
    if (observation) {
      if (observation.protocolVersion !== 33) throw new Error("pilot-v1 requires bot protocol 33");
      if (typeof this.id !== "string") throw new Error("observations require a revision");
      const remaining = { ...observation };
      const take = keys => Object.fromEntries(keys.filter(key => Object.hasOwn(remaining, key)).map(key => {
        const value = remaining[key]; delete remaining[key]; return [key, value];
      }));
      const situation = take(SITUATION), players = take(PLAYERS);
      const provenance = { reference: this.reference("provenance and checkpoint", take(PROVENANCE)) };
      const zones = {};
      for (const field of ZONES) {
        // Absent and empty are distinct. Nested player arrays retain [p1,p2].
        zones[field] = Object.hasOwn(remaining, field) ? this.collection(field, remaining[field]) : null;
        delete remaining[field];
      }
      const names = objectNames(observation);
      const actions = remaining.legalActions;
      if (!Array.isArray(actions)) throw new Error("legalActions must be an array");
      const choices = actions.map(action => {
        if (!Number.isInteger(action.index) || typeof action.type !== "string") throw new Error("invalid exact action");
        const ticket = `${this.id}:a${action.index}`;
        this.tickets.set(ticket, { index: action.index, action });
        return { ticket, label: pilotActionLabel(action, names), action };
      });
      delete remaining.legalActions;
      let decision = null;
      if (remaining.decision != null) {
        const { options, ...schema } = remaining.decision;
        decision = { schema, options: this.collection("decision options", options) };
      }
      delete remaining.decision;
      const updates = Object.hasOwn(remaining, "updates") ? this.collection("updates", remaining.updates) : null;
      delete remaining.updates;
      // State references must not shift when optional catalog lookups fail.
      const choiceCollection = this.collection("choices", choices);
      const compatible = !catalog || ["protocolVersion", "simulationFingerprint"].every(key =>
        catalog[key] === undefined || observation[key] === undefined || catalog[key] === observation[key]);
      const cards = new Map((compatible ? catalog?.cards ?? [] : []).map(card => [card.definition, card]));
      let budget = 12_000;
      const rules = visibleDefinitions(observation).map(definition => {
        const card = cards.get(definition);
        if (!card) return { definition, available: false, name: null, printed: null, reference: null };
        // Normalize absent optional reference fields as JSON transport does.
        const encoded = JSON.stringify(printedReference(card));
        const body = JSON.parse(encoded), size = encoded.length;
        const printed = size <= budget ? body : null;
        if (printed) budget -= size;
        return { definition, available: true, name: card.name, printed,
          reference: this.reference("printed card reference", card) };
      });
      packet.position = { situation, players, zones, updates,
        choices: choiceCollection, decision,
        facts: remaining, provenance, rules: this.collection("rules", rules) };
    }
    validatePilot(packet);
    this.packet = packet;
  }

  collection(kind, items) {
    if (!Array.isArray(items)) throw new Error(`${kind} must be an array`);
    const paged = items.length > 100 || JSON.stringify(items).length > 12_000;
    return { items: paged ? [] : items, total: items.length,
      reference: paged ? this.reference(kind, items) : null };
  }

  reference(kind, value) {
    // Catalog availability must never retarget a previously issued reference.
    // Namespacing also prevents interpreting legacy decision-v1 references here.
    const suffix = kind === "printed card reference" ? `:${value.definition}` : "";
    const reference = `${this.id}:r:pilot-v1:${encodeURIComponent(kind + suffix)}`;
    this.references.set(reference, { kind, value: structuredClone(value) });
    return reference;
  }
  inspect(reference, { offset = 0, limit = 100, query, actionType } = {}) {
    const found = this.references.get(reference);
    if (!found) throw new Error("unknown or expired reference; observe again");
    let items = Array.isArray(found.value) ? found.value : [found.value];
    if (actionType) items = items.filter(item => (item?.action?.type ?? item?.type) === actionType);
    if (query) items = items.filter(item => JSON.stringify(item).toLowerCase().includes(query.toLowerCase()));
    return { view: this.id, reference, kind: found.kind, ...page(items, offset, limit) };
  }
}

function pilotActionLabel(action, names) {
  let label = actionLabel(action, names);
  const details = [];
  function targets(value) {
    if (Array.isArray(value)) value.forEach(targets);
    else if (value && typeof value === "object") {
      if (value.type === "player" && (value.seat === "p1" || value.seat === "p2")) details.push(`target ${value.seat}`);
      if (value.player === "p1" || value.player === "p2") details.push(`target ${value.player}`);
      for (const [key, child] of Object.entries(value)) if (key !== "player") targets(child);
    }
  }
  targets(action.targets);
  // These are annotations only. Preserve canonical ordering and exact values.
  const append = (key, value) => {
    if (value !== undefined && value !== null && (!Array.isArray(value) || value.length)) {
      details.push(`${key}: ${JSON.stringify(value)}`);
    }
  };
  for (const key of ["modeIds", "x", "ability", "manaPayment"]) {
    append(key, action[key]);
  }
  for (const key of ["playOptionId", "alternativeCostId", "additionalCostIds", "manaPayment", "permissionSource"]) {
    append(`choices.${key}`, action.choices?.[key]);
  }
  if (details.length) label += `; ${details.join("; ")}`;
  return label;
}
