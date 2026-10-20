import { readFileSync } from "node:fs";
import { pilotSchema, validateContract } from "./pilot-contract.mjs";

/** Compose shared native contracts into a standalone draft-2020-12 document.
 * The checked-in v2 fragment contains only definitions changed by presentation. */
export function composePilotV2Schema() {
  const schema = structuredClone(pilotSchema);
  const changes = JSON.parse(readFileSync(new URL("./pilot-v2.schema.json", import.meta.url), "utf8"));
  Object.assign(schema, changes, { $defs: { ...schema.$defs, ...changes.$defs } });
  const rename = (name, before, after) => {
    const properties = schema.$defs[name].properties;
    properties[after] = properties[before];
    delete properties[before];
  };
  rename("Host", "yield", "currentYield");
  rename("Receipt", "yield", "yieldAtAcceptance");
  schema.$defs.Receipt.properties.yieldInstallation = {
    type: "object", properties: {
      status: { enum: ["installed", "notInstalled"] },
      reason: { enum: [null, "notRequested", "partialBatch", "paused", "complete", "decision"] },
    }, required: ["status", "reason"], additionalProperties: false,
  };
  schema.$defs.Host.properties.eventDelivery = {
    type: "object", properties: {
      after: { type: "integer", minimum: 0 }, next: { type: "integer", minimum: 0 },
      items: { type: "array", items: { $ref: "#/$defs/JsonValue" } },
    }, required: ["after", "next"], additionalProperties: false,
  };
  for (const variant of schema.$defs.PilotPacket.oneOf) variant.properties.presentation.const = "pilot-v2";
  const position = schema.$defs.Position;
  position.properties.reference = { type: "string" };
  position.required.push("reference");
  position.properties.choiceGroups = {
    type: "array", items: { type: "object", properties: {
      example: { type: "string" }, firstIndex: { type: "integer", minimum: 0 },
      count: { type: "integer", minimum: 1 },
    }, required: ["example", "firstIndex", "count"], additionalProperties: false },
  };
  for (const value of Object.values(position.properties.zones.properties)) value.anyOf[0].$ref = "#/$defs/ZoneSummaries";
  return schema;
}

export const pilotV2Schema = composePilotV2Schema();
export function validatePilotV2(packet) {
  validateContract(packet, "PilotPacket", pilotV2Schema);
  if (packet.host.status !== packet.status) throw new Error("host and pilot status disagree");
  if (packet.host.revision !== undefined && packet.host.revision !== packet.revision) throw new Error("host and pilot revision disagree");
  if (packet.position) {
    const position = packet.position;
    for (const collection of [...Object.values(position.zones), position.updates, position.choices, position.decision?.options, position.rules].filter(Boolean)) {
      if (collection.reference === null ? collection.items.length !== collection.total : collection.items.length > collection.total) throw new Error("collection contents disagree with total");
    }
    const tickets = position.choices.items.map(choice => choice.ticket);
    if (new Set(tickets).size !== tickets.length) throw new Error("duplicate action tickets");
  }
  return packet;
}
