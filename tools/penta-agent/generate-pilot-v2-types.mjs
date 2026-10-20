import { writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { declarations } from "./generate-pilot-types.mjs";
import { pilotV2Schema } from "./pilot-v2-contract.mjs";

export const pilotV2Declarations = () => declarations(pilotV2Schema, "composed pilot-v2.schema.json");
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  writeFileSync(new URL("./pilot-v2.d.ts", import.meta.url), pilotV2Declarations());
}
