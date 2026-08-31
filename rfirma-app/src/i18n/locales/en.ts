import type { Catalog } from "../catalog";

/**
 * El catálogo en inglés, el otro que lleva contenido en v0.1 (ADR-0009).
 *
 * Los nombres de los idiomas van en endónimo también aquí: un desplegable de
 * idiomas enseña cada lengua como se llama a sí misma.
 */
export const en: Catalog = {
  app: {
    name: "rfirma",
  },
  actions: {
    sign: "Sign document",
    chooseCertificate: "Choose certificate",
    cancel: "Cancel",
    close: "Close",
    change: "Change",
  },
  languages: {
    es: "Español",
    ca: "Català",
    eu: "Euskara",
    gl: "Galego",
    va: "Valencià",
    en: "English",
  },
  errors: {
    technicalDetail: "Technical detail",
    situations: {
      unknown: {
        title: "The operation could not be completed",
        body: "Try again. If it keeps happening, attach the technical detail to the bug report.",
      },
    },
  },
};
