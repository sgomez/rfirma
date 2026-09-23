// Cómo se declara un guion en el manifiesto: su sede, su familia, sus modos y sus condiciones.

/** Los modos en los que puede correr un guion de la sede publicada, salvo los del intermedio. */
export const THE_PUBLISHED_SITE_MODES = ["v4", "v3", "service", "service-bind-failure"];

/** Un guion de la sede publicada: una operación de punta a punta por el `autoscript.js`. */
export function aPublishedScript(
  run,
  {
    conditions = [],
    modes = THE_PUBLISHED_SITE_MODES,
    benchOnly = false,
    patch,
    family = "end-to-end",
  } = {},
) {
  return { site: "published", family, modes, conditions, benchOnly, patch, run };
}

/** Un guion de la sede a mano: mensajes del protocolo escritos en crudo, sin `autoscript.js`. */
export function aHandwrittenScript(run, { family, modes, conditions = [] }) {
  return { site: "handwritten", family, modes, conditions, benchOnly: false, run };
}
