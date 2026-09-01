import type { UntranslatedCatalog } from "../catalog";

/**
 * El catálogo en valencià. Las claves están, los textos no: en v0.1 solo `es` e
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
export const va: UntranslatedCatalog = {
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
      certificateExpired: {
        title: "",
        body: "",
      },
      certificateNotYetValid: {
        title: "",
        body: "",
      },
      certificateRevoked: {
        title: "",
        body: "",
      },
      certificateUnreadable: {
        title: "",
        body: "",
      },
      notAPdf: {
        title: "",
        body: "",
      },
      documentEncrypted: {
        title: "",
        body: "",
      },
      documentCertified: {
        title: "",
        body: "",
      },
      documentUnreadable: {
        title: "",
        body: "",
      },
      droppedFileUnreadable: {
        title: "",
        body: "",
      },
      droppedOnlyFirst: {
        title: "",
        body: "",
      },
      boxOutOfPage: {
        title: "",
        body: "",
      },
      sealMismatch: {
        title: "",
        body: "",
      },
      bridgeFailed: {
        title: "",
        body: "",
      },
      folderMissing: {
        title: "",
        body: "",
      },
      notAFolder: {
        title: "",
        body: "",
      },
      folderUnreadable: {
        title: "",
        body: "",
      },
      folderUnwritable: {
        title: "",
        body: "",
      },
      noFreeName: {
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
    dropZoneHint: "",
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
      pages: {
        one: "",
        many: "",
      },
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
      list: "",
      stores: {
        card: "",
        firefox: "",
        chrome: "",
        nssdb: "",
      },
      empty: {
        title: "",
        body: "",
        retry: "",
        otherModule: "",
      },
      failed: {
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
    signed: {
      summary: "",
      format: "",
      signAnother: "",
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
  progress: {
    title: "",
    stages: {
      presign: "",
      presignTerm: "",
      sign: "",
      postsign: "",
      postsignTerm: "",
    },
    states: {
      done: "",
      running: "",
      pending: "",
    },
    keepTheCard: "",
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
    theme: {
      label: "",
      system: "",
      light: "",
      dark: "",
    },
    language: {
      label: "",
    },
  },
  about: {
    version: "",
    whatItDoes: "",
    independence: "",
    repository: "",
    licenses: {
      view: "",
      rfirma: "",
      afirma: "",
    },
  },
};
