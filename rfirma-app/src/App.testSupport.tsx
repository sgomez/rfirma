import { screen } from "@testing-library/react";
import type { UserEvent } from "@testing-library/user-event";
import { App } from "./App";
import {
  type ExternalDestinationOpener,
  unavailableExternalDestinationOpener,
} from "./desktop/externalDestination";
import type { DocumentInHand } from "./documents/document";
import type { Drop, FakeDocumentDrops } from "./documents/drops";
import { inMemoryDocumentDrops } from "./documents/drops";
import { inMemoryDocumentPicker } from "./documents/picker";
import { inMemoryRecents, type RecentDocument } from "./documents/recents";
import type { Preferences } from "./preferences/preferences";
import { inMemoryPreferences } from "./preferences/preferences";
import type { Certificate, CertificateStore } from "./signing/certificate";
import { emptyCertificateStore } from "./signing/certificate";
import { inMemoryDestination, unavailableOpener } from "./signing/destination";
import { type SigningBackend, unavailableSigningBackend } from "./signing/flow";
import { emptyRubricPicker, type RubricPicker } from "./signing/rubric";
import { unavailableStampComposer } from "./signing/stampPreview";
import type { StatusPort } from "./status/status";
import { renderWithCatalog } from "./testing/render";
import { inMemoryVersionCheck, type VersionCheck } from "./updates/newVersion";
import type { PdfDocument, PdfPage, Viewport } from "./viewer/pdf";
import { type PdfSource, unavailablePdfSource } from "./viewer/source";

/** El destino que contesta el backend mientras la prueba no diga otra cosa. */
export const aDestination = () =>
  inMemoryDestination({ folder: "Documentos", name: "contrato-firmado.pdf", writable: true });

/**
 * **El documento que se tiene delante**: lo que entra por el diálogo o por el
 * arrastre, y lo que se pinta y se firma. No es la fila (ID-287).
 */
export function document(name: string, overrides: Partial<DocumentInHand> = {}): DocumentInHand {
  return {
    // El identificador lo acuña el backend y es opaco: aquí se finge con un
    // prefijo que ninguna ruta tendría, para que nada pueda leerlo como tal.
    id: `id-${name}`,
    name,
    badge: "Unsigned",
    modified: 1_700_000_000,
    placement: null,
    remembered: true,
    ...overrides,
  };
}

/** **La fila que se guarda**: con lo que arrancan los recientes en una prueba. */
export function row(name: string, overrides: Partial<RecentDocument> = {}): RecentDocument {
  return {
    id: `id-${name}`,
    name,
    badge: "Unsigned",
    modified: 1_700_000_000,
    lastUsed: 1_700_000_000,
    available: true,
    placement: null,
    ...overrides,
  };
}

const A4 = { width: 595, height: 842 };

/** Un viewport de `pdf.js` sin rotación: escala y voltea el eje Y. */
function viewportAt(scale: number): Viewport {
  return {
    width: A4.width * scale,
    height: A4.height * scale,
    convertToPdfPoint: (x, y) => [x / scale, A4.height - y / scale],
    convertToViewportPoint: (x, y) => [x * scale, (A4.height - y) * scale],
  };
}

/** Un PDF que se deja pintar: `pdf.js` no cabe en `jsdom` (ver `pdf.ts`). */
function aPdfOf(pageCount: number): PdfDocument {
  const pageOf = (number: number): PdfPage => ({
    number,
    rotate: 0,
    view: [0, 0, A4.width, A4.height],
    getViewport: ({ scale }) => viewportAt(scale),
    render: () => ({ promise: Promise.resolve(), cancel: () => {} }),
  });
  return { pageCount, getPage: (number) => Promise.resolve(pageOf(number)) };
}

/** Un origen que abre cada documento con las páginas que se le digan. */
export function pdfsOf(pages: Record<string, number>): PdfSource {
  return {
    open: async (document) => {
      const pageCount = pages[document.name];
      if (pageCount === undefined) {
        return { ok: false, failure: { situation: "documentUnreadable", detail: "roto" } };
      }
      return { ok: true, pdf: aPdfOf(pageCount), sizeBytes: 2_400_000 };
    },
  };
}

