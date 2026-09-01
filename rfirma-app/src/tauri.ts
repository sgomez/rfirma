/**
 * Los cinco puertos que hablan con las órdenes de Tauri: los tres de firma
 * (#60) y los dos del documento (#82).
 *
 * Este es el **único** fichero del frontal que sabe que debajo hay Tauri, y por
 * eso es el único que importa `invoke`. Vive en la raíz de `src/` y no dentro
 * de `signing/` justamente por eso: la frontera es una sola para toda la
 * aplicación, y repartirla por módulos sería tener dos. La ventana y sus
 * pruebas siguen hablando con `CertificateStore`, `Layer2Composer`,
 * `SigningBackend`, `DocumentPicker` y `PdfSource`, y quien elige entre estas
 * implementaciones y los dobles de memoria es `main.tsx`.
 *
 * # Los fallos llegan clasificados, no traducidos
 *
 * Las órdenes rechazan con la forma del ID-29 —una situación nuestra y el texto
 * original crudo al lado—, así que aquí no hay ni una tabla de `CKR_*` ni un
 * `catch` que invente un mensaje: lo que no venga con esa forma —una excepción
 * del propio puente de Tauri, una orden que no existe— cae en `unknown` con su
 * texto tal cual, que es exactamente lo que el ADR-0009 pide. Quien lo decide
 * es `errors/classify.ts`, que no es de Tauri sino del ID-29.
 */

import { invoke } from "@tauri-apps/api/core";
import type { DocumentPicker } from "./documents/picker";
import type { RecentDocument } from "./documents/recents";
import { classify } from "./errors/classify";
import type { Certificate, CertificateStore } from "./signing/certificate";
import type { SignedDocument, SigningBackend, SigningOrder, StageResult } from "./signing/flow";
import type { TokenFailure } from "./signing/token";
import type { Layer2Composer, SigningIdentity, VisibleSignature } from "./signing/visibleSignature";
import { type PdfSource, pdfjsSource } from "./viewer/source";

/** Lo que se enseña cuando falla una etapa de la firma. Ver [`classify`]. */
function failureOf(thrown: unknown): TokenFailure {
  const named = classify(thrown);
  return {
    situation: named.situation as TokenFailure["situation"],
    detail: named.detail,
    attemptsLeft: named.attemptsLeft,
  };
}

/**
 * Envuelve una etapa: sale bien, o sale con una situación clasificada.
 *
 * Recibe **la llamada sin hacer** y no la promesa ya hecha, para que la orden
 * se invoque **dentro** del `try`. Con la promesa por parámetro, un fallo
 * síncrono de `invoke` —una orden que no existe— se escaparía de este `catch`,
 * y el rechazo quedaría suelto entre que se crea y que se espera.
 */
async function stage<T>(call: () => Promise<T>): Promise<StageResult<T>> {
  try {
    return { ok: true, value: await call() };
  } catch (thrown) {
    return { ok: false, failure: failureOf(thrown) };
  }
}

/** Los certificados de los tokens conectados. No pide el PIN. */
export function tauriCertificateStore(): CertificateStore {
  return {
    list: () => invoke<readonly Certificate[]>("list_certificates"),
  };
}

/**
 * El compositor autoritativo: el mismo `signing::layer2_text` que compone lo
 * que se envía en `layer2Text`.
 *
 * Por eso la vista previa es honesta y no una imitación: es literalmente la
 * cadena que va a acabar estampada.
 */
export function tauriLayer2Composer(): Layer2Composer {
  return {
    compose: async (signature: VisibleSignature, signer: SigningIdentity) => {
      try {
        return await invoke<string>("compose_visible_text", {
          order: previewOrder(signature, signer),
        });
      } catch {
        // La vista previa no es sitio para un aviso de error: si no se puede
        // componer —el token se ha retirado mientras se miraba—, el recuadro
        // se queda en su estado vacío y lo contará el intento de firmar.
        return null;
      }
    },
  };
}

