import type { Catalog } from "../catalog";

/**
 * El catálogo en inglés, el otro que lleva contenido en v0.1 (ADR-0009).
 *
 * Los nombres de los idiomas van en endónimo también aquí: un desplegable de
 * idiomas enseña cada lengua como se llama a sí misma.
 */
export const en: Catalog = {
  app: {
    name: "rFirma",
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
  window: {
    tray: "Document tray",
    viewer: "Document viewer",
    panel: "Signature panel",
  },
  header: {
    menu: "Menu",
    preferences: "Preferences…",
    about: "About rFirma",
  },
  badges: {
    signed: "Signed",
    unsigned: "Unsigned",
    unavailable: "Unavailable",
  },
  tray: {
    dropZone: "Drag a PDF here, or click to open one",
    empty: "The documents you sign will show up here",
    recents: "Recent",
    remove: "Remove from the list",
  },
  viewer: {
    dropZone: "Drag a PDF here, or click to open one",
    privacy: "PDF only. The document never leaves your computer.",
    pageNumber: "Page number",
    pageOf: "of {{total}}",
    firstPage: "First page",
    previousPage: "Previous page",
    nextPage: "Next page",
    lastPage: "Last page",
    zoomOut: "Zoom out",
    zoomIn: "Zoom in",
    fitToWindow: "Fit to window",
    signatureBox: "Visible signature box",
    dragHandle: "Drag to place",
    outOfPage: "The box ended up off the page, so it stayed where it was.",
  },
  preferences: {
    title: "Preferences",
    rememberVisibleSignature: {
      label: "Remember the last visible signature setup",
      hint: "The page, the position and the contents of the box are reused on the next document.",
    },
    rememberActivity: {
      label: "Remember my activity",
      hint: "Covers the recent documents and the certificate used last time.",
      clear: "Empty the list",
      confirm: {
        body: "Turning it off erases what is already remembered: the recent documents and the certificate used last time.",
        accept: "Turn off and erase",
      },
    },
    destination: {
      label: "Where the signed document is saved",
    },
    language: {
      label: "Language",
    },
  },
  about: {
    version: "Version {{version}}",
    whatItDoes:
      "Signs and countersigns PDF documents with your certificate. Neither the document nor the private key leaves your computer.",
    independence:
      "Independent project. rFirma is not related to AutoFirma or to the Spanish Administration, who publish the official client, nor is it endorsed by them. If you need the official application, download it from their website.",
    licenses: {
      title: "Licences",
      view: "View the licences",
      rfirma: "rFirma: EUPL-1.2.",
      afirma: "Cliente @firma libraries: GPL-2.0+ / EUPL-1.1.",
    },
  },
};
