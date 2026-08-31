/**
 * Los tres puertos de firma, enchufados a las órdenes de Tauri (#60).
 *
 * Este es el **único** fichero del frontal que sabe que debajo hay Tauri, y por
 * eso es el único que importa `invoke`. La ventana y sus pruebas siguen
 * hablando con `CertificateStore`, `Layer2Composer` y `SigningBackend`, y quien
 * elige entre estas implementaciones y las de memoria es `main.tsx`.
 *
 * # Los fallos llegan clasificados, no traducidos
 *
 * Las órdenes rechazan con la forma del ID-29 —una situación nuestra y el texto
 * original crudo al lado—, así que aquí no hay ni una tabla de `CKR_*` ni un
 * `catch` que invente un mensaje: lo que no venga con esa forma —una excepción
 * del propio puente de Tauri, una orden que no existe— cae en `unknown` con su
 * texto tal cual, que es exactamente lo que el ADR-0009 pide.
 */

import { invoke } from "@tauri-apps/api/core";
import type { Certificate, CertificateStore } from "./certificate";
import type { SignedDocument, SigningBackend, SigningOrder, StageResult } from "./flow";
import type { TokenFailure } from "./token";
import type { Layer2Composer, SigningIdentity, VisibleSignature } from "./visibleSignature";

/**
 * Un fallo tal y como lo rechaza una orden. Es `commands::Failure` de Rust,
 * campo a campo.
 */
interface RejectedFailure {
  situation: string;
  detail: string;
  attemptsLeft: number | null;
}

function isRejectedFailure(thrown: unknown): thrown is RejectedFailure {
  return (
    typeof thrown === "object" &&
    thrown !== null &&
    typeof (thrown as RejectedFailure).situation === "string" &&
    typeof (thrown as RejectedFailure).detail === "string"
  );
}

/**
 * Lo que se enseña cuando algo falla.
 *
 * Un rechazo con forma se pasa tal cual —la situación ya viene clasificada por
 * Rust—; cualquier otra cosa cae en `unknown` **conservando su texto**. Perder
 * ese texto sería quedarse sin lo único que sirve para diagnosticar el fallo
 * que no supimos clasificar.
 */
function failureOf(thrown: unknown): TokenFailure {
  if (isRejectedFailure(thrown)) {
    return {
      situation: thrown.situation as TokenFailure["situation"],
      detail: thrown.detail,
      attemptsLeft: thrown.attemptsLeft,
    };
  }
  return {
    situation: "unknown",
    detail: thrown instanceof Error ? thrown.message : String(thrown),
    attemptsLeft: null,
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
  };
}

/** Olvida el ciclo a medias cuando se cancela en el diálogo del PIN. */
export function cancelSigning(): Promise<void> {
  return invoke<void>("cancel_signing");
}
