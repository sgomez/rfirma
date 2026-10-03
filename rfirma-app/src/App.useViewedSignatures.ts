//! El documento abierto para ver sus firmas (`verify --gui`) y lo que se lee de él.

import { useEffect, useRef, useState } from "react";
import { classify, type NamedFailure } from "./errors/classify";
import type { SigningBackend } from "./signing/flow";
import type { PreviousSignature, SignatureFormat } from "./signing/previousSignatures";

type Reading =
  | { kind: "reading" }
  | { kind: "read"; signatures: readonly PreviousSignature[]; format: SignatureFormat }
  | { kind: "failed"; failure: NamedFailure };

/** El documento abierto para ver sus firmas (`verify --gui`), con lo que se lee de él. */
export function useViewedSignatures(signer: SigningBackend, activeDocumentId: string | null) {
  const [viewedId, setViewedId] = useState<string | null>(null);
  const [reading, setReading] = useState<Reading>({ kind: "reading" });

  const viewing = viewedId !== null && viewedId === activeDocumentId;

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
    setReading({ kind: "reading" });
    let current = true;
    signer
      .previousSignatures(viewedId)
      .then((report) => {
        if (current) {
          setReading({
            kind: "read",
            signatures: report.signatures,
            format: report.format ?? "pades",
          });
        }
      })
      .catch((thrown: unknown) => {
        if (current) setReading({ kind: "failed", failure: classify(thrown) });
      });
    return () => {
      current = false;
    };
  }, [signer, viewedId]);

  return { viewing, reading, view: setViewedId, stopViewing: () => setViewedId(null) };
}
