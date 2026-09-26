import { useState } from "react";
import type { Certificate } from "./certificate";
import { refusalFor, type SigningFailure } from "./failure";
import type { SignedDocument, SigningBackend, SigningOrder, SigningStage } from "./flow";

/**
 * En qué punto del recorrido de firma está la ventana.
 *
 * `signed` y `failed` llevan dentro el **identificador del documento de
 * partida** (`origin`), y no solo lo que produjo el ciclo: el acuse de recibo y
 * el error son de un documento concreto, y sin esa atadura la ventana los
 * enseñaba al lado de cualquier otro que se abriera después —el nombre de A
 * con el recuento de páginas de B, o el error de A sobre la pestaña de B—.
 */
export type SigningState =
  | { kind: "idle" }
  | { kind: "running"; stage: SigningStage }
  | { kind: "signed"; document: SignedDocument; origin: string }
  | { kind: "failed"; failure: SigningFailure; origin: string };

/** Lo que la ventana necesita para conducir la firma. */
export interface Signing {
  state: SigningState;
  /**
   * Arranca por la prefirma con la orden completa. Antes comprueba el
   * certificado: uno caducado o revocado se avisa **sin** llegar a pedir el
   * secreto al almacén.
   *
   * La orden va entera en esta llamada y no se guarda aquí: entre la prefirma
   * y la postfirma el ciclo vive en el backend, con su sello de sesión, y la
   * ventana no tiene nada que pueda alterar (ADR-0016).
   *
   * `singleDestinationId` es el destino elegido para esta firma con el
   * diálogo de guardar (ADR-0011); sin él cae en la preferencia de carpeta.
   */
  start: (
    certificate: Certificate | null,
    order: SigningOrder,
    singleDestinationId?: string | null,
  ) => Promise<void>;
  /**
   * Cancelar, o cerrar un fallo: se vuelve al panel **y
   * el backend olvida el ciclo a medias**.
   *
   * Las dos cosas, no solo la primera: volver al panel sin avisar al backend
   * dejaba el `OpenCycle` entero vivo en memoria —el PDF, los atributos a
   * firmar, el sello y el PKCS#1— hasta que se cerrara la ventana o hasta que
   * otra firma lo pisara.
   */
  cancel: () => void;
  /**
   * Cerrar el estado «Firmado» para empezar otra firma.
   *
   * No avisa al backend, a diferencia de [`cancel`]: el ciclo ya terminó por
   * su propio pie en la postfirma y no queda nada a medias que olvidar.
   */
  signAnother: () => void;
}

/**
 * El recorrido de firma, etapa a etapa.
 *
 * El orden no es negociable y es el del ADR: **prefirma → firma → postfirma**.
 * El diálogo modal del secreto, de ser necesario, lo gestiona de forma nativa
 * el sistema operativo en el backend.
 *
 * Quien implementa [`SigningBackend`] de verdad son las órdenes de Tauri del
 * #60; aquí solo se pide cada etapa por su turno.
 */
export function useSigning(backend: SigningBackend): Signing {
  const [state, setState] = useState<SigningState>({ kind: "idle" });

  const start = async (
    certificate: Certificate | null,
    order: SigningOrder,
    singleDestinationId: string | null = null,
  ) => {
    // El estado del certificado se sabe leyendo su DER, sin tocar la tarjeta:
    // fallar por una fecha ya conocida evita iniciar el ciclo innecesariamente.
    const refusal = refusalFor(certificate);
    if (refusal) {
      setState({ kind: "failed", failure: refusal, origin: order.document });
      return;
    }
    setState({ kind: "running", stage: "presign" });
    const presigned = await backend.presign(order);
    if (!presigned.ok) {
      setState({ kind: "failed", failure: presigned.failure, origin: order.document });
      return;
    }

    setState({ kind: "running", stage: "sign" });
    const signed = await backend.sign("");
    if (!signed.ok) {
      setState({ kind: "failed", failure: signed.failure, origin: order.document });
      return;
    }

    setState({ kind: "running", stage: "postsign" });
    const assembled = await backend.postsign(singleDestinationId);
    setState(
      assembled.ok
        ? { kind: "signed", document: assembled.value, origin: order.document }
        : { kind: "failed", failure: assembled.failure, origin: order.document },
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

  const signAnother = () => setState({ kind: "idle" });

  return { state, start, cancel, signAnother };
}

/**
 * El acuse de recibo, **solo si sigue delante el documento que lo produjo**.
 *
 * El estado «Firmado» enseña el fichero que quedó escrito, pero el recuento de
 * páginas sale del PDF que la ventana tiene abierto: son dos fuentes, y solo
 * dicen lo mismo mientras hablen del mismo documento. Con otro delante el panel
 * enseñaba el nombre de A con las páginas de B —un dato inventado, que es justo
 * lo que el ID-44 prohíbe—, y sin ninguno se quedaba una tercera columna al
 * lado del visor vacío, que es lo que quita el ID-51.
 *
 * `activeId` es `null` cuando no hay documento activo: se ha olvidado el que
 * había, o se ha vaciado la lista.
 */
export function acknowledgementFor(
  state: SigningState,
  activeId: string | null,
): Extract<SigningState, { kind: "signed" }> | null {
  if (state.kind !== "signed") return null;
  return state.origin === activeId ? state : null;
}

/**
 * El error de firma, **solo si sigue delante el documento que falló**.
 *
 * Simétrico a [`acknowledgementFor`]: cambiar de pestaña no lo cierra por su
 * cuenta —eso lo hace quien monte el panel, olvidando el ciclo a medias en el
 * backend con [`Signing.cancel`]—, pero no se enseña el error de un documento
 * sobre la pestaña de otro.
 */
export function failureFor(
  state: SigningState,
  activeId: string | null,
): Extract<SigningState, { kind: "failed" }> | null {
  if (state.kind !== "failed") return null;
  return state.origin === activeId ? state : null;
}
