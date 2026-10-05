//! Abrir el documento firmado o su carpeta, y el fallo al abrirlos, que se guarda hasta el siguiente intento o hasta que alguien lo olvida.

import { useCallback, useState } from "react";
import { classify, type NamedFailure } from "../errors/classify";
import type { SignedDocumentOpener } from "./destination";

/** Los dos caminos hasta el firmado y el último fallo al recorrerlos (ADR-0011). */
export function useSignedDocumentOpening(opener: SignedDocumentOpener) {
  const [failure, setFailure] = useState<NamedFailure | null>(null);

  const attempt = useCallback((open: () => Promise<void>) => {
    setFailure(null);
    open().catch((thrown: unknown) => setFailure(classify(thrown)));
  }, []);

  const openDocument = useCallback(
    (documentId?: string) => attempt(() => opener.openDocument(documentId)),
    [attempt, opener],
  );

  const openFolder = useCallback(
    (documentId?: string) => attempt(() => opener.openFolder(documentId)),
    [attempt, opener],
  );

  const forgetFailure = useCallback(() => setFailure(null), []);

  return { failure, openDocument, openFolder, forgetFailure };
}
