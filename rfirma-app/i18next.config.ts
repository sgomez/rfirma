import { defineConfig } from "i18next-cli";

/**
 * `i18next-cli` entra como **vigilante, nunca como dueño del catálogo**
 * (ID-127).
 *
 * El catálogo lo manda `po/`; lo que aquí se configura es lo único que la
 * cadena `.pot` → `.po` → `.ts` no puede hacer: **mirar el código**. Dos
 * fallos, y son los dos que existe para cazar:
 *
 * - `i18next-cli extract --ci` — una `t()` cuya clave no está en el catálogo.
 * - `i18next-cli status --unused` — una clave del catálogo que ya no usa nadie.
 *
 * Y `i18next-cli types`, que lleva ese mismo aviso al editor.
 *
 * Dos ajustes son **condiciones medidas**, no gustos:
 *
 * - **`defaultNS: false`**. Con el espacio de nombres puesto, `extract` envuelve
 *   las claves en él y `status` reporta un **0 % falso**: no falla, miente.
 * - **`outputFormat: "ts"`**, que emite `export default`. Una exportación con
 *   nombre es invisible para el cargador de la herramienta, y otra vez sale el
 *   0 % con el fichero lleno delante. Es la misma forma que escribe
 *   `tools/po-import.mjs`, que es quien de verdad genera estos ficheros.
 *
 * Ojo con `extract` a secas: **escribiría** sobre `src/i18n/locales/`, que son
 * ficheros generados. Por eso el `justfile` solo lo invoca con `--ci`, que no
 * escribe y sale con 1 si algo faltaba.
 */
export default defineConfig({
  // Solo el idioma de referencia. Las instantáneas de
  // `node_modules/.cache/i18next-cli/` las escribe `po-import`, y **solo para
  // los idiomas al 100 %** (ID-123): en cuanto `en.po` baje del 100 % —lo
  // normal en el instante en que se añade una cadena castellana— no habría
  // `en.ts` en la caché y `extract --ci` daría por ausentes todas sus claves,
  // un muro de falsos positivos por un idioma incompleto, que es justo lo que
  // `check-po` documenta que NO es un fallo. Las dos preguntas del ID-127
  // —¿hay una `t()` sin entrada? ¿sobra alguna clave?— se contestan enteras
  // con `es`, el único catálogo que `po-import` garantiza.
  locales: ["es"],
  extract: {
    input: ["src/**/*.{ts,tsx}"],
    ignore: ["src/**/*.test.{ts,tsx}", "src/i18n/locales/**"],
    output: "node_modules/.cache/i18next-cli/{{language}}.ts",
    outputFormat: "ts",
    defaultNS: false,
    // Por omisión vale `true`, y entonces `extract` BORRA del catálogo toda
    // clave que no vea en el código: sobre ficheros generados eso es
    // reescribirlos enteros. Quien informa de lo que sobra es
    // `status --unused`, que no escribe nada.
    removeUnusedKeys: false,
    // Las claves que el código compone (`t(`errors.situations.${situation}.title`)`).
    // La herramienta resuelve solas las uniones que declara un `as const` en el
    // mismo módulo —`preferences.sections.*`, `progress.stages.*`—, pero no las
    // que salen de un tipo importado; esas se nombran aquí o `status --unused`
    // las daría por muertas.
    // `actions.chooseCertificate` no es dinámica: no la usa nadie, y la encontró
    // `status --unused` al montar este circuito. Se queda anotada aquí porque
    // **la poda es del sub-issue siguiente** del #168, que borra la entrada del
    // .pot y esta línea a la vez. `status.ignoreKeys` no vale para esto: lo
    // probamos y `--unused` la sigue reportando.
    preservePatterns: [
      "errors.situations.*",
      "languages.*",
      "panel.certificate.stores.*",
      "actions.chooseCertificate",
    ],
  },
  types: {
    input: ["node_modules/.cache/i18next-cli/es.ts"],
    output: "src/i18n/i18next.d.ts",
  },
});
