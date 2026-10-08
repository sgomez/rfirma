//! El recorrido de firma de la ventana principal detrás de una sola interfaz: ocho entradas, sus secciones y dos señales.

import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { DocumentInHand } from "../documents/document";
import type { OpenPdf } from "../documents/useOpenPdf";
import { classify, type NamedFailure } from "../errors/classify";
import { firstSealedPage } from "../placement/pageSets";
import type { PlacementState } from "../placement/usePlacement";
import type { DestinationSource, SignedDocumentOpener } from "../signing/destination";
import type { SigningBackend } from "../signing/flow";
import type { Rubric } from "../signing/rubric";
import type { StampComposer } from "../signing/stampPreview";
import type { useCertificateListing } from "../signing/useCertificateListing";
import { useSigning } from "../signing/useSigning";
import type { VisibleSignature } from "../signing/visibleSignature";
import { useCertificateChoice } from "./useCertificateChoice";
import { useDestination } from "./useDestination";
import { usePageGeometry } from "./usePageGeometry";
import { usePreviousSignatures } from "./usePreviousSignatures";
import { useSignFlow } from "./useSignFlow";
import { useSigningOutcome } from "./useSigningOutcome";
import { useVisibleSignature } from "./useVisibleSignature";

/** Los puertos de la ventana que usa el recorrido. */
interface SigningJourneyPorts {
  signer: SigningBackend;
  stamps: StampComposer;
  destinations: DestinationSource;
  opener: SignedDocumentOpener;
}

/** El documento abierto: el que hay delante y su PDF. */
interface OpenedDocument extends Pick<OpenPdf, "pdf" | "sizeBytes" | "opening"> {
  active: DocumentInHand | null;
}

interface SigningJourneyInput {
  ports: SigningJourneyPorts;
  document: OpenedDocument;
  placement: PlacementState;
  certificates: Pick<
    ReturnType<typeof useCertificateListing>,
    "listing" | "reader" | "lookAgain" | "install"
  >;
  rubric: Rubric | null;
  /** La carpeta de destino de los ajustes; `null` mientras no se han leído. */
  settingsFolder: string | null;
  initialSignature: VisibleSignature;
  reopenDocument: () => void;
}

/** El recorrido de firma: lo que pintan el visor, el panel de firma y el acuse. */
export function useSigningJourney({
  ports: { signer, stamps, destinations, opener },
  document: { active, pdf, sizeBytes, opening },
  placement: { placing, placement, placeOnViewedPage },
  certificates,
  rubric,
  settingsFolder,
  initialSignature,
  reopenDocument,
}: SigningJourneyInput) {
  const { i18n } = useTranslation();
  const language = i18n.resolvedLanguage ?? i18n.language;
  const activeId = active?.id ?? null;

  const { certificate, chooseCertificate } = useCertificateChoice(certificates.listing);
  const [installFailure, setInstallFailure] = useState<NamedFailure | null>(null);
  const installCertificate = async () => {
    setInstallFailure(null);
    try {
      await certificates.install();
    } catch (thrown) {
      setInstallFailure(classify(thrown));
    }
  };
  const lookCertificatesAgain = async () => {
    setInstallFailure(null);
    await certificates.lookAgain();
  };
  const chosen = certificate.kind === "chosen" ? certificate.certificate : null;
  const signing = useSigning(signer);
  const previousSignatures = usePreviousSignatures(signer, activeId);
  const { destination, singleDestinationId, chooseSingleDestination } = useDestination(
    destinations,
    activeId,
    settingsFolder,
    signing.state.kind,
  );

  const [signature, setSignature] = useVisibleSignature(initialSignature, chosen, {
    documentOpen: pdf !== null,
    placed: placing.rect !== null,
    placeOnViewedPage,
  });

  // Se lee aquí, y no en la vista previa, porque es asíncrono y el ciclo de la
  // firma se decide con la orden ya armada.
  const boxPage = placement === null ? null : (firstSealedPage(placement) ?? 1);
  const geometry = usePageGeometry(pdf, boxPage);

  const outcome = useSigningOutcome(signing, activeId, reopenDocument, signer, opener);

  const signFlow = useSignFlow({
    pdf,
    activeDocument: active,
    placement,
    geometry,
    boxPage,
    signature,
    rubric,
    opening,
    language,
    chosen,
    signer,
    stamps,
    sizeBytes,
    singleDestinationId,
    previousSignatures,
    startSigning: signing.start,
  });
  const { stamp } = signFlow;

  const running = signing.state.kind === "running";
  const promptOpen = signFlow.sealLossPrompt !== null || signFlow.signAnywayPrompt !== null;
  const { signedHere } = outcome;

  return {
    certificate: {
      state: certificate,
      reader: certificates.reader,
      choose: chooseCertificate,
      installFailure,
      install: installCertificate,
      lookAgain: lookCertificatesAgain,
    },
    signature: { value: signature, change: setSignature },
    previousSignatures,
    destination: {
      value: destination,
      chooseSingle: chooseSingleDestination,
    },
    stamp: {
      state: stamp.state,
      pdf: stamp.pdf,
      frozen: stamp.state.kind === "frozen",
      rubricGap: stamp.rubricGap,
      compose: stamp.compose,
      onGesture: stamp.onGesture,
    },
    signing: { running, sign: signFlow.sign, back: signing.cancel },
    failure: outcome.failedHere?.failure ?? null,
    acknowledgement:
      signedHere === null
        ? null
        : {
            documentName: signedHere.document.name,
            signedAt: signFlow.signingInstant,
            signatures: outcome.signatures,
            findings: outcome.findings,
            destination,
            changeDestination: chooseSingleDestination,
            openDocument: () => outcome.opening.openDocument(),
            openFolder: () => outcome.opening.openFolder(),
            signAgain: outcome.signAgain,
            openFailure: outcome.opening.failure,
          },
    signals: { signing: running, dialogOpen: promptOpen || running },
    dialogs: {
      sealLoss: signFlow.sealLossPrompt && {
        fallen: signFlow.sealLossPrompt.fallen,
        confirm: signFlow.signAnyway,
        cancel: () => signFlow.setSealLossPrompt(null),
      },
      signAnyway: signFlow.signAnywayPrompt && {
        problems: signFlow.signAnywayPrompt,
        confirm: signFlow.signDespiteProblems,
        cancel: () => signFlow.setSignAnywayPrompt(null),
      },
      stage: signing.state.kind === "running" ? signing.state.stage : null,
    },
  };
}

/** Lo que entrega el recorrido de firma. */
export type SigningJourney = ReturnType<typeof useSigningJourney>;
