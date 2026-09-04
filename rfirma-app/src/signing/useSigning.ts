import { useRef, useState } from "react";
import type { Certificate } from "./certificate";
import { refusalFor, type SigningFailure } from "./failure";
import type { SignedDocument, SigningBackend, SigningOrder, SigningStage } from "./flow";
import { belongsToPinDialog, type TokenFailure } from "./token";

/**
 * En qué punto del recorrido de firma está la ventana.
 *
 * `pin` con un fallo dentro es la clave del reintento: un PIN incorrecto vuelve
 * a este mismo estado, **sin repetir la prefirma** y sin desmontar el diálogo.
 *
 * `signed` lleva dentro el **identificador del documento de partida** (`origin`), y no
 * solo el fichero que quedó escrito: el acuse de recibo es de un documento
 * concreto, y sin esa atadura la ventana lo enseñaba al lado de cualquier otro
 * que se abriera después —el nombre de A con el recuento de páginas de B—.
 */
export type SigningState =
  | { kind: "idle" }
  | { kind: "running"; stage: SigningStage }
  | { kind: "pin"; failure: TokenFailure | null }
  | { kind: "signed"; document: SignedDocument; origin: string }
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
 * El orden no es negociable y es el del ADR: **prefirma → PIN → firma →
 * postfirma**. Pedir el PIN antes de la prefirma sería pedir el secreto que
 * desbloquea la clave sin saber todavía qué se va a firmar.
 *
 * Dónde se cuenta cada fallo lo decide `belongsToPinDialog`, y no este bucle:
 * solo el PIN incorrecto es respuesta a lo que el usuario acaba de teclear, y
 * se contesta en el diálogo; el resto —incluida la tarjeta bloqueada— sale al
 * pie del panel, que es el estado «error de firma» de la ficha.
 *
 * El bucle ya no pasa siempre por el estado `pin`: cuando el almacén no
 * necesita sesión (ID-190), `advance` arranca directamente con la cadena
 * vacía y el diálogo no llega a abrirse.
 *
 * Quien implementa [`SigningBackend`] de verdad son las órdenes de Tauri del
 * #60; aquí solo se pide cada etapa por su turno.
 */
export function useSigning(backend: SigningBackend): Signing {
  const [state, setState] = useState<SigningState>({ kind: "idle" });
  // De qué documento es el ciclo en curso. Vive en una referencia y no en el
  // estado porque no se pinta en ninguna etapa: entra con la orden y solo
  // vuelve a salir al llegar a «Firmado», para atarlo a su documento.
  const origin = useRef<string | null>(null);

  const routed = (failure: TokenFailure): SigningState =>
    belongsToPinDialog(failure) ? { kind: "pin", failure } : { kind: "failed", failure };

  // Etapas 2 y 3, con el PIN que ya se tiene: el que se acaba de teclear, o la
  // cadena vacía cuando el almacén no necesita sesión (ID-190). Es el único
  // sitio donde `sign` se llama, tanto si el diálogo se abrió como si no.
  const advance = async (pin: string) => {
    setState({ kind: "running", stage: "sign" });
    const signed = await backend.sign(pin);
    if (!signed.ok) {
      setState(routed(signed.failure));
      return;
    }
    setState({ kind: "running", stage: "postsign" });
    const assembled = await backend.postsign();
    setState(
      assembled.ok
        ? { kind: "signed", document: assembled.value, origin: origin.current ?? "" }
        : routed(assembled.failure),
    );
  };

  const start = async (certificate: Certificate | null, order: SigningOrder) => {
    // El estado del certificado se sabe leyendo su DER, sin tocar la tarjeta:
    // pedir el PIN para luego fallar por una fecha ya conocida es hacer teclear
    // el secreto que desbloquea la clave para nada.
    const refusal = refusalFor(certificate);
    if (refusal) {
      setState({ kind: "failed", failure: refusal });
      return;
    }
    origin.current = order.document;
    setState({ kind: "running", stage: "presign" });
    const presigned = await backend.presign(order);
    if (!presigned.ok) {
      setState(routed(presigned.failure));
      return;
    }
    // Sin necesidad de sesión no hay diálogo: se firma directo, con la cadena
    // vacía (ID-190). El diálogo solo se abre cuando el almacén de verdad pide
    // un secreto.
    if (presigned.value.kind === "notNeeded") {
      await advance("");
      return;
    }
    setState({ kind: "pin", failure: null });
  };

  const submitPin = async (pin: string) => {
    await advance(pin);
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

  return { state, start, submitPin, cancel, signAnother };
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
