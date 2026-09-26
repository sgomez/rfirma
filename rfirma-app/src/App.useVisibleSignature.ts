import { useMemo, useState } from "react";
import type { Certificate } from "./signing/certificate";
import type { VisibleSignature } from "./signing/visibleSignature";

/** La firma visible, apagada mientras no hay certificado elegido y recordada para cuando lo haya. */
export function useVisibleSignature(initial: VisibleSignature, chosen: Certificate | null) {
  const [remembered, setSignature] = useState<VisibleSignature>(initial);
  const signature = useMemo(
    () => (chosen === null ? { ...remembered, enabled: false } : remembered),
    [chosen, remembered],
  );
  return [signature, setSignature] as const;
}
