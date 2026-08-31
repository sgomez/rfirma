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
      notAnAcceptedImage: {
        title: "",
        body: "",
      },
      damagedImage: {
        title: "",
        body: "",
      },
      imageTooLarge: {
        title: "",
        body: "",
      },
      sourceUnreadable: {
        title: "",
        body: "",
      },
      storeUnwritable: {
        title: "",
        body: "",
      },
      storeUnreadable: {
        title: "",
        body: "",
      },
      incorrectPin: {
        title: "",
        body: "",
      },
      pinLocked: {
        title: "",
        body: "",
      },
      tokenAbsent: {
        title: "",
        body: "",
      },
      expiredSession: {
        title: "",
        body: "",
      },
      moduleNotFound: {
        title: "",
        body: "",
      },
      certificateNotFound: {
        title: "",
        body: "",
      },
    },
  },
  window: {
    tray: "",
    viewer: "",
    panel: "",
  },
  header: {
    menu: "",
    preferences: "",
    about: "",
  },
  badges: {
    signed: "",
    unsigned: "",
    unavailable: "",
  },
  tray: {
    dropZone: "",
    empty: "",
    recents: "",
    remove: "",
  },
  viewer: {
    dropZone: "",
    privacy: "",
    pageNumber: "",
    pageOf: "",
    firstPage: "",
    previousPage: "",
    nextPage: "",
    lastPage: "",
    zoomOut: "",
    zoomIn: "",
    fitToWindow: "",
    signatureBox: "",
    dragHandle: "",
    outOfPage: "",
  },
  panel: {
    document: {
      pages: "",
    },
    coSignature: {
      one: "",
      many: "",
    },
    certificate: {
      title: "",
      loading: "",
      issuer: "",
      choose: "",
      empty: {
        title: "",
        body: "",
        retry: "",
        otherModule: "",
      },
      expired: "",
      notYetValid: "",
      revoked: "",
      unreadable: "",
    },
    visibleSignature: {
      title: "",
      toggle: "",
      placement: "",
      noPlacement: "",
      content: "",
      fields: {
        rubric: "",
        rubricDisabled: "",
        signerName: "",
        idNumber: "",
        signedAt: "",
        reason: "",
      },
      reason: {
        label: "",
        placeholder: "",
      },
      rubric: {
        title: "",
        choose: "",
        change: "",
        thumbnail: "",
        flattened: "",
      },
      preview: {
        title: "",
        empty: "",
        unavailable: "",
      },
    },
    footer: {
      savedIn: "",
      unwritable: "",
      retry: "",
    },
  },
  pin: {
    title: "",
    signingAs: "",
    label: "",
    hint: "",
    incorrect: "",
    incorrectOne: "",
    incorrectUnknown: "",
    submit: "",
  },
  preferences: {
    title: "",
    rememberVisibleSignature: {
      label: "",
      hint: "",
    },
    rememberActivity: {
      label: "",
      hint: "",
      clear: "",
      confirm: {
        body: "",
        accept: "",
      },
    },
    destination: {
      label: "",
    },
    language: {
      label: "",
    },
  },
  about: {
    version: "",
    whatItDoes: "",
    independence: "",
    licenses: {
      title: "",
      view: "",
      rfirma: "",
      afirma: "",
    },
  },
};
