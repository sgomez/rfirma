//! El aviso de que la página usa un cliente web antiguo, que no detiene el trámite.

import { useTranslation } from "react-i18next";
import { useDefaultButton } from "../design-system/actionKeys";
import { Button } from "../design-system/Button";
import { AlertIcon } from "../design-system/icons";
import { Row } from "../design-system/Row";
import { Stack } from "../design-system/Stack";
import { SedeBody } from "./SedeFrame";

/** El aviso de que la página usa un cliente web antiguo, que no detiene el trámite. */
export function SedeOldWebClient({ onDismiss }: { onDismiss: () => void }) {
  const { t } = useTranslation();
  const dismissButton = useDefaultButton();

  return (
    <SedeBody
      primary={dismissButton}
      steadyFooter
      footer={
        <>
          <div className="sede-window__spacer" />
          <Button ref={dismissButton} variant="primary" onClick={onDismiss}>
            {t("actions.continue")}
          </Button>
        </>
      }
    >
      <Stack className="sede-outcome">
        <Row gap="xs" className="sede-outcome__head">
          <span className="sede-outcome__icon">
            <AlertIcon size={24} />
          </span>
          <p className="rf-title sede-outcome__title">{t("sede.oldWebClient.title")}</p>
        </Row>
        <p className="rf-prose">{t("sede.oldWebClient.body")}</p>
      </Stack>
    </SedeBody>
  );
}
