import { createHash } from "node:crypto";
import { PilotView } from "./pilot-view.mjs";
import { actionLabel, objectNames, objectLabel } from "./decision-labels.mjs";
import { validatePilotV2 } from "./pilot-v2-contract.mjs";

const CORE = ["objectId", "name", "controller", "owner", "tapped", "canAttack", "enteredThisTurn", "power", "toughness", "damage", "loyalty", "counters"];
const METADATA = new Set(["instance", "definition", "art", "presentedPartId"]);
// Only known protocol defaults are elided. Unknown values, including false,
// zero and empty containers, remain visible in current rather than disappearing.
const DEFAULTS = new Set(["attackDefender", "attacking", "attackingBand", "blockedThisCombat", "blocking", "blockingThisCombat", "chosenBasicLandType", "chosenBasicLandTypeSubstitution", "chosenCardName", "chosenCardType", "chosenColor", "chosenColors", "chosenCreatureType", "chosenLabels", "classLevel", "designations", "faceDown", "flying", "hasIndividualState", "loyaltyAbilityUsedThisTurn", "phasedOut", "token"]);
const empty = value => value == null || value === false || (typeof value === "object" && Object.keys(value).length === 0);

/** Stateless native presentation; references always resolve in this revision. */
export class PilotV2View extends PilotView {
  reference(kind, value) {
    const suffix = kind === "printed card reference" ? `:${value.definition}` : "";
    const reference = `p2r:${digest([this.id, kind, suffix])}`;
    this.references.set(reference, { kind, value: structuredClone(value) });
    return reference;
  }

  constructor(view, catalog) {
    super(view, catalog);
    const packet = this.packet;
    packet.presentation = "pilot-v2";
    if (Object.hasOwn(packet.host, "yield")) {
      packet.host.currentYield = packet.host.yield;
      delete packet.host.yield;
    }
    if (packet.host.receipt && Object.hasOwn(packet.host.receipt, "yield")) {
      packet.host.receipt.yieldAtAcceptance = packet.host.receipt.yield;
      delete packet.host.receipt.yield;
    }
    const position = packet.position;
    if (position) {
      position.reference = this.reference("canonical observation", view.observation);
      const read = collection => collection.reference ? this.references.get(collection.reference).value : collection.items;
      for (const [zone, collection] of Object.entries(position.zones)) {
        if (!collection) continue;
        const summarize = (item, path) => Array.isArray(item)
          ? item.map((child, index) => summarize(child, `${path}.${index}`))
          : this.summary(item, path);
        position.zones[zone] = this.collection(zone, read(collection).map((item, index) => summarize(item, `${zone}.${index}`)));
      }
      const names = objectNames(view.observation);
      const choices = read(position.choices).map(choice => ({ ...choice,
        label: conciseActionLabel(choice.action, names) }));
      position.choices = this.collection("choices", choices);
      position.rules = this.collection("rules", read(position.rules).map(rule => {
        const card = rule.reference ? this.references.get(rule.reference).value : null;
        return { definition: rule.definition, available: rule.available, name: rule.name,
          ...(card ? { printed: concisePrinted(card) } : { printed: null }), reference: rule.reference };
      }));
    }
    validatePilotV2(packet);
  }

  summary(value, path) {
    if (!value || typeof value !== "object") return { current: { value }, reference: this.reference(path, value) };
    const summary = { reference: this.reference(path, value) };
    const current = {};
    for (const [key, item] of Object.entries(value)) {
      if (CORE.includes(key)) {
        if (item !== null) summary[key] = item;
      } else if (!METADATA.has(key) && !(DEFAULTS.has(key) && empty(item))) current[key] = item;
    }
    if (Object.keys(current).length) summary.current = current;
    return summary;
  }
}

function concisePrinted(card) {
  // Catalog roots repeat the primary part for protocol-v1 compatibility. A
  // single-part card needs its characteristics once; full catalog data remains
  // inspectable through the rule reference.
  if (card.structure?.kind === "single" && card.parts?.length === 1) return concisePrinted(card.parts[0]);
  const result = {};
  for (const key of ["rulesText", "manaCost", "typeLine", "power", "toughness", "loyalty"]) {
    if (card[key] != null) result[key] = card[key];
  }
  if (card.parts?.length) result.parts = card.parts.map(part => ({ id: part.id, name: part.name, ...concisePrinted(part) }));
  return result;
}

export function conciseActionLabel(action, names) {
  let label = actionLabel(action, names);
  if (action.ability != null) {
    label += `; ability ${abilityLabel(action.ability)}`;
  }
  const targets = [];
  function visit(value) {
    if (Array.isArray(value)) return value.forEach(visit);
    if (!value || typeof value !== "object") return;
    if (value.type === "player" && value.seat) targets.push(value.seat);
    else if (value.player) targets.push(value.player);
    if (value.objectId !== undefined) targets.push(objectLabel(value.objectId, names));
    if (value.permanent !== undefined) targets.push(objectLabel(value.permanent, names));
    for (const [key, child] of Object.entries(value)) if (!["seat", "player", "objectId", "permanent"].includes(key)) visit(child);
  }
  visit(action.targets);
  if (targets.length) label += `; target ${[...new Set(targets)].join(", ")}`;
  if (action.x != null && action.x !== 0) label += `; X=${action.x}`;
  if (action.modeIds?.length) label += `; modes ${action.modeIds.join(",")}`;
  return label;
}

// Sort descriptor keys so equivalent JSON objects have the same identity even
// when a transport or caller changes property insertion order.
function digest(value) {
  const canonical = item => Array.isArray(item) ? item.map(canonical)
    : item && typeof item === "object"
      ? Object.fromEntries(Object.keys(item).sort().map(key => [key, canonical(item[key])])) : item;
  return createHash("sha256").update(JSON.stringify(canonical(value))).digest("base64url");
}

function abilityLabel(ability) {
  const { kind, abilityId, partId } = ability;
  if (["printed", "token", "emblem", "faceDown"].includes(kind) && Number.isInteger(abilityId)) {
    // Printed origins may refer to different definitions on the same source.
    // A short definition fingerprint distinguishes them without a full UUID.
    const origin = kind === "printed" ? ` ${digest(ability.definition ?? null).slice(0, 8)}` : "";
    return `${kind}${origin} ${partId === undefined ? "" : `${partId}.`}${abilityId}`;
  }
  return `${kind ?? "other"} ${digest(ability).slice(0, 12)}`;
}
