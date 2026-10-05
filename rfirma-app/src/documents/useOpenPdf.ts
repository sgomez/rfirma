//! El PDF del documento activo, abierto por el puerto de PDF, con su fallo, su tamaño y una marca nueva por apertura; también cuenta el fallo de abrir y de vaciar recientes.

import { useEffect, useState } from "react";
import { classify } from "../errors/classify";
import type { PlacedDocument } from "../placement/usePlacement";
import type { PdfDocument } from "../viewer/pdf";
import type { DocumentFailure, PdfSource } from "../viewer/source";
import { isAPdf } from "./document";
import type { Documents } from "./useDocuments";

export interface OpenPdf {
  pdf: PdfDocument | null;
  failure: DocumentFailure | null;
  sizeBytes: number | null;
  /** Un valor nuevo por cada apertura, también del mismo documento; `null` sin PDF por abrir. */
  opening: PlacedDocument | null;
  /** Abre un documento por el selector; su fallo queda en `failure`. */
  open: () => void;
  /** Vacía los recientes; su fallo queda en `failure`. */
  clearRecents: () => void;
}

/** Abre el PDF del documento activo y se queda con lo que salga. */
export function useOpenPdf(documents: Documents, pdfs: PdfSource): OpenPdf {
  const [pdf, setPdf] = useState<PdfDocument | null>(null);
  const [failure, setFailure] = useState<DocumentFailure | null>(null);
  const [sizeBytes, setSizeBytes] = useState<number | null>(null);
  const [opening, setOpening] = useState<PlacedDocument | null>(null);

  useEffect(() => {
    const active = documents.active;
    if (!active || !isAPdf(active)) {
      setPdf(null);
      setFailure(null);
      setSizeBytes(null);
      setOpening(null);
      return;
    }
    let current = true;
    void pdfs.open(active).then((opened) => {
      if (!current) return;
      setPdf(opened.ok ? opened.pdf : null);
      setFailure(opened.ok ? null : opened.failure);
      setSizeBytes(opened.ok ? opened.sizeBytes : null);
      setOpening({
        placement: active.placement,
        pageCount: opened.ok ? opened.pdf.pageCount : 0,
      });
    });
    return () => {
      current = false;
    };
  }, [documents.active, pdfs]);

  const reporting = (command: () => Promise<void>) => () => {
    command().catch((thrown: unknown) => setFailure(classify(thrown)));
  };

  return {
    pdf,
    failure,
    sizeBytes,
    opening,
    open: reporting(documents.open),
    clearRecents: reporting(documents.clearRecents),
  };
}
