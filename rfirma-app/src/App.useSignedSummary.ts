//! El acuse de recibo del documento firmado y los dos caminos hasta el fichero.

import { useEffect, useState } from "react";
import type { SignedDocumentOpener } from "./signing/destination";
import type { SigningBackend } from "./signing/flow";
import type { DocumentFinding, PreviousSignature } from "./signing/previousSignatures";
import { useSignedDocumentOpening } from "./signing/useSignedDocumentOpening";
import { acknowledgementFor, type Signing } from "./signing/useSigning";

/**
 * El acuse de recibo del documento que se acaba de firmar, y los dos caminos
 * hasta el fichero. Solo sigue en pie mientras el documento firmado siga
 * activo: cambiar de fila lo cierra, en vez de esperar a que vuelva.
 */
export function useSignedSummary(
  signing: Signing,
  activeDocumentId: string | null,
  reopenDocument: () => void,
  signer: SigningBackend,
  opener: SignedDocumentOpener,
) {
  const opening = useSignedDocumentOpening(opener);
  const { forgetFailure } = opening;

  // El acuse de recibo, solo si sigue siendo de lo que hay delante. El estado
  // «Firmado» guarda el asa del documento que se firmó; el recuento de páginas
  // que enseña sale del PDF abierto, así que el panel solo puede montarse
  // mientras los dos sean el mismo documento.
  const signedHere = acknowledgementFor(signing.state, activeDocumentId);
  const signedSomewhere = signing.state.kind === "signed";

  // Las firmas del documento tal y como ha quedado, leídas del fichero escrito.
  // Un fallo al leerlas deja la lista vacía: el resumen sigue sirviendo para
  // llegar al fichero.
  const [signatures, setSignatures] = useState<readonly PreviousSignature[]>([]);
  const [findings, setFindings] = useState<readonly DocumentFinding[]>([]);
  const readingSignatures = signedHere !== null;
  useEffect(() => {
    setSignatures([]);
    setFindings([]);
    if (!readingSignatures) return;
    let current = true;
    signer
      .signedDocumentSignatures()
      .then((report) => {
        if (!current) return;
        setSignatures(report.signatures);
        setFindings(report.findings);
      })
      .catch(() => {});
    return () => {
      current = false;
    };
  }, [signer, readingSignatures]);

  // Y cuando deja de serlo —se elige otro en la bandeja, se olvida el activo,
  // se vacía la lista— el estado se cierra, en vez de quedarse esperando a que
  // el documento firmado vuelva a estar delante.
  const signAnother = signing.signAnother;
  useEffect(() => {
    if (signedSomewhere && signedHere === null) signAnother();
    // Y el fallo de abrir se va con el resumen del que hablaba: es de un
    // documento concreto, como el propio acuse de recibo.
    if (signedHere === null) forgetFailure();
  }, [signedSomewhere, signedHere, signAnother, forgetFailure]);

  // «Firmar»: se cierra el resumen y **se relee el original del
  // disco**. Es abrir el documento otra vez, porque entre una firma y
  // la siguiente el usuario ha podido modificarlo fuera o haberse equivocado al
  // configurar la firma. Lo que decida el recuadro recordado —incluido el aviso
  // de que ya no cabe— lo resuelve el camino de siempre, no uno nuevo.
  const signAgain = () => {
    forgetFailure();
    signing.signAnother();
    reopenDocument();
  };

  return { signedHere, signatures, findings, opening, signAgain };
}
