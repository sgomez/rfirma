//! El destino previsto del firmado para el documento activo y el de una sola firma que lo sustituye.

import { useEffect, useState } from "react";
import type { Destination, DestinationSource } from "../signing/destination";
import type { SigningState } from "../signing/useSigning";

/**
 * Dónde caerá el firmado, tal y como lo cuenta el backend, y el destino de una
 * sola firma que lo puede sustituir. Es estado y no un cálculo del pie porque
 * el nombre lo compone Rust —con el sufijo y el homónimo ya resueltos— y
 * `writable` sale de comprobar la carpeta de verdad: la ventana lo
 * enseña, no lo deduce.
 *
 * El destino de una sola firma, elegido con «Cambiar», vale solo para el
 * documento activo: cambiar de pestaña o terminar de firmar lo olvida
 * (ADR-0011).
 */
export function useDestination(
  destinations: DestinationSource,
  activeId: string | null,
  chosenFolder: string | null,
  signingStateKind: SigningState["kind"],
) {
  const [singleDestinationId, setSingleDestinationId] = useState<string | null>(null);
  const [destination, setDestination] = useState<Destination | null>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `activeId` dispara el efecto, no lo alimenta.
  useEffect(() => {
    setSingleDestinationId(null);
  }, [activeId]);

  useEffect(() => {
    if (signingStateKind === "signed") setSingleDestinationId(null);
  }, [signingStateKind]);

  useEffect(() => {
    // Sin documento delante no hay destino que enseñar, y sin ajustes leídos
    // tampoco: la carpeta que se va a consultar es la que ellos dicen.
    if (activeId === null || chosenFolder === null) {
      setDestination(null);
      return;
    }
    let current = true;
    destinations
      .previewFor(activeId, singleDestinationId)
      .then((found) => {
        if (current) setDestination(found);
      })
      .catch(() => {
        // Un destino que no se puede consultar no apaga el panel: se queda sin
        // pie hasta la siguiente vuelta, que es menos que perder el documento.
        if (current) setDestination(null);
      });
    return () => {
      current = false;
    };
  }, [destinations, activeId, chosenFolder, singleDestinationId]);

  const chooseSingleDestination = async () => {
    if (activeId === null) return;
    const chosen = await destinations.chooseSingle(activeId);
    if (chosen !== null) setSingleDestinationId(chosen.id);
  };

  return {
    destination: destination ?? fallbackDestination(chosenFolder),
    singleDestinationId,
    chooseSingleDestination,
  };
}

/** El destino mientras el backend no ha respondido o ha fallado: la carpeta de los ajustes. */
function fallbackDestination(chosenFolder: string | null): Destination {
  return { folder: chosenFolder ?? "", name: null, writable: true };
}
