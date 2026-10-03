//! La vista previa del sello y la firma, con los dos avisos que pueden interponerse antes del PIN.

import { useMemo, useState } from "react";
import type { PageGeometry } from "./App.signingOrder";
import { signingOrderFor } from "./App.signingOrder";
import type { DocumentInHand } from "./documents/document";
import type { Certificate } from "./signing/certificate";
import { isUsable } from "./signing/certificate";
import type { SigningBackend, SigningOrder } from "./signing/flow";
import {
  type PreviousSignaturesReport,
  type SigningProblem,
  signingProblems,
} from "./signing/previousSignatures";
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
  previousSignatures: PreviousSignaturesReport;
  startSigning: (
    certificate: Certificate,
    order: SigningOrder,
    singleDestinationId?: string | null,
  ) => Promise<void>;
}

/**
 * La firma **entera**: la vista previa del sello, la orden que se
 * manda y los dos avisos que pueden interponerse antes del PIN («¿Firmar de
 * todos modos?» y páginas sin sello).
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
  // El diálogo de páginas sin sello, guardado con la orden y el
  // certificado ya armados: `Firmar de todos modos` no rehace el viaje a
  // `pdf.js`, manda exactamente lo que se enseñó.
  const [sealLossPrompt, setSealLossPrompt] = useState<{
    fallen: number;
    chosen: number;
    certificate: Certificate;
    order: SigningOrder;
  } | null>(null);
  // «¿Firmar de todos modos?» con algún problema en el documento, antes de
  // tocar nada más. Cierra sin pedir nada al backend: ya sabe lo que necesita
  // del informe que trajo `usePreviousSignatures`.
  const [signAnywayPrompt, setSignAnywayPrompt] = useState<readonly SigningProblem[] | null>(null);

  // ── La vista previa del sello ───────────────────────────────────
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
        box: { placement, geometry },
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
   * (`signing::placement`), pero los datos de la
   * página los tiene `pdf.js`.
   */
  const sign = async () => {
    if (pdf === null || activeDocument === null || chosen === null) {
      // El botón ya está apagado sin certificado en vigor; aquí solo se
      // estrecha el tipo, y callar es mejor que fabricar una orden a medias.
      return;
    }
    const problems = signingProblems(previousSignatures);
    if (problems.length > 0) {
      setSignAnywayPrompt(problems);
      return;
    }

    await signPastPreviousSignatures(pdf, activeDocument, stampedPlacement(), chosen, false);
  };

  const showsUnknownSignatureType = (problem: SigningProblem) =>
    problem.kind === "unregisteredSignatures" ||
    (problem.kind === "signature" &&
      problem.signature.validityReason?.kind === "unknownSignatureType");

  const stampedPlacement = () => (signature.enabled ? placement : null);

  const signPastPreviousSignatures = async (
    pdf: PdfDocument,
    activeDocument: DocumentInHand,
    stamped: Placement | null,
    chosen: Certificate,
    consented: boolean,
  ) => {
    // La misma orden que compuso la vista previa, armada por el mismo sitio: si
    // aquí se armara a mano, lo que se enseñó y lo que se firma podrían
    // separarse sin que ninguna prueba lo notara.
    const order = signingOrderFor({
      documentId: activeDocument.id,
      certificate: chosen,
      box: stamped && { placement: stamped, geometry: await geometryOf(pdf, stamped) },
      pageCount: pdf.pageCount,
      signature,
      rubric,
      signedAt,
      language,
    });

    // El permiso de cofirmar sobre firmas de tipo desconocido solo sale de un
    // «Firmar igualmente» que enseñó esa fila; si no, se pregunta aquí. Si la
    // orden que lo averigua falla, la prefirma dirá lo que pasa.
    const unregistered = await signer.unregisteredSignatures(order.document).catch(() => false);
    if (unregistered && !consented) {
      setSignAnywayPrompt([{ kind: "unregisteredSignatures" }]);
      return;
    }
    const permitted = unregistered ? { ...order, allowUnregisteredSignatures: true } : order;

    await signUnlessTheSealFalls(chosen, permitted, pdf, stamped);
  };

  /**
   * La segunda mitad de `sign`: el aviso de páginas sin sello y, si no hay nada
   * que avisar, la firma.
   *
   * Está aparte para que la orden con el permiso de las firmas de tipo
   * desconocido caiga justo aquí, y no vuelva a empezar.
   */
  const signUnlessTheSealFalls = async (
    chosen: Certificate,
    order: SigningOrder,
    pdf: PdfDocument,
    placement: Placement | null,
  ) => {
    if (placement === null || order.placement === null) {
      await startSigning(chosen, order, singleDestinationId);
      return;
    }
    // `correctPositionSignature` descarta en silencio, contra cada
    // página, aquella donde no cabe la esquina inferior izquierda del
    // recuadro. Es el único aviso que queda desde que se cayó la tira del
    // visor, así que se calcula aquí, antes de mandar la orden.
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

  // `Firmar igualmente` de «¿Firmar de todos modos?»: el resto del recorrido
  // sigue igual, con el aviso de las páginas sin sello que aún puede interponerse.
  const signDespiteProblems = async () => {
    if (signAnywayPrompt === null || pdf === null || activeDocument === null || chosen === null) {
      return;
    }
    const consented = signAnywayPrompt.some(showsUnknownSignatureType);
    setSignAnywayPrompt(null);
    await signPastPreviousSignatures(pdf, activeDocument, stampedPlacement(), chosen, consented);
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
    signAnywayPrompt,
    setSignAnywayPrompt,
    signDespiteProblems,
  };
}

/**
 * La página que mide la `MediaBox` y la `/Rotate`: la primera del conjunto,
 * porque el widget se replica idéntico en todas.
 */
async function geometryOf(pdf: PdfDocument, placement: Placement): Promise<PageGeometry> {
  const first = firstSealedPage(placement) ?? 1;
  const page = await pdf.getPage(first);
  return { page: first, view: page.view, rotate: page.rotate };
}
