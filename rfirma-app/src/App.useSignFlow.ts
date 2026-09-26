import { useMemo, useState } from "react";
import type { PageGeometry } from "./App.signingOrder";
import { signingOrderFor } from "./App.signingOrder";
import type { DocumentInHand } from "./documents/document";
import type { Certificate } from "./signing/certificate";
import { isUsable } from "./signing/certificate";
import type { SigningBackend, SigningOrder } from "./signing/flow";
import { invalidSignatures, type PreviousSignature } from "./signing/previousSignatures";
import type { Rubric } from "./signing/rubric";
import { composesOnRelease, type StampComposer, type StampRequest } from "./signing/stampPreview";
import { pagesWithoutSeal } from "./signing/unsealedPages";
import { useStampPreview } from "./signing/useStampPreview";
import { rubricGapFor, type VisibleSignature } from "./signing/visibleSignature";
import type { PdfDocument } from "./viewer/pdf";
import { firstSealedPage, type Placement, sealedPages } from "./viewer/signatureBox";

interface SignFlowInput {
  pdf: PdfDocument | null;
  activeDocument: DocumentInHand | null;
  placement: Placement | null;
  geometry: PageGeometry | null;
  boxPage: number | null;
  signature: VisibleSignature;
  rubric: Rubric | null;
  signedAt: string;
  language: string;
  chosen: Certificate | null;
  signer: SigningBackend;
  stamps: StampComposer;
  sizeBytes: number | null;
  gesturing: boolean;
  /** El destino elegido para esta firma con «Cambiar», sin tocar la preferencia (ADR-0011). */
  singleDestinationId: string | null;
  previousSignatures: readonly PreviousSignature[];
  startSigning: (
    certificate: Certificate,
    order: SigningOrder,
    singleDestinationId?: string | null,
  ) => Promise<void>;
}

/**
 * La firma **entera**: la vista previa del sello (ID-107), la orden que se
 * manda y los tres avisos que pueden interponerse antes del PIN (ID-105,
 * ID-297…ID-301, y el de las firmas previas no válidas).
 */
