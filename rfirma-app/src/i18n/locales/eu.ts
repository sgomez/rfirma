import type { UntranslatedCatalog } from "../catalog";

/**
 * El catálogo en euskara. Las claves están, los textos no: en v0.1 solo `es` e
 * `en` llevan contenido y este idioma cae al castellano (ADR-0009). No es un
 * olvido y no se arregla generando la traducción: nadie va a revisarla antes
 * de v0.1, y traducción sin revisar en una aplicación de firma es peor que su
 * ausencia.
 *
 * Para rellenarlo: escribe los textos y cambia el tipo a `Catalog`; el idioma
 * aparecerá solo en el desplegable de Preferencias, que enseña únicamente los
 * catálogos completos. Los `.properties` cooficiales de AutoFirma valen como
 * referencia terminológica, no como origen de las cadenas.
 */
export const eu: UntranslatedCatalog = {
  app: {
    name: "",
  },
  actions: {
    sign: "",
    chooseCertificate: "",
    cancel: "",
    close: "",
    change: "",
  },
  languages: {
    es: "",
    ca: "",
    eu: "",
    gl: "",
    va: "",
    en: "",
  },
  errors: {
    technicalDetail: "",
    situations: {
      unknown: {
        title: "",
        body: "",
      },
    },
  },
};
