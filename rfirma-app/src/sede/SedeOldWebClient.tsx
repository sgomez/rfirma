import { useTranslation } from "react-i18next";
import { AlertIcon } from "../design-system/icons";
import { SedeBody } from "./SedeFrame";

/** El aviso de que la página usa un cliente web antiguo, que no detiene el trámite. */
export function SedeOldWebClient({ onDismiss }: { onDismiss: () => void }) {
  const { t } = useTranslation();

  return (
    <SedeBody
      steadyFooter
      footer={
        <>
          <div className="sede-window__spacer" />
          <button type="button" className="rf-btn rf-btn--primary" onClick={onDismiss}>
            {t("sede.oldWebClient.dismiss")}
          </button>
        </>
      }
    >
      <div className="rf-stack sede-outcome">
        <div className="rf-row rf-gap-xs sede-outcome__head">
          <span className="sede-outcome__icon">
            <AlertIcon size={24} />
          </span>
          <p className="rf-title sede-outcome__title">{t("sede.oldWebClient.title")}</p>
        </div>
        <p className="rf-prose">{t("sede.oldWebClient.body")}</p>
      </div>
    </SedeBody>
  );
}
