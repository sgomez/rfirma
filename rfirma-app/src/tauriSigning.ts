/** Los puertos de Tauri de la firma: certificados, las tres etapas, la rúbrica y el sello (#60, #128, #194). */

import { invoke } from "@tauri-apps/api/core";
import { classify } from "./errors/classify";
import type { Certificate, CertificateStore } from "./signing/certificate";
import type { SignedDocument, SigningBackend } from "./signing/flow";
import type { Rubric, RubricPicker, RubricSituation } from "./signing/rubric";
import type { StoreSecret } from "./signing/secret";
import type { StampComposer } from "./signing/stampPreview";
import type {
  RememberedVisibleSignature,
  VisibleSignatureMemory,
} from "./signing/visibleSignature";
import { stage } from "./tauriStage";
import { pdfjsLoader } from "./viewer/pdfjsLoader";

/**
 * Los certificados de los tokens conectados, y los dos gestos de Preferencias
 * sobre los `.p12` instalados. Listar no pide el PIN.
 */
export function tauriCertificateStore(): CertificateStore {
  return {
    list: () => invoke<readonly Certificate[]>("list_certificates"),
    install: (password) => invoke<boolean>("install_certificate", { password }),
    remove: (id) => invoke<void>("remove_certificate", { id }),
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
    presign: (order) => stage(() => invoke<StoreSecret>("begin_signing", { order })),
    sign: (pin) => stage(() => invoke<void>("sign_with_pin", { pin })),
    postsign: (singleDestinationId = null) =>
      stage(() => invoke<SignedDocument>("finish_signing", { destination: singleDestinationId })),
    padesLowerLeft: (placement) => invoke<[number, number]>("pades_lower_left", { placement }),
    unregisteredSignatures: (document) => invoke<boolean>("unregistered_signatures", { document }),
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
function cancelSigning(): Promise<void> {
  return invoke<void>("cancel_signing");
}

/**
 * La rúbrica ya normalizada, tal como la devuelve `choose_rubric`. Es
 * `commands::RubricView` de Rust, campo a campo: el JPEG en Base64, sin el
 * prefijo `data:` —lo antepone aquí, que es quien sabe que es para un
 * `<img>`— y sus dimensiones.
 */
interface RubricViewPayload {
  base64: string;
  width: number;
  height: number;
}

/**
 * Lo que devuelve elegir una rúbrica, tal cual lo emite Rust. Es
 * `commands::RubricChoiceView`, campo a campo: la imagen adoptada o por qué
 * no se ha podido, nunca las dos.
 */
interface RubricChoiceViewPayload {
  rubric: RubricViewPayload | null;
  failure: { situation: string; detail: string } | null;
}

/**
 * El selector de la rúbrica, por la orden que abre el diálogo del portal
 * desde Rust y adopta lo elegido en `RubricStore` (ID-82).
 *
 * Cancelar el diálogo devuelve `null`, y **no es un fallo**: es lo que deja
 * la rúbrica ya elegida como estaba. Una imagen que no vale tampoco revienta
 * la promesa —viaja como `{ failure }`, con el panel de firma todavía
 * abierto (ADR-0010)—, así que `choose` no necesita `try`/`catch`: las seis
 * situaciones de `RubricSituation` llegan ya clasificadas en la propia
 * respuesta.
 */
function rubricOf(payload: RubricViewPayload): Rubric {
  const { base64, width, height } = payload;
  return { dataUrl: `data:image/jpeg;base64,${base64}`, width, height };
}

export function tauriRubricPicker(): RubricPicker {
  return {
    choose: async () => {
      const outcome = await invoke<RubricChoiceViewPayload | null>("choose_rubric");
      if (outcome === null) return null;
      if (outcome.rubric !== null) return { rubric: rubricOf(outcome.rubric) };
      const failure = outcome.failure;
      if (failure === null) return null;
      return {
        failure: {
          situation: failure.situation as RubricSituation,
          detail: failure.detail,
        },
      };
    },
    stored: async () => {
      const found = await invoke<RubricViewPayload | null>("read_rubric");
      return found === null ? null : rubricOf(found);
    },
  };
}

/** Modelo, frase y «Con rúbrica» de la última firma visible configurada: `remembered_visible_signature`. */
export function tauriVisibleSignatureMemory(): VisibleSignatureMemory {
  return {
    read: () => invoke<RememberedVisibleSignature>("remembered_visible_signature"),
  };
}

/**
 * El sello de verdad, antes de firmar: `preview_signature` (ID-107).
 *
 * Debajo hay un **ciclo trifásico en seco** con un `PK1` inventado, que
 * devuelve un PDF cuyos bytes visibles están medidos idénticos a los del
 * firmado de verdad. No pide PIN —el certificado elegido es público y se lee
 * sin él— y lo que devuelve **se tira**: firmar de verdad vuelve a prefirmar
 * desde cero.
 *
 * Los bytes viajan como bytes, igual que en `read_document`, y se abren con el
 * mismo `pdf.js` que pinta el original: por eso la vista previa no es una
 * imitación de nada, es el mismo compositor y el mismo lector.
 */
export function tauriStampComposer(): StampComposer {
  const loader = pdfjsLoader();
  return {
    compose: async (order) => {
      try {
        const bytes = await invoke<ArrayBuffer>("preview_signature", { order });
        return { ok: true, pdf: await loader.load(new Uint8Array(bytes)) };
      } catch (thrown) {
        // La vista previa **no es una puerta** (ID-111): el fallo se cuenta y
        // se sigue pudiendo firmar, así que aquí no se relanza nada.
        const named = classify(thrown);
        return {
          ok: false,
          failure: {
            situation: named.situation === "unknown" ? "documentUnreadable" : named.situation,
            detail: named.detail,
          },
        };
      }
    },
  };
}
