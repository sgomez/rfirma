import { GlobalWorkerOptions, getDocument, type PageViewport, type PDFPageProxy } from "pdfjs-dist";
// El fichero del worker, empaquetado por Vite. Con `?url` sale una ruta a un
// activo de `dist/`, que es lo que hay que darle a `pdf.js`: bajo el arenero no
// se puede cargar un worker de fuera de la aplicación.
import workerSource from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import type { PdfDocument, PdfLoader, PdfPage, Viewport } from "./pdf";

GlobalWorkerOptions.workerSrc = workerSource;

/**
 * Dónde están las catorce fuentes estándar, que las empaqueta `vite.config.ts`
 * (ID-112).
 *
 * Sin esto `pdf.js` avisa y sustituye por una fuente del sistema: pinta igual
 * —está medido—, pero con otras métricas, y el corte de línea del texto que se
 * ve dentro del recuadro dejaría de ser el que produce el compositor. La ruta
 * es la misma en la ventana y en `vite dev`; **la vigila una guardia**, en
 * `pdfjsLoader.test.ts`.
 */
const STANDARD_FONTS = "/standard_fonts/";

/**
 * `pdf.js` de verdad, detrás del puerto de [`pdf.ts`](./pdf.ts).
 *
 * Es el **único** fichero del frontal que importa `pdfjs-dist`, y no tiene ni
 * una decisión propia: cada método es una línea. Todo lo que hay que pensar
 * —cancelar la pintada en vuelo, guardar el recuadro en espacio de usuario—
 * vive en los módulos que sí se prueban en `jsdom`, donde esta librería no
 * cabe.
 */
export function pdfjsLoader(): PdfLoader {
  return {
    async load(bytes) {
      const document = await getDocument({ data: bytes, standardFontDataUrl: STANDARD_FONTS })
        .promise;
      return {
        pageCount: document.numPages,
        getPage: async (number) => adaptPage(await document.getPage(number)),
      } satisfies PdfDocument;
    },
  };
}

function adaptPage(page: PDFPageProxy): PdfPage {
  return {
    number: page.pageNumber,
    rotate: page.rotate,
    // `pdf.js` tipa `view` como `number[]`, así que las cuatro esquinas salen
    // opcionales aunque siempre estén. Un cero por omisión sería una caja
    // inventada; el `?? 0` está para que `tsc` pase, y el caso no ocurre.
    view: [page.view[0] ?? 0, page.view[1] ?? 0, page.view[2] ?? 0, page.view[3] ?? 0],
    getViewport: ({ scale }) => adaptViewport(page, scale),
    // `render` exige el `PageViewport` original, no una copia con los mismos
    // números, así que el nuestro guarda una referencia al suyo.
    render: ({ canvas, viewport }) => page.render({ canvas, viewport: originalOf(page, viewport) }),
  };
}

/** El `PageViewport` del que salió cada viewport nuestro. Ver [`adaptPage`]. */
const originals = new WeakMap<Viewport, PageViewport>();

function adaptViewport(page: PDFPageProxy, scale: number): Viewport {
  const viewport = page.getViewport({ scale });
  const adapted: Viewport = {
    width: viewport.width,
    height: viewport.height,
    convertToPdfPoint: (x, y) => pair(viewport.convertToPdfPoint(x, y)),
    convertToViewportPoint: (x, y) => pair(viewport.convertToViewportPoint(x, y)),
  };
  originals.set(adapted, viewport);
  return adapted;
}

function originalOf(page: PDFPageProxy, viewport: Viewport): PageViewport {
  return originals.get(viewport) ?? page.getViewport({ scale: 1 });
}

/** `convertTo*Point` devuelve `any[]`; aquí se estrecha a las dos coordenadas. */
function pair(values: unknown[]): [number, number] {
  return [Number(values[0]), Number(values[1])];
}
