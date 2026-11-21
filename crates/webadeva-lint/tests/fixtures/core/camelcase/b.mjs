import { some_import } from "x";
import { orig_name as local_name } from "x";
import { okName as other_local } from "x";
import default_import, * as ns_import from "x";
import { camelOk as camel_alias } from "x";
export { some_import as export_name };
export { local_name };
export * as ns_export from "x";
export * from "x";
const used = some_import;
export const exported_const = 1;
export function exported_fn() {}
export default function default_fn() {}
import json_data from "./a.json" with { type_key: "json" };
