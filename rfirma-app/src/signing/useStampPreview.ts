import { useCallback, useEffect, useState } from "react";
import type { PdfDocument } from "../viewer/pdf";
import type { DocumentFailure } from "../viewer/source";
import type { StampComposer, StampPreview, StampRequest } from "./stampPreview";

interface StampPreviewInput {
  /** Quien compone el sello de verdad. Ver [`StampComposer`]. */
  composer: StampComposer;
  /** Qué hay que enseñar dentro del recuadro, si es que hay algo. */
  request: StampRequest;
  /** Se está arrastrando o redimensionando el recuadro. */
  gesturing: boolean;
  /** Documento grande: el recálculo se pide con «Ver cómo queda» (ID-109). */
  onDemand: boolean;
}

interface StampPreviewState {
  state: StampPreview;
  /**
   * El PDF **con el sello ya estampado**, que el visor pinta en lugar del
   * original. `null` cuando no hay ninguno que enseñar.
   */
  pdf: PdfDocument | null;
  /** «Ver cómo queda», y también «Volver a intentarlo». */
  compose: () => void;
}

/** Lo compuesto, atado a la orden de la que salió. Ver [`keyOf`]. */
interface Attempt<T> {
  key: string;
  value: T;
}

/**
 * La orden, reducida a una cadena con la que compararla.
 *
 * Se compara por valor y no por identidad **a propósito**: la orden se arma en
 * cada pintada a partir del recuadro, del certificado y de las casillas, así
 * que dos objetos distintos describen el mismo sello a cada rato. Por identidad
 * esto pediría un ciclo trifásico entero por pintada, que es exactamente lo que
 * el ID-109 prohíbe.
 */
function keyOf(request: StampRequest): string | null {
  return request.kind === "ready" ? JSON.stringify(request.order) : null;
}

/**
 * El ciclo de la vista previa: cuándo se compone el sello y qué se enseña
 * mientras tanto.
 *
 * Lo que hace que esto sea un módulo y no cuatro `useState` sueltos en el panel
 * es **cuándo no se compone**: sin certificado, sin colocar, durante el gesto,
 * en un documento grande que nadie ha pedido ver, y con la misma orden que ya
 * se compuso. Cada uno de esos cinco casos vale ≈1,9 s y 507 MB de RSS en el
 * peor documento medido, así que el trabajo de este módulo es no hacer nada.
 *
 * Y un fallo **no se reintenta solo**: el ID-111 dice que la vista previa no es
 * una puerta, y un bucle de reintentos contra un documento con contraseña sería
 * la peor puerta posible.
 */
export function useStampPreview({
  composer,
  request,
  gesturing,
  onDemand,
}: StampPreviewInput): StampPreviewState {
  const [composed, setComposed] = useState<Attempt<PdfDocument> | null>(null);
  const [failed, setFailed] = useState<Attempt<DocumentFailure> | null>(null);
  // La orden que se ha pedido componer a mano, que es la única que se compone
  // en un documento grande y la única que se vuelve a intentar tras un fallo.
  const [asked, setAsked] = useState<string | null>(null);
  const [composing, setComposing] = useState(false);

  const key = keyOf(request);
  const settled = composed?.key === key || failed?.key === key;
  const wanted = key !== null && !gesturing && !settled && (!onDemand || asked === key);

  // La orden vive en la clave, no en las dependencias: el efecto se dispara por
  // el valor de la orden y no por la identidad del objeto que la describe.
  const order = request.kind === "ready" ? request.order : null;
  // biome-ignore lint/correctness/useExhaustiveDependencies: `order` entra por `key`, que es su valor; por identidad esto compondría en bucle.
  useEffect(() => {
    if (!wanted || key === null || order === null) return;
    let live = true;
    setComposing(true);
    void composer.compose(order).then((result) => {
      if (!live) return;
      setComposing(false);
      if (result.ok) setComposed({ key, value: result.pdf });
      else setFailed({ key, value: result.failure });
    });
    return () => {
      live = false;
    };
  }, [composer, key, wanted]);

  const compose = useCallback(() => {
    if (key === null) return;
    setAsked(key);
    // Volver a intentarlo es olvidar el fallo: mientras esté apuntado contra
    // esta orden, la vista previa la da por resuelta y no vuelve a pedirla.
    setFailed((current) => (current?.key === key ? null : current));
  }, [key]);

  return {
    state: stateOf({ request, gesturing, onDemand, composing, composed, failed, key }),
    // Sin certificado o sin colocar **no hay recuadro**, así que tampoco hay
    // nada que pintar dentro: el visor vuelve al documento original.
    pdf: request.kind === "ready" ? (composed?.value ?? null) : null,
    compose,
  };
}

function stateOf({
  request,
  gesturing,
  onDemand,
  composing,
  composed,
  failed,
  key,
}: {
  request: StampRequest;
  gesturing: boolean;
  onDemand: boolean;
  composing: boolean;
  composed: Attempt<PdfDocument> | null;
  failed: Attempt<DocumentFailure> | null;
  key: string | null;
}): StampPreview {
  if (request.kind !== "ready") return { kind: request.kind };
  // Durante el gesto manda la vista anterior, aunque sea de otra orden: es lo
  // que se congela y se atenúa, y sigue sirviendo para medir el bulto.
  if (gesturing && composed !== null) return { kind: "frozen" };
  if (composed?.key === key) return { kind: "composed" };
  if (failed?.key === key) return { kind: "failed", failure: failed.value };
  if (composing) return { kind: "composing" };
  return onDemand ? { kind: "onDemand" } : { kind: "composing" };
}
