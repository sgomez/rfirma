import type { RecentDocument } from "../documents/recents";
import { classify } from "../errors/classify";
import type { ErrorSituation } from "../errors/ErrorNotice";
import type { PdfDocument } from "./pdf";
import { pdfjsLoader } from "./pdfjsLoader";

/**
 * Por qué no se ha podido pintar un documento, con la forma del ID-29.
 *
 * No lleva `attemptsLeft`: eso es del token, y aquí no hay ninguno.
 */
export interface DocumentFailure {
  situation: ErrorSituation;
  /** El texto original, sin traducir ni recortar. Nunca vacío. */
  detail: string;
}

/**
 * Lo que sale de abrir un documento: el PDF, o un fallo **con nombre**.
 *
 * No es `PdfDocument | null` a propósito. Un `null` dejaba el visor en su
 * estado vacío —el mismo que cuando no se ha abierto nada— y quien acababa de
 * elegir un PDF corrupto se quedaba mirando la zona de soltar sin que nadie le
 * dijera qué había pasado.
 */
export type OpenedPdf =
  /**
   * El PDF abierto y **cuánto ocupa**, en bytes.
   *
   * El tamaño viaja con él porque sale de los mismos bytes que se acaban de
   * leer y no hay una segunda forma de saberlo: bajo el sandbox la aplicación
   * no conoce la ruta del documento, así que nadie puede preguntarle al disco.
   * Quien lo usa es la vista previa del sello, que por encima de cierto tamaño
   * deja de recalcularse sola (ID-109).
   */
  { ok: true; pdf: PdfDocument; sizeBytes: number } | { ok: false; failure: DocumentFailure };

/**
 * De qué documento se pinta el PDF.
 *
 * Es un puerto por lo mismo que lo es el selector: bajo el sandbox los bytes
 * los entrega el **portal**, no una ruta que el WebView pueda abrir. La
 * aplicación nunca conoce la ruta original de un documento, así que aquí no hay
 * ni una URL: entra el documento de la bandeja y sale el PDF ya abierto.
 */
export interface PdfSource {
  /** El documento abierto, o el fallo que lo impidió. */
  open(document: RecentDocument): Promise<OpenedPdf>;
}

/** Lee los bytes de un documento. Lo aporta el backend, por el portal. */
export type ReadDocument = (document: RecentDocument) => Promise<Uint8Array>;

/**
 * El origen de verdad: los bytes del portal, abiertos con `pdf.js`.
 *
 * Las dos mitades fallan por sitios distintos —la orden, porque el documento ya
 * no está concedido; `pdf.js`, porque lo que llegó no es un PDF que sepa
 * abrir— y las dos acaban contadas igual: una situación nuestra y el texto
 * original crudo al lado.
 */
export function pdfjsSource(read: ReadDocument): PdfSource {
  const loader = pdfjsLoader();
  return {
    open: async (document) => {
      try {
        const bytes = await read(document);
        return { ok: true, pdf: await loader.load(bytes), sizeBytes: bytes.byteLength };
      } catch (thrown) {
        const named = classify(thrown);
        return {
          ok: false,
          failure: {
            // Lo que `pdf.js` rechaza no viene clasificado, y llegar aquí
            // significa que los bytes ya se leyeron: el documento está, pero no
            // se puede abrir.
            situation: named.situation === "unknown" ? "documentUnreadable" : named.situation,
            detail: named.detail,
          },
        };
      }
    },
  };
}

/**
 * Un origen que **no abre nada**, y lo dice.
 *
 * Desde el #82 quien pinta de verdad es `tauriPdfSource`, así que esto ya no es
 * el relleno de `main.tsx` sino un doble: sirve para montar la ventana en una
 * prueba sin backend. Falla diciendo la verdad en vez de dejar el visor vacío,
 * que era indistinguible de no haber abierto nada.
 */
export function unavailablePdfSource(): PdfSource {
  return {
    open: async () => ({
      ok: false,
      failure: {
        situation: "documentUnreadable",
        detail: "esta composicion no tiene origen de PDF",
      },
    }),
  };
}
