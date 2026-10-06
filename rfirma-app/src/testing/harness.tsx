//! Los dobles y el `renderApp` que comparten las pruebas de `App`.

import { screen } from "@testing-library/react";
import type { UserEvent } from "@testing-library/user-event";
import { App } from "../App";
import type { DocumentInHand } from "../documents/document";
import type { Drop } from "../documents/drops";
import { inMemoryDocumentDrops } from "../documents/drops";
import { inMemoryDocumentPicker } from "../documents/picker";
import { inMemoryRecents, type RecentDocument } from "../documents/recents";
import type { Preferences } from "../preferences/preferences";
import { inMemoryPreferences } from "../preferences/preferences";
import { defaults } from "../preferences/testing/fixtures";
import type { NativeTitlebar } from "../shell/nativeTitlebar";
import type { Certificate, CertificateStore } from "../signing/certificate";
import { emptyCertificateStore } from "../signing/certificate";
import {
  type Destination,
  type DestinationSource,
  inMemoryDestination,
  type SingleDestination,
} from "../signing/destination";
import { DEFAULT_VISIBLE_SIGNATURE, type VisibleSignature } from "../signing/visibleSignature";
import type { PdfDocument, PdfPage, Viewport } from "../viewer/pdf";
import type { PdfSource } from "../viewer/source";
import { aMainWindowDoubles, type MainWindowDoubleOverrides } from "./mainWindowDoubles";
import { renderWithCatalog } from "./render";

/** El destino que contesta el backend mientras la prueba no diga otra cosa. */
export const aDestination = () =>
  inMemoryDestination({ folder: "Documentos", name: "contrato-firmado.pdf", writable: true });

/**
 * Un destino que, al «Cambiar», ofrece `single` para esta firma —y solo para
 * ella—: la vista previa vuelve a `single` en cuanto se pide con su id, y se
 * queda en `initial` para cualquier otro (ADR-0011).
 */
export function destinationOfferingSingleChoice(
  initial: Destination,
  single: SingleDestination,
): DestinationSource {
  return {
    previewFor: async (_documentId, singleDestinationId) =>
      singleDestinationId === single.id ? single : initial,
    chooseSingle: async () => single,
  };
}

/**
 * **El documento que se tiene delante**: lo que entra por el diálogo o por el
 * arrastre, y lo que se pinta y se firma. No es la fila.
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
    folder: null,
    location: null,
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
 * y para probarlo hace falta más de un tamaño en el mismo documento.
 */
export function aPdfWithViews(
  views: readonly (readonly [number, number, number, number])[],
): PdfDocument {
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

export const aCertificate: Certificate = {
  id: "0123456789abcdef0123456789abcdef",
  label: "Firma",
  holderName: "Ada Lovelace Byron",
  stampedSigner: "Ada Lovelace Byron",
  givenName: "Ada",
  surname: "Lovelace Byron",
  idNumber: "99999999R",
  organizationIdentifier: null,
  entityName: null,
  issuer: "AC FNMT Usuarios",
  certificateSerialNumber: "1234567890",
  stores: ["card"],
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

interface RenderAppOptions extends Omit<MainWindowDoubleOverrides, "titlebar"> {
  documents?: DocumentInHand[];
  settings?: Partial<Preferences>;
  invoked?: Drop | null;
  initialSignature?: VisibleSignature;
  titlebar?: NativeTitlebar | null;
}

export function renderApp({
  documents = [],
  settings = {},
  invoked = null,
  initialSignature = DEFAULT_VISIBLE_SIGNATURE,
  titlebar = null,
  ...overrides
}: RenderAppOptions = {}) {
  const recents = overrides.recents ?? inMemoryRecents();
  const preferences = inMemoryPreferences({ ...defaults, ...settings }, () => void recents.clear());
  const ports = aMainWindowDoubles({
    picker: inMemoryDocumentPicker(documents),
    drops: inMemoryDocumentDrops(invoked),
    destinations: aDestination(),
    ...overrides,
    recents,
    preferences,
    ...(titlebar === null ? {} : { titlebar }),
  });
  renderWithCatalog(
    <App
      ports={ports}
      initialSignature={initialSignature}
      version="0.1.0"
      menuAnchor={titlebar === null ? "header" : "titlebar"}
    />,
  );
  return { recents, preferences, drops: ports.drops };
}

/** Abre un PDF por el segmento principal del botón partido. */
export async function openPdf(user: UserEvent) {
  await user.click(screen.getByRole("button", { name: "Abrir PDF…" }));
}