export function useSignFlow({
  pdf,
  activeDocument,
  placement,
  geometry,
  boxPage,
  signature,
  rubric,
  signedAt,
  language,
  chosen,
  signer,
  stamps,
  sizeBytes,
  gesturing,
  singleDestinationId,
  previousSignatures,
  startSigning,
}: SignFlowInput) {
  // El diálogo de páginas sin sello (ID-105), guardado con la orden y el
  // certificado ya armados: `Firmar de todos modos` no rehace el viaje a
  // `pdf.js`, manda exactamente lo que se enseñó.
  const [sealLossPrompt, setSealLossPrompt] = useState<{
    fallen: number;
    chosen: number;
    certificate: Certificate;
    order: SigningOrder;
  } | null>(null);
  // El aviso de las firmas sin registrar (ID-297…ID-301), guardado igual que el
  // anterior: decir que sí no rehace el viaje a `pdf.js`, manda la misma orden
  // con el permiso puesto.
  const [unregisteredPrompt, setUnregisteredPrompt] = useState<{
    certificate: Certificate;
    order: SigningOrder;
  } | null>(null);
  // «¿Firmar de todos modos?» con alguna firma previa no válida, antes de
  // tocar nada más — ni la comprobación de firmas sin registrar, ni la del
  // sello. Cierra sin pedir nada al backend: ya sabe lo que necesita del
  // informe que trajo `usePreviousSignatures`.
  const [invalidPreviousSignaturesPrompt, setInvalidPreviousSignaturesPrompt] = useState<
    readonly PreviousSignature[] | null
  >(null);

  // ── La vista previa del sello (ID-107) ───────────────────────────────────
  //
  // La orden en seco es **la misma** que se manda a firmar: por eso lo que se
  // ve dentro del recuadro coincide con el PDF firmado, y no porque nadie lo
  // compare. Lo que la ventana decide aquí es sólo si hay algo que componer.
  const stampRequest: StampRequest = useMemo(() => {
    if (chosen === null || !isUsable(chosen.status)) return { kind: "noCertificate" };
    if (
      !signature.enabled ||
      pdf === null ||
      placement === null ||
      geometry === null ||
      activeDocument === null ||
      // La geometría llega por un efecto asíncrono: al cambiar el conjunto de
      // páginas hay una pintada con la página, la `MediaBox` y la `/Rotate`
      // viejas junto al recuadro nuevo. Componer eso cuesta un ciclo entero
      // para enseñar el sello de la página anterior.
      geometry.page !== boxPage
    ) {
      return { kind: "unplaced" };
    }
    return {
      kind: "ready",
      order: signingOrderFor({
        documentId: activeDocument.id,
        certificate: chosen,
        placement,
        geometry,
        pageCount: pdf.pageCount,
        signature,
        rubric,
        signedAt,
        language,
      }),
    };
  }, [
    chosen,
    signature,
    pdf,
    placement,
    geometry,
    boxPage,
    activeDocument,
    rubric,
    signedAt,
    language,
  ]);

  const stamp = useStampPreview({
    composer: stamps,
    request: stampRequest,
    gesturing,
    onDemand: !composesOnRelease(sizeBytes),
  });

  /**
   * La firma: se arma la orden con lo que hay decidido y se manda entera.
   *
   * La `MediaBox` y la `/Rotate` salen de la página abierta porque el backend
   * **no lee PDFs**: la conversión del recuadro a puntos PAdES es suya
   * (`signing::placement`, con la guardia del ID-22), pero los datos de la
   * página los tiene `pdf.js`.
   */
  const sign = async () => {
    if (pdf === null || activeDocument === null || placement === null || chosen === null) {
      // El botón ya está apagado sin certificado en vigor; aquí solo se
      // estrecha el tipo, y callar es mejor que fabricar una orden a medias.
      return;
    }
    // Con alguna firma previa no válida, «Firmar como…» pregunta antes de
    // tocar nada más.
    const invalid = invalidSignatures(previousSignatures);
    if (invalid.length > 0) {
      setInvalidPreviousSignaturesPrompt(invalid);
      return;
    }

    await signPastPreviousSignatures(pdf, activeDocument, placement, chosen);
  };

  const signPastPreviousSignatures = async (
    pdf: PdfDocument,
    activeDocument: DocumentInHand,
    placement: Placement,
    chosen: Certificate,
  ) => {
    // La página que mide la `MediaBox` y la `/Rotate` es la **primera del
    // conjunto**: el widget se replica idéntico en todas (ID-96), así que
    // cualquiera de ellas daría la misma conversión, y la primera es la única
    // que se puede nombrar sin elegir.
    const first = firstSealedPage(placement) ?? 1;
    const page = await pdf.getPage(first);
    // La misma orden que compuso la vista previa, armada por el mismo sitio: si
    // aquí se armara a mano, lo que se enseñó y lo que se firma podrían
    // separarse sin que ninguna prueba lo notara.
    const order = signingOrderFor({
      documentId: activeDocument.id,
      certificate: chosen,
      placement,
      geometry: { page: first, view: page.view, rotate: page.rotate },
      pageCount: pdf.pageCount,
      signature,
      rubric,
      signedAt,
      language,
    });

    // ID-297 / ID-300: si el documento trae firmas que no sabemos leer, la
    // pregunta va **antes** del PIN. No es un rechazo: sin ella el puente
    // aborta la cofirma con `PdfHasUnregisteredSignaturesException`, y con un
    // «sí» la orden sale con el permiso puesto. Si la orden que lo averigua
    // falla, no se inventa un aviso: la prefirma dirá lo que pasa de verdad.
    const unregistered = await signer.unregisteredSignatures(order.document).catch(() => false);
    if (unregistered) {
      setUnregisteredPrompt({ certificate: chosen, order });
      return;
    }

    await signUnlessTheSealFalls(chosen, order, pdf, placement);
  };

  /**
   * La segunda mitad de `sign`: el aviso de páginas sin sello y, si no hay nada
   * que avisar, la firma.
   *
   * Está aparte porque los dos avisos previos —éste y el de las firmas sin
   * registrar— van en fila: aceptar el primero tiene que caer justo aquí, y no
   * volver a empezar.
   */
  const signUnlessTheSealFalls = async (
    chosen: Certificate,
    order: SigningOrder,
    pdf: PdfDocument,
    placement: Placement,
  ) => {
    // ID-105: `correctPositionSignature` descarta en silencio, contra cada
    // página, aquella donde no cabe la esquina inferior izquierda del
    // recuadro. Es el único aviso que queda desde que se cayó la tira del
    // visor (#152), así que se calcula aquí, antes de mandar la orden.
    //
    // La esquina se compara en **puntos PAdES**, no en espacio de usuario
    // PDF: son espacios distintos en cuanto la `/Rotate` no es 0, y la
    // conversión (`T⁻¹`) no tiene copia en TypeScript — se pide al backend,
    // que es quien la aplica de verdad al armar la orden.
    const chosenPages = sealedPages(placement.pages, pdf.pageCount);
    const views = await Promise.all(
      chosenPages.map(async (number) => ({ number, view: (await pdf.getPage(number)).view })),
    );
    const [lowerLeftX, lowerLeftY] = await signer.padesLowerLeft(order.placement);
    const fallen = pagesWithoutSeal({ x: lowerLeftX, y: lowerLeftY }, views);
    if (fallen.length > 0) {
      setSealLossPrompt({
        fallen: fallen.length,
        chosen: chosenPages.length,
        certificate: chosen,
        order,
      });
      return;
    }

    await startSigning(chosen, order, singleDestinationId);
  };

  // `Firmar de todos modos` del aviso de las firmas sin registrar: la misma
  // orden, ahora con el permiso que el puente necesita (ID-301).
  const signWithUnregisteredSignatures = async () => {
    if (unregisteredPrompt === null || pdf === null || placement === null) return;
    const { certificate: chosen, order } = unregisteredPrompt;
    setUnregisteredPrompt(null);
    await signUnlessTheSealFalls(
      chosen,
      { ...order, allowUnregisteredSignatures: true },
      pdf,
      placement,
    );
  };

  // `Firmar de todos modos` del diálogo de firmas previas no válidas: el
  // resto del recorrido sigue igual, con los dos avisos que todavía pueden
  // interponerse.
  const signDespiteInvalidPreviousSignatures = async () => {
    if (
      invalidPreviousSignaturesPrompt === null ||
      pdf === null ||
      activeDocument === null ||
      placement === null ||
      chosen === null
    ) {
      return;
    }
    setInvalidPreviousSignaturesPrompt(null);
    await signPastPreviousSignatures(pdf, activeDocument, placement, chosen);
  };

  // `Firmar de todos modos`: la orden ya estaba armada, se manda tal cual.
  const signAnyway = async () => {
    if (sealLossPrompt === null) return;
    const { certificate: chosen, order } = sealLossPrompt;
    setSealLossPrompt(null);
    await startSigning(chosen, order, singleDestinationId);
  };

  return {
    stamp: { ...stamp, rubricGap: rubricGapFor(signature, rubric !== null) },
    sign,
    sealLossPrompt,
    setSealLossPrompt,
    signAnyway,
    unregisteredPrompt,
    setUnregisteredPrompt,
    signWithUnregisteredSignatures,
    invalidPreviousSignaturesPrompt,
    setInvalidPreviousSignaturesPrompt,
    signDespiteInvalidPreviousSignatures,
  };
}
