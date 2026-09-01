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
      notAnAcceptedImage: {
        title: "That image cannot be used as a rubric",
        body: "The rubric has to be a PNG or a JPEG.",
      },
      damagedImage: {
        title: "The image could not be opened",
        body: "The file is a PNG or a JPEG, but it is damaged. Try another copy.",
      },
      imageTooLarge: {
        title: "The image is too large",
        body: "Choose a smaller image, or save it at a lower resolution.",
      },
      sourceUnreadable: {
        title: "The file you chose could not be read",
        body: "Check that it is still where it was and choose it again.",
      },
      storeUnwritable: {
        title: "Your rubric could not be saved",
        body: "Choose the image again. If it keeps happening, attach the technical detail to the bug report.",
      },
      storeUnreadable: {
        title: "Your saved rubric could not be read",
        body: "Choose the image again to replace it.",
      },
      incorrectPin: {
        title: "The PIN is not correct",
        body: "Type it again. The card locks after a few failed attempts.",
      },
      pinLocked: {
        title: "The card is locked",
        body: "It locked after too many failed PIN attempts. Unlock it with its PUK before signing again.",
      },
      tokenAbsent: {
        title: "We cannot find the card",
        body: "Check that it is still inserted and that the reader is connected, then try again.",
      },
      expiredSession: {
        title: "The session with the card expired",
        body: "Try again: a new session will be opened and we will ask for the PIN once more.",
      },
      moduleNotFound: {
        title: "The card module could not be loaded",
        body: "Check that the PKCS#11 module is where you said it was, or choose another one.",
      },
      certificateNotFound: {
        title: "The certificate is no longer on the card",
        body: "Look for the certificates again and choose one of those that turn up.",
      },
      certificateExpired: {
        title: "The certificate has expired",
        body: "An expired certificate cannot sign. Renew it with its issuer and try again.",
      },
      certificateNotYetValid: {
        title: "The certificate is not valid yet",
        body: "Its validity period has not started, so it cannot sign.",
      },
      certificateRevoked: {
        title: "The certificate is revoked",
        body: "Its issuer revoked it, so it cannot sign. You need a new one.",
      },
      certificateUnreadable: {
        title: "The certificate could not be read",
        body: "What is on the card is not a certificate we can read. Try another one.",
      },
      notAPdf: {
        title: "That file is not a PDF",
        body: "rFirma signs PDFs. Choose a PDF document and try again.",
      },
      documentEncrypted: {
        title: "The PDF is protected",
        body: "It is encrypted or has restricted permissions, so it cannot be signed. Open it with its password, save it without protection and try again.",
      },
      documentCertified: {
        title: "The PDF is certified",
        body: "Its author signed it forbidding changes, so adding a signature would invalidate theirs. Ask them for an uncertified copy.",
      },
      documentUnreadable: {
        title: "The document could not be read",
        body: "Check that it is still where it was and open it again.",
      },
      droppedFileUnreadable: {
        title: "We could not read the file you dropped",
        body: "Dropping a file does not give us permission to read from every folder. Open it with the open button, which does.",
      },
      droppedOnlyFirst: {
        title: "We only opened the first PDF",
        body: "rFirma signs one document at a time. The other files you dropped are still where they were: drop them once you are done with this one.",
      },
      boxOutOfPage: {
        title: "The box falls outside the page",
        body: "Drag it inside the page and sign again. Left as it is, it would be clipped in the PDF without warning.",
      },
      sealMismatch: {
        title: "Signing was interrupted halfway",
        body: "What we prepared at the start and what came back at the end do not match, so we stopped before writing anything. Sign the document again.",
      },
      bridgeFailed: {
        title: "The signature could not be prepared",
        body: "The component that assembles the signed PDF failed. Try again and, if it keeps happening, attach the technical detail to the bug report.",
      },
      folderMissing: {
        title: "The destination folder is gone",
        body: "We do not create it for you: if we did, the document would look saved and would not be there. Create it again or choose another one in Preferences.",
      },
      notAFolder: {
        title: "The destination is not a folder",
        body: "There is a file with that name where we expected a folder. Choose another destination in Preferences.",
      },
      folderUnreadable: {
        title: "The destination folder could not be checked",
        body: "It may be a permissions problem or a drive that is not responding. Choose another destination in Preferences.",
      },
      folderUnwritable: {
        title: "The signed document could not be saved",
        body: "Signing worked, but the file could not be written to the destination folder. Choose another one in Preferences and sign again.",
      },
      noFreeName: {
        title: "There are already too many documents with that name",
        body: "We stopped rather than overwrite one. Move or rename the earlier ones and sign again.",
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
    dropZoneHint: "The file browser will open",
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
  panel: {
    document: {
      pages: {
        one: "1 page",
        many: "{{pages}} pages",
      },
    },
    coSignature: {
      one: "It already carries 1 signature: yours will be a countersignature.",
      many: "It already carries {{count}} signatures: yours will be a countersignature.",
    },
    certificate: {
      title: "Certificate",
      loading: "Looking for certificates…",
      issuer: "Issued by {{issuer}}",
      choose: "Choose certificate",
      empty: {
        title: "We found no certificate",
        body: "If you use a card, check that it is inserted and that the reader is connected.",
        retry: "Look again",
        otherModule: "Another module…",
      },
      failed: {
        title: "We could not look for certificates",
        body: "The search failed before finishing, so we do not know whether there is any.",
        retry: "Look again",
        otherModule: "Another module…",
      },
      expired: "The certificate expired on {{date}}, so it cannot be used to sign.",
      notYetValid: "The certificate is not valid yet, so it cannot be used to sign.",
      revoked: "The certificate is revoked ({{reason}}), so it cannot be used to sign.",
      unreadable: "This certificate could not be read, so it cannot be used to sign.",
    },
    visibleSignature: {
      title: "Visible signature",
      toggle: "Stamp a signature box on the document",
      placement: "Page {{page}} · drag it to place it",
      noPlacement: "Drag the box over the document to place it.",
      content: "Contents",
      fields: {
        rubric: "Your rubric",
        rubricDisabled: "Choose an image first",
        signerName: "Name and surname",
        idNumber: "ID number",
        signedAt: "Date and time of the signature",
        reason: "A reason",
      },
      reason: {
        label: "Reason",
        placeholder: "Approved",
      },
      rubric: {
        title: "Rubric image",
        choose: "Choose image",
        change: "Change image",
        thumbnail: "Your rubric, as it will be stamped",
        flattened:
          "It is stamped on white: the box goes into the PDF as a JPEG, and a JPEG has no transparency.",
      },
      preview: {
        title: "What the box will say",
        empty: "Tick a box so that the signature box says something.",
        unavailable: "The preview will show up once you choose the certificate.",
      },
    },
    footer: {
      savedIn: "It will be saved in",
      unwritable: "Cannot write in {{folder}}",
      retry: "Try again",
    },
    signed: {
      summary: "Summary",
      format: "PAdES",
      signAnother: "Sign another document",
    },
  },
  pin: {
    title: "Enter the card PIN",
    signingAs: "{{holder}} · {{idNumber}}",
    label: "PIN",
    hint: "The PIN is used for this signature only, and is not stored anywhere.",
    incorrect: "Wrong PIN. You have {{attempts}} attempts left before the card locks.",
    incorrectOne: "Wrong PIN. You have 1 attempt left before the card locks.",
    incorrectUnknown: "Wrong PIN. The card locks after a few failed attempts.",
    submit: "Sign",
  },
  progress: {
    title: "Signing the document…",
    stages: {
      presign: "Preparing the signature",
      presignTerm: "presignature",
      sign: "Signing on the card",
      postsign: "Assembling the PDF",
      postsignTerm: "postsignature",
    },
    states: {
      done: "Done",
      running: "Under way",
      pending: "Pending",
    },
    keepTheCard: "Do not remove the card until it finishes.",
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
    theme: {
      label: "Theme",
      system: "System",
      light: "Light",
      dark: "Dark",
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
    repository: "github.com/sgomez/rfirma",
    licenses: {
      view: "View the licences",
      rfirma: "rFirma: EUPL-1.2.",
      afirma: "Cliente @firma libraries: GPL-2.0+ / EUPL-1.1.",
    },
  },
};
