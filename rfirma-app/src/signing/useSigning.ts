import { useState } from "react";
import type { SignedDocument, SigningBackend, SigningStage } from "./flow";
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
  | { kind: "failed"; failure: TokenFailure };

/** Lo que la ventana necesita para conducir la firma. */
export interface Signing {
  state: SigningState;
  /** Arranca por la prefirma. El PIN se pide después, nunca antes. */
  start: () => Promise<void>;
  /** El PIN tecleado: firma en la tarjeta y ensambla. */
  submitPin: (pin: string) => Promise<void>;
  /** Cancelar en el diálogo del PIN, o cerrar un fallo: se vuelve al panel. */
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

  const start = async () => {
    setState({ kind: "running", stage: "presign" });
    const presigned = await backend.presign();
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

  const cancel = () => setState({ kind: "idle" });

  return { state, start, submitPin, cancel };
}
