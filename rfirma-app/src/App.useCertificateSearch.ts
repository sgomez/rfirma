import { useCallback, useEffect, useState } from "react";
import { chosenFrom } from "./App.signingOrder";
import { classify } from "./errors/classify";
import type { Certificate, CertificateStore } from "./signing/certificate";
import { installedCertificates } from "./signing/certificate";
import type { CertificateState } from "./signing/SigningPanel";

/**
 * Los certificados de los tokens conectados, buscados al arrancar y cada vez
 * que alguien pide volver a buscar: una tarjeta insertada tarde es el caso
 * corriente, no la excepción.
 */
export function useCertificateSearch(certificates: CertificateStore) {
  const [certificate, setCertificate] = useState<CertificateState>({ kind: "loading" });

  const lookForCertificates = useCallback(async () => {
    setCertificate({ kind: "loading" });
    try {
      const found = await certificates.list();
      setCertificate(chosenFrom(found));
    } catch (thrown) {
      // El rechazo se recoge **aquí** y no se deja escapar (ID-11): sin este
      // `catch` nadie volvía a llamar a `setCertificate` y la ficha se quedaba
      // girando en «Buscando certificados…» para siempre. Se clasifica con el
      // mismo `classify` que el visor y la rúbrica: no hay dos formas de
      // contar un error (ID-29).
      setCertificate({ kind: "failed", failure: classify(thrown) });
    }
  }, [certificates]);

  useEffect(() => {
    void lookForCertificates();
  }, [lookForCertificates]);

  /**
   * Los `.p12` instalados, que son los que enseña Preferencias.
   *
   * Salen del **mismo** listado que el desplegable y no de una segunda orden:
   * instalar o quitar uno cambia las dos pantallas a la vez, y con dos listados
   * independientes una de ellas se quedaría contando lo de antes.
   */
  const listed = certificate.kind === "chosen" || certificate.kind === "unchosen";
  const installed = installedCertificates(listed ? certificate.certificates : []);

  // Los dos gestos de Preferencias vuelven a buscar: lo que acaba de entrar o
  // de salir tiene que aparecer también en el desplegable de la firma.
  const installCertificate = useCallback(
    async (password: string) => {
      const chosen = await certificates.install(password);
      if (chosen) await lookForCertificates();
      return chosen;
    },
    [certificates, lookForCertificates],
  );

  const removeCertificate = useCallback(
    async (id: string) => {
      await certificates.remove(id);
      await lookForCertificates();
    },
    [certificates, lookForCertificates],
  );

  /**
   * Elegir un certificado del desplegable.
   *
   * Solo cambia cuál está puesto: **no** se recuerda aquí. El certificado se
   * recuerda al firmar con él, que es lo que dicen el glosario —«el certificado
   * usado la última vez»— y la historia 7 —«con cuál firmé»—; ninguna dice «el
   * último que miré» (ADR-0010).
   */
  const chooseCertificate = useCallback((chosen: Certificate) => {
    setCertificate((state) =>
      state.kind === "unchosen" || state.kind === "chosen"
        ? { kind: "chosen", certificate: chosen, certificates: state.certificates }
        : state,
    );
  }, []);

  return {
    certificate,
    lookForCertificates,
    installed,
    installCertificate,
    removeCertificate,
    chooseCertificate,
  };
}
