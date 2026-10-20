import { readFileSync } from "node:fs";

export const pilotSchema = JSON.parse(readFileSync(new URL("./pilot.schema.json", import.meta.url), "utf8"));

/** Validate the deliberately small JSON Schema vocabulary used by this contract.
 * The same schema is consumable by standard draft-2020-12 validators. */
export function validateContract(value, definition = "PilotPacket", schemaDocument = pilotSchema) {
  const visit = (value, schema, path) => {
    if (schema === true) return;
    if (schema === false) throw new Error(`${path}: unexpected field`);
    if (schema.$ref) return visit(value, schemaDocument.$defs[schema.$ref.split("/").at(-1)], path);
    const alternatives = schema.oneOf ?? schema.anyOf;
    if (alternatives) {
      let matches = 0;
      for (const alternative of alternatives) {
        try { visit(value, alternative, path); matches++; } catch { /* Try the other allowed shapes. */ }
      }
      if (!matches || (schema.oneOf && matches !== 1)) throw new Error(`${path}: invalid union value`);
      return;
    }
    if (Object.hasOwn(schema, "const") && value !== schema.const) throw new Error(`${path}: expected ${schema.const}`);
    if (schema.enum && !schema.enum.includes(value)) throw new Error(`${path}: unknown enum value`);
    const type = value === null ? "null" : Array.isArray(value) ? "array" : typeof value;
    if (schema.type && !(schema.type === "integer" ? Number.isInteger(value) : type === schema.type)) throw new Error(`${path}: expected ${schema.type}`);
    if (typeof value === "number" && (!Number.isFinite(value) || value < (schema.minimum ?? -Infinity) || value > (schema.maximum ?? Infinity))) throw new Error(`${path}: number out of bounds`);
    if (typeof value === "string") {
      const length = [...value].length;
      if (length < (schema.minLength ?? 0)) throw new Error(`${path}: string too short`);
      if (length > (schema.maxLength ?? Infinity)) throw new Error(`${path}: string too long`);
    }
    if (Array.isArray(value)) {
      if (value.length < (schema.minItems ?? 0) || value.length > (schema.maxItems ?? Infinity)) throw new Error(`${path}: invalid item count`);
      value.forEach((item, index) => visit(item, schema.items ?? true, `${path}[${index}]`));
    } else if (value && typeof value === "object") {
      for (const key of schema.required ?? []) if (!Object.hasOwn(value, key)) throw new Error(`${path}.${key}: required`);
      for (const [key, item] of Object.entries(value)) visit(item, schema.properties?.[key] ?? schema.additionalProperties ?? true, `${path}.${key}`);
    }
  };
  if (!schemaDocument.$defs[definition]) throw new Error(`unknown contract ${definition}`);
  visit(value, schemaDocument.$defs[definition], definition);
  return value;
}

export function validatePilot(packet) {
  validateContract(packet);
  if (packet.host.status !== packet.status) throw new Error("host and pilot status disagree");
  if (packet.host.revision !== undefined && packet.host.revision !== packet.revision) throw new Error("host and pilot revision disagree");
  const position = packet.position;
  if (position) {
    const collections = [...Object.values(position.zones), position.updates, position.choices,
      position.decision?.options, position.rules].filter(Boolean);
    for (const collection of collections) {
      if (collection.reference === null ? collection.items.length !== collection.total : collection.items.length > collection.total) {
        throw new Error("collection contents disagree with total");
      }
    }
    const tickets = position.choices.items.map(choice => choice.ticket);
    if (new Set(tickets).size !== tickets.length) throw new Error("duplicate action tickets");
  }
  return packet;
}
