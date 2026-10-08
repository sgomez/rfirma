//! La ventana de sede: sigue el trámite por `SiteErrandPort` y se lo pasa a `SedeView`.

import { useEffect, useState } from "react";
import type { ExternalDestinationOpener } from "../desktop/externalDestination";
import { classify, type NamedFailure } from "../errors/classify";
import { type Certificate, NO_READER, type ReaderStatus } from "../signing/certificate";
import type { Errand, SiteErrandPort } from "./errand";
import { SedeView } from "./SedeView";

/** La lista que trajo un lector para el momento del trámite en que llegó; otro momento vuelve a la de la sede. */
interface LiveList {
  errand: Errand;
  certificates: readonly Certificate[];
}

interface SedeWindowProps {
  errands: SiteErrandPort;
  externalDestinations?: ExternalDestinationOpener;
  consentCountdown?: boolean;
  onOpenHelp?: () => void;
}

/**
 * La ventana que abre rFirma cuando una sede electrónica lo invoca por
 * `afirma://` (docs/design/ventana-de-sede.md).
 *
 * **Una ventana con una secuencia, no una pantalla por momento**: todos los
 * momentos comparten las dos regiones fijas —cuerpo y pie— y lo único que cambia es lo
 * que va dentro. Sin cabecera de aplicación, sin menú, sin bandeja y sin pie de
 * destino: sugerir que hay más dentro invita a buscar cosas que no están.
 *
 * **La barra de título es la del sistema operativo**, no una pintada aquí: una
 * de mentira no la mueve el gestor de ventanas, así que la ventana no se podía
 * ni arrastrar. Con ella vienen gratis el título, la cruz, el menú del gestor y
 * el arrastre; y cerrar por la cruz llega igual al backend, que trata
 * `CloseRequested` sobre esta ventana como abandonar el trámite.
 *
 * El desenlace se cierra solo a los `OUTCOME_CLOSE_MS`. El momento
 * de «no ha llegado» lo decide el backend con su reloj de respaldo y lo publica
 * como un momento más.
 */
export function SedeWindow({
  errands,
  externalDestinations,
  consentCountdown = true,
  onOpenHelp,
}: SedeWindowProps) {
  const [errand, setErrand] = useState<Errand | null>(null);

  useEffect(() => errands.watch(setErrand), [errands]);

  if (errand === null) return null;
  return (
    <SedeDialog
      errand={errand}
      errands={errands}
      externalDestinations={externalDestinations}
      consentCountdown={consentCountdown}
      onOpenHelp={onOpenHelp}
    />
  );
}

/**
 * La ventana ya con trámite. Va aparte para que los relojes de cada momento
 * arranquen al entrar en él y no al montar el árbol: montados en
 * `SedeWindow`, un `useEffect` con `errand` en las dependencias volvería a
 * contar los 15 segundos con cada latido del puerto.
 */
function SedeDialog({
  errand,
  errands,
  externalDestinations,
  consentCountdown,
  onOpenHelp,
}: {
  errand: Errand;
  errands: SiteErrandPort;
  externalDestinations?: ExternalDestinationOpener;
  consentCountdown: boolean;
  onOpenHelp?: () => void;
}) {
  const openHelp = () => {
    onOpenHelp?.();
    void externalDestinations?.open("discussions");
  };

  const [reader, setReader] = useState<ReaderStatus>(NO_READER);
  const [live, setLive] = useState<LiveList | null>(null);
  useEffect(
    () =>
      errands.followReaders((news) => {
        setReader(news.reader);
        if (news.certificates !== null) setLive({ errand, certificates: news.certificates });
      }),
    [errands, errand],
  );

  const [installFailure, setInstallFailure] = useState<NamedFailure | null>(null);
  // Un fallo al instalar se enseña en línea, con la misma clasificación que
  // Preferencias; cancelar el selector de fichero no rechaza nada.
  const installCertificate = async () => {
    setInstallFailure(null);
    try {
      await errands.installCertificate();
    } catch (thrown) {
      setInstallFailure(classify(thrown));
    }
  };

  return (
    <SedeView
      errand={errand}
      consentCountdown={consentCountdown}
      installFailure={installFailure}
      reader={reader}
      liveCertificates={live?.errand === errand ? live.certificates : null}
      onConsent={(certificateId) => void errands.consent(certificateId)}
      onConfirmSignatures={() => errands.confirmSignatures()}
      onMarkArea={(area) => errands.markArea(area)}
      onCancel={() => void errands.cancel()}
      onClose={() => void errands.close()}
      onLookAgain={() => void errands.lookAgain()}
      onInstallCertificate={() => void installCertificate()}
      onInstallLocalCa={() => void errands.installLocalCa()}
      onDismissWarning={() => void errands.dismissWarning()}
      onOpenHelp={openHelp}
    />
  );
}
