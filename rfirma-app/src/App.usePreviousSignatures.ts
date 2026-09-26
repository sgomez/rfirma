import { useEffect, useState } from "react";
import type { SigningBackend } from "./signing/flow";
import {
  NO_PREVIOUS_SIGNATURES,
  type PreviousSignaturesReport,
} from "./signing/previousSignatures";

/**
 * Las firmas que ya trae el documento activo, pedidas al abrir o cargar el
 * documento: no se vuelven a pedir al elegir certificado.
 *
 * Un fallo al pedirlas no es una puerta: el aviso simplemente no se monta, y
 * el resto del panel sigue funcionando igual que con un PDF sin firmas.
 */
export function usePreviousSignatures(
  signer: SigningBackend,
  activeDocumentId: string | null,
): PreviousSignaturesReport {
  const [report, setReport] = useState<PreviousSignaturesReport>(NO_PREVIOUS_SIGNATURES);

  useEffect(() => {
    setReport(NO_PREVIOUS_SIGNATURES);
    if (activeDocumentId === null) {
      return;
    }
    let current = true;
    void signer
      .previousSignatures(activeDocumentId)
      .then((found) => {
        if (current) setReport(found);
      })
      .catch(() => {
        if (current) setReport(NO_PREVIOUS_SIGNATURES);
      });
    return () => {
      current = false;
    };
  }, [signer, activeDocumentId]);

  return report;
}
