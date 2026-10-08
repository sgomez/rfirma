//! El certificado elegido en el desplegable: una búsqueda vuelve a proponer el de por omisión, un lector no toca lo elegido, y nada se recuerda hasta firmar.

import { useCallback, useState } from "react";
import { type Certificate, type CertificateState, isUsable } from "../signing/certificate";
import type { CertificateListing } from "../signing/useCertificateListing";
import { chosenFrom } from "./signingOrder";

interface Choice {
  listing: CertificateListing;
  state: CertificateState;
}

/** El estado del desplegable: cada búsqueda nueva vuelve a proponer el de por omisión (ADR-0010). */
export function useCertificateChoice(listing: CertificateListing) {
  const [choice, setChoice] = useState<Choice>(() => ({ listing, state: proposed(listing) }));

  let current = choice;
  if (choice.listing !== listing) {
    current = { listing, state: following(choice.state, listing) };
    setChoice(current);
  }

  const chooseCertificate = useCallback(
    (certificate: Certificate) => {
      if (listing.kind !== "listed") return;
      setChoice({
        listing,
        state: { kind: "chosen", certificate, certificates: listing.certificates },
      });
    },
    [listing],
  );

  return { certificate: current.state, chooseCertificate };
}

function proposed(listing: CertificateListing): CertificateState {
  return listing.kind === "listed" ? chosenFrom(listing.certificates) : listing;
}

function following(previous: CertificateState, listing: CertificateListing): CertificateState {
  if (listing.kind !== "listed" || listing.byReader !== true) return proposed(listing);
  return keptAcrossTheReader(previous, listing.certificates);
}

/** Lo elegido sigue si su tarjeta sigue; sin elegido, se elige el recordado que acaba de llegar. */
function keptAcrossTheReader(
  previous: CertificateState,
  found: readonly Certificate[],
): CertificateState {
  if (found.length === 0) return { kind: "empty" };
  if (previous.kind === "chosen") {
    const still = found.find((one) => one.id === previous.certificate.id);
    return still === undefined
      ? { kind: "unchosen", certificates: found }
      : { kind: "chosen", certificate: still, certificates: found };
  }
  const before = previous.kind === "unchosen" ? previous.certificates : [];
  const arrived = found.find(
    (one) => one.remembered && isUsable(one.status) && !before.some((known) => known.id === one.id),
  );
  return arrived === undefined
    ? { kind: "unchosen", certificates: found }
    : { kind: "chosen", certificate: arrived, certificates: found };
}
