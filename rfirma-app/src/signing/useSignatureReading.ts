//! La lectura de firmas: el documento abierto solo para ver sus firmas (`verify --gui`) y todo lo que el panel necesita de ese momento.

import { useEffect, useRef, useState } from "react";
import { type DocumentInHand, isAPdf } from "../documents/document";
import type { RecentDocument } from "../documents/recents";
import { classify, type NamedFailure } from "../errors/classify";
import type { Destination, SignedDocumentOpener } from "./destination";
import type { SigningBackend } from "./flow";
import type { DocumentFinding, PreviousSignature, SignatureFormat } from "./previousSignatures";
import { useSignedDocumentOpening } from "./useSignedDocumentOpening";

/** El estado de la lectura de firmas: leyendo, leído o fallido. */
export type ReadingState =
  | { kind: "reading" }
  | {
      kind: "read";
      signatures: readonly PreviousSignature[];
      findings: readonly DocumentFinding[];
      format: SignatureFormat;
    }
  | { kind: "failed"; failure: NamedFailure };

/** El momento de la lectura: el estado de lo que se lee y lo que el panel puede hacer con el documento. */
type SignatureReading = ReadingState & {
  documentName: string;
  signable: boolean;
  destination: Destination;
  openDocument: () => void;
  openFolder: () => void;
  openFailure: NamedFailure | null;
  signAgain: () => void;
};

/** El documento abierto para ver sus firmas; `reading` es `null` mientras no se lee ninguno. */
export function useSignatureReading(
  signer: SigningBackend,
  opener: SignedDocumentOpener,
  active: Pick<DocumentInHand, "id" | "name"> | null,
  recentRow: Pick<RecentDocument, "folder"> | undefined,
) {
  const [viewedId, setViewedId] = useState<string | null>(null);
  const [state, setState] = useState<ReadingState>({ kind: "reading" });
  const opening = useSignedDocumentOpening(opener);
  const activeDocumentId = active?.id ?? null;

  // El resumen es de un documento concreto: cambiar de pestaña lo cierra.
  const wasInFront = useRef(false);
  useEffect(() => {
    if (activeDocumentId === viewedId) {
      wasInFront.current = viewedId !== null;
    } else if (wasInFront.current) {
      wasInFront.current = false;
      setViewedId(null);
    }
  }, [activeDocumentId, viewedId]);

  useEffect(() => {
    if (viewedId === null) return;
    setState({ kind: "reading" });
    let current = true;
    signer
      .previousSignatures(viewedId)
      .then((report) => {
        if (current) {
          setState({
            kind: "read",
            signatures: report.signatures,
            findings: report.findings,
            format: report.format ?? "pades",
          });
        }
      })
      .catch((thrown: unknown) => {
        if (current) setState({ kind: "failed", failure: classify(thrown) });
      });
    return () => {
      current = false;
    };
  }, [signer, viewedId]);

  const reading: SignatureReading | null =
    active !== null && viewedId === active.id
      ? {
          ...state,
          documentName: active.name,
          signable: isAPdf(active),
          destination: { folder: recentRow?.folder ?? "", name: active.name, writable: true },
          openDocument: () => opening.openDocument(active.id),
          openFolder: () => opening.openFolder(active.id),
          openFailure: opening.failure,
          signAgain: () => setViewedId(null),
        }
      : null;

  return { reading, view: setViewedId };
}
