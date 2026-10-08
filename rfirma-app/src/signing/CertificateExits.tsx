//! Las dos salidas de sin certificados, «Añadir un certificado…» y «Volver a buscar», y el error de la búsqueda fallida, justo debajo del selector.

import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import { Row } from "../design-system/Row";
import type { NamedFailure } from "../errors/classify";
import { ErrorNotice } from "../errors/ErrorNotice";
import type { CertificateState } from "./certificate";

/** Las salidas de sin certificados (docs/design/panel-de-firma.md § Estados); con certificados no pinta nada. */
export function CertificateExits({
  state,
  installFailure,
  onInstall,
  onLookAgain,
  onOpenHelp,
}: {
  state: CertificateState;
  installFailure: NamedFailure | null;
  onInstall: () => void;
  onLookAgain: () => void;
  onOpenHelp?: () => void;
}) {
  const { t } = useTranslation();

  if (state.kind !== "empty" && state.kind !== "failed") {
    return null;
  }

  return (
    <>
      <Row gap="xs" className="panel__certificate-actions">
        <Button variant="primary" className="panel__add-certificate" onClick={onInstall}>
          {t("panel.footer.addCertificate")}
        </Button>
        <Button variant="secondary" onClick={onLookAgain}>
          {t("actions.lookAgain")}
        </Button>
      </Row>
      {installFailure !== null && (
        <ErrorNotice situation={installFailure.situation} technicalDetail={installFailure.detail} />
      )}
      {state.kind === "failed" && (
        <ErrorNotice
          situation={state.failure.situation}
          technicalDetail={state.failure.detail}
          onOpenHelp={onOpenHelp}
        />
      )}
    </>
  );
}
