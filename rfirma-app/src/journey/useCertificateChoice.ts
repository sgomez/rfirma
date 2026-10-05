//! El certificado elegido en el desplegable, que dura lo que dura el listado del que salió y no se recuerda hasta firmar.

import { useCallback, useMemo, useState } from "react";
import type { Certificate } from "../signing/certificate";
import type { CertificateState } from "../signing/SigningPanel";
import type { CertificateListing } from "../signing/useCertificateListing";
import { chosenFrom } from "./signingOrder";

interface Choice {
  listing: CertificateListing;
  certificate: Certificate;
}

/** El estado del desplegable: cada listado nuevo vuelve a proponer el de por omisión (ADR-0010). */
export function useCertificateChoice(listing: CertificateListing) {
  const [choice, setChoice] = useState<Choice | null>(null);

  const chooseCertificate = useCallback(
    (certificate: Certificate) => {
      if (listing.kind === "listed") setChoice({ listing, certificate });
    },
    [listing],
  );

  const certificate = useMemo(() => certificateStateOf(listing, choice), [listing, choice]);

  return { certificate, chooseCertificate };
}

function certificateStateOf(listing: CertificateListing, choice: Choice | null): CertificateState {
  if (listing.kind !== "listed") return listing;
  if (choice?.listing === listing) {
    return { kind: "chosen", certificate: choice.certificate, certificates: listing.certificates };
  }
  return chosenFrom(listing.certificates);
}
