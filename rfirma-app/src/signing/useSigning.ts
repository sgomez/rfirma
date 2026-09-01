import { useState } from "react";
import type { Certificate } from "./certificate";
import { refusalFor, type SigningFailure } from "./failure";
import type { SignedDocument, SigningBackend, SigningOrder, SigningStage } from "./flow";
import { belongsToPinDialog, type TokenFailure } from "./token";

/**
 * En qué punto del recorrido de firma está la ventana.
 *
 * `pin` con un fallo dentro es la clave del reintento: un PIN incorrecto vuelve
 * a este mismo estado, **sin repetir la prefirma** y sin desmontar el diálogo.
 */
export type SigningState =
  | { kind: "idle" }
  | { kind: "running"; stage: SigningStage }
  | { kind: "pin"; failure: TokenFailure | null }
  | { kind: "signed"; document: SignedDocument }
  | { kind: "failed"; failure: SigningFailure };

/** Lo que la ventana necesita para conducir la firma. */
export interface Signing {
  state: SigningState;
  /**
   * Arranca por la prefirma con la orden completa. Antes comprueba el
   * certificado: uno caducado o revocado se avisa **sin** llegar a pedir el
   * PIN.
   *
   * La orden va entera en esta llamada y no se guarda aquí: entre la prefirma
   * y la postfirma el ciclo vive en el backend, con su sello de sesión, y la
   * ventana no tiene nada que pueda alterar (ADR-0016).
   */
  start: (certificate: Certificate | null, order: SigningOrder) => Promise<void>;
  /** El PIN tecleado: firma en la tarjeta y ensambla. */
  submitPin: (pin: string) => Promise<void>;
  /**
   * Cancelar en el diálogo del PIN, o cerrar un fallo: se vuelve al panel **y
   * el backend olvida el ciclo a medias**.
   *
   * Las dos cosas, no solo la primera: volver al panel sin avisar al backend
   * dejaba el `OpenCycle` entero vivo en memoria —el PDF, los atributos a
   * firmar, el sello y el PKCS#1— hasta que se cerrara la ventana o hasta que
   * otra firma lo pisara.
   */
  cancel: () => void;
}

/**
 * El recorrido de firma, etapa a etapa.
 *
 * El orden no es negociable y es el del ADR: **prefirma → PIN → firma →
 * postfirma**. Pedir el PIN antes de la prefirma sería pedir el secreto que
 * desbloquea la clave sin saber todavía qué se va a firmar.
 *
 * Dónde se cuenta cada fallo lo decide `belongsToPinDialog`, y no este bucle:
 * el PIN incorrecto y la tarjeta bloqueada son respuestas a lo que el usuario
 * acaba de teclear, y se contestan en el diálogo; el resto sale al pie del
 * panel, que es el estado «error de firma» de la ficha.
 *
 * Quien implementa [`SigningBackend`] de verdad son las órdenes de Tauri del
 * #60; aquí solo se pide cada etapa por su turno.
 */
export function useSigning(backend: SigningBackend): Signing {
  const [state, setState] = useState<SigningState>({ kind: "idle" });

  const routed = (failure: TokenFailure): SigningState =>
    belongsToPinDialog(failure) ? { kind: "pin", failure } : { kind: "failed", failure };

  const start = async (certificate: Certificate | null, order: SigningOrder) => {
    // El estado del certificado se sabe leyendo su DER, sin tocar la tarjeta:
    // pedir el PIN para luego fallar por una fecha ya conocida es hacer teclear
    // el secreto que desbloquea la clave para nada.
    const refusal = refusalFor(certificate);
    if (refusal) {
      setState({ kind: "failed", failure: refusal });
      return;
    }
    setState({ kind: "running", stage: "presign" });
    const presigned = await backend.presign(order);
    setState(presigned.ok ? { kind: "pin", failure: null } : routed(presigned.failure));
  };

  const submitPin = async (pin: string) => {
    setState({ kind: "running", stage: "sign" });
    const signed = await backend.sign(pin);
    if (!signed.ok) {
      setState(routed(signed.failure));
      return;
    }
    setState({ kind: "running", stage: "postsign" });
    const assembled = await backend.postsign();
    setState(
      assembled.ok ? { kind: "signed", document: assembled.value } : routed(assembled.failure),
    );
  };

  const cancel = () => {
    setState({ kind: "idle" });
    // De cortesía y sin esperar: la ventana ya está en el panel, y si el
    // backend no puede olvidar el ciclo no hay nada que contarle a nadie. El
    // `catch` está porque una promesa rechazada y sin dueño tumba el proceso
    // de pruebas, no porque haya un fallo que tragarse.
    void backend.discard().catch(() => {});
  };

  return { state, start, submitPin, cancel };
}
