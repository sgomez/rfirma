//! La firma visible, apagada mientras no hay certificado elegido, que al encenderse con un documento delante coloca el recuadro en la página a la vista.

import { useEffect, useMemo, useState } from "react";
import type { Certificate } from "../signing/certificate";
import type { VisibleSignature } from "../signing/visibleSignature";

/** Lo que la firma visible necesita de la colocación para ponerse en la página. */
export interface SignaturePlacing {
  documentOpen: boolean;
  placed: boolean;
  placeOnViewedPage: () => void;
}

/** La firma visible, apagada mientras no hay certificado elegido y recordada para cuando lo haya. */
export function useVisibleSignature(
  initial: VisibleSignature,
  chosen: Certificate | null,
  { documentOpen, placed, placeOnViewedPage }: SignaturePlacing,
) {
  const [remembered, setSignature] = useState<VisibleSignature>(initial);
  const signature = useMemo(
    () => (chosen === null ? { ...remembered, enabled: false } : remembered),
    [chosen, remembered],
  );

  const signatureOn = signature.enabled && documentOpen;
  useEffect(() => {
    if (signatureOn && !placed) placeOnViewedPage();
  }, [signatureOn, placed, placeOnViewedPage]);

  return [signature, setSignature] as const;
}