/**
 * La orden que compone la vista previa.
 *
 * Lleva un recuadro degenerado y ningún documento **a propósito**:
 * `compose_visible_text` solo mira las casillas, el motivo y el instante, y
 * darle una posición de mentira es más honesto que darle una de verdad que
 * nadie va a usar.
 */
function previewOrder(signature: VisibleSignature, signer: SigningIdentity): SigningOrder {
  return {
    document: "",
    certificate: signer.certificate,
    placement: { page: 1, mediaBox: [0, 0, 0, 0], rotation: 0, rect: [0, 0, 0, 0] },
    fields: signature.fields,
    reason: signature.reason,
    signedAt: signer.signedAt,
    rubric: null,
    language: signer.language,
  };
}

/**
 * Las tres etapas, cada una en su orden.
 *
 * El ciclo a medias **no vive aquí**: entre la prefirma y la postfirma lo
 * guarda el backend, que es quien tiene el sello de sesión. Este objeto no
 * tiene estado, y eso es lo que impide que la ventana pueda alterar el sello
 * que la postfirma exige idéntico (ADR-0016).
 */
export function tauriSigningBackend(): SigningBackend {
  return {
    presign: (order) => stage(() => invoke<void>("begin_signing", { order })),
    sign: (pin) => stage(() => invoke<void>("sign_with_pin", { pin })),
    postsign: () => stage(() => invoke<SignedDocument>("finish_signing")),
    discard: cancelSigning,
  };
}

/**
 * Olvida el ciclo a medias cuando se cancela en el diálogo del PIN.
 *
 * Quien la llama es `useSigning.cancel`, a través del puerto: por eso está
 * enchufada como `discard` arriba y no exportada suelta para que alguien se
 * acuerde de invocarla.
 */
export function cancelSigning(): Promise<void> {
  return invoke<void>("cancel_signing");
}

/**
 * Un documento recién abierto, tal como lo devuelve `open_document`. Es
 * `commands::OpenedDocumentView` de Rust, campo a campo: **un identificador y
 * un nombre, ninguna ruta** (ADR-0011).
 */
interface OpenedDocumentView {
  id: string;
  name: string;
  modified: number | null;
}

/**
 * El portal de ficheros, por la orden que abre el diálogo desde Rust (ID-63).
 *
 * El diálogo no se abre desde aquí: si lo hiciera, el frontal tendría que pedir
 * el permiso del complemento de diálogo y la lista de permisos de la ventana
 * crecería. Lo que cruza es lo que el backend apuntó.
 *
 * Cancelar devuelve `null`, y eso **no es un fallo**: es lo que deja el
 * documento activo, la lista y el visor como estaban (ID-73).
 */
export function tauriDocumentPicker(): DocumentPicker {
  return {
    choose: async () => {
      const opened = await invoke<OpenedDocumentView | null>("open_document");
      if (opened === null) return null;
      return {
        id: opened.id,
        name: opened.name,
        // Un documento recién abierto se anota como **no firmado** (ID-71):
        // saber si un PDF ya trae firmas es otro trabajo, y el panel ya declara
        // ese dato como desconocido. Se anota lo que se sabe.
        badge: "Unsigned",
        modified: opened.modified,
        lastUsed: Math.floor(Date.now() / 1000),
        // Lo acaba de conceder el portal, así que responde.
        available: true,
      } satisfies RecentDocument;
    },
  };
}

/**
 * El PDF que se pinta: los bytes del portal, abiertos con `pdf.js` (ID-76).
 *
 * Los bytes viajan **como bytes** y no como una lista de números en JSON
 * (ID-66): `read_document` contesta con la respuesta binaria del puente de
 * Tauri, que aquí llega como un `ArrayBuffer`.
 */
export function tauriPdfSource(): PdfSource {
  return pdfjsSource(async (document) => {
    const bytes = await invoke<ArrayBuffer>("read_document", { id: document.id });
    return new Uint8Array(bytes);
  });
}
