// El vocabulario de la sede: sus modos y sus guiones, cada uno con su sede, su familia y sus condiciones.

import { MODES } from "./lib/modes.mjs";
import { BATCH_SCRIPTS } from "./scripts/batch.mjs";
import { CERTIFICATE_SCRIPTS } from "./scripts/certificate.mjs";
import { FILE_SCRIPTS } from "./scripts/files.mjs";
import { RELAY_SCRIPTS } from "./scripts/relay.mjs";
import { REQUEST_SCRIPTS } from "./scripts/request.mjs";
import { SERVICE_SCRIPTS } from "./scripts/service.mjs";
import { SIGNATURE_SCRIPTS } from "./scripts/signature.mjs";
import { WEBSOCKET_SCRIPTS } from "./scripts/websocket.mjs";

export { MODES };

export const SCRIPTS = {
  ...CERTIFICATE_SCRIPTS,
  ...SIGNATURE_SCRIPTS,
  ...FILE_SCRIPTS,
  ...BATCH_SCRIPTS,
  ...RELAY_SCRIPTS,
  ...WEBSOCKET_SCRIPTS,
  ...SERVICE_SCRIPTS,
  ...REQUEST_SCRIPTS,
};

/** Lo que `--manifest` publica para que la suite valide su catálogo contra ello. */
export function theManifest() {
  return {
    modes: Object.fromEntries(
      Object.entries(MODES).map(([name, mode]) => [name, { bench_only: mode.benchOnly ?? false }]),
    ),
    scripts: Object.fromEntries(
      Object.entries(SCRIPTS).map(([name, script]) => [
        name,
        {
          site: script.site,
          family: script.family,
          modes: script.modes,
          conditions: script.conditions,
          bench_only: script.benchOnly,
        },
      ]),
    ),
  };
}
