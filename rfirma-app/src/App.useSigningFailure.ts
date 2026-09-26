import { useEffect } from "react";
import { failureFor, type Signing } from "./signing/useSigning";

/**
 * El error de firma del documento que se tiene delante, y su salida al
 * cambiar de pestaña.
 *
 * Cambiar de pestaña con un fallo en pantalla no lo deja pendiente: el ciclo a
 * medias se olvida en el backend, igual que pulsar «Volver» a mano, en vez de
 * quedarse esperando a que el documento que falló vuelva a estar delante.
 */
export function useSigningFailure(signing: Signing, activeDocumentId: string | null) {
  const failedHere = failureFor(signing.state, activeDocumentId);
  const failedSomewhere = signing.state.kind === "failed";

  const cancel = signing.cancel;
  useEffect(() => {
    if (failedSomewhere && failedHere === null) cancel();
  }, [failedSomewhere, failedHere, cancel]);

  return { failedHere };
}