/**
 * Un PDF de tamaños mezclados: cada página trae su propio `view`. Es lo que
 * hace falta para que `correctPositionSignature` se coma alguna en silencio
 * (ID-105) y para probarlo hace falta más de un tamaño en el mismo documento.
 */
function aPdfWithViews(views: readonly (readonly [number, number, number, number])[]): PdfDocument {
  const pageOf = (number: number): PdfPage => {
    const view = views[number - 1];
    if (view === undefined) throw new Error(`no hay view para la página ${number}`);
    return {
      number,
      rotate: 0,
      view,
      getViewport: ({ scale }) => viewportAt(scale),
      render: () => ({ promise: Promise.resolve(), cancel: () => {} }),
    };
  };
  return { pageCount: views.length, getPage: (number) => Promise.resolve(pageOf(number)) };
}

/** Un origen que abre `name` con las `views` que se le den, tamaños mezclados incluidos. */
export function pdfsWithViews(
  name: string,
  views: readonly (readonly [number, number, number, number])[],
): PdfSource {
  return {
    open: async (opened) => {
      if (opened.name !== name) {
        return { ok: false, failure: { situation: "documentUnreadable", detail: "roto" } };
      }
      return { ok: true, pdf: aPdfWithViews(views), sizeBytes: 2_400_000 };
    },
  };
}

export const aCertificate: Certificate = {
  id: "0123456789abcdef0123456789abcdef",
  label: "Firma",
  holderName: "Ada Lovelace Byron",
  idNumber: "99999999R",
  issuer: "AC FNMT Usuarios",
  store: "card",
  status: { kind: "valid", notAfter: 1_894_752_000 },
  remembered: false,
};

/**
 * Un almacén que **rechaza** las primeras `failures` búsquedas y a partir de
 * ahí devuelve lo que se le diga: es el token que no carga y que, arreglado el
 * problema, sí carga al volver a buscar.
 */
export function failingCertificateStore(failures: number, then: readonly Certificate[] = []) {
  let left = failures;
  const store: CertificateStore = {
    ...emptyCertificateStore(),
    list: async () => {
      if (left > 0) {
        left -= 1;
        // La forma que rechaza `invoke` cuando Rust ya clasificó el fallo.
        throw { situation: "moduleNotFound", detail: "CKR_GENERAL_ERROR" };
      }
      return then;
    },
  };
  return store;
}

export function renderApp(
  recents = inMemoryRecents(),
  documents: DocumentInHand[] = [],
  pdfs: PdfSource = unavailablePdfSource(),
  settings: Partial<Preferences> = {},
  certificates: Partial<CertificateStore> = {},
  rubrics: RubricPicker = emptyRubricPicker(),
  signer: SigningBackend = unavailableSigningBackend(),
  invoked: Drop | null = null,
  drops: FakeDocumentDrops = inMemoryDocumentDrops(invoked),
  versions: VersionCheck = inMemoryVersionCheck(),
  externalDestinations: ExternalDestinationOpener = unavailableExternalDestinationOpener(),
  status?: StatusPort,
) {
  const preferences = inMemoryPreferences(
    {
      theme: "system",
      destination: "Documentos",
      offersOriginalFolder: false,
      rememberVisibleSignature: true,
      rememberActivity: true,
      notifyNewVersion: true,
      setupWizardSeen: false,
      consentCountdown: true,
      honourAutomaticSelection: false,
      ...settings,
    },
    () => void recents.clear(),
  );
  renderWithCatalog(
    <App
      recents={recents}
      picker={inMemoryDocumentPicker(documents)}
      drops={drops}
      pdfs={pdfs}
      preferences={preferences}
      destinations={aDestination()}
      certificates={{ ...emptyCertificateStore(), ...certificates }}
      rubrics={rubrics}
      stamps={unavailableStampComposer()}
      signer={signer}
      opener={unavailableOpener()}
      versions={versions}
      menuAnchor="header"
      externalDestinations={externalDestinations}
      status={status}
    />,
  );
  return { recents, preferences, drops };
}

/** Abre un PDF por el menú «+», que es el camino que existe con y sin documentos abiertos. */
export async function openPdf(user: UserEvent) {
  await user.click(screen.getByRole("button", { name: "Abrir un PDF" }));
  await user.click(screen.getByRole("menuitem", { name: "Abrir un PDF…" }));
}
