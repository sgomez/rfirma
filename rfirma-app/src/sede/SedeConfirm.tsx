import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { AlertIcon } from "../design-system/icons";
import { SedeBody } from "./SedeFrame";

interface SedeConfirmProps {
  messageCode: string;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * **2b · Hay que confirmar.** El validador del original no da por buenas las
 * firmas que el documento ya trae y la decisión no es de rFirma: se pregunta con
 * las palabras del original y no se resume ni se reinterpreta.
 *
 * Es un momento y no un diálogo encima del consentimiento: llega **antes** de
 * que haya nada que consentir, y cancelar aquí es contestarle a la sede, no
 * cerrar un aviso.
 */
export function SedeConfirm({ messageCode, onConfirm, onCancel }: SedeConfirmProps) {
  const { t } = useTranslation();

  return (
    <SedeBody
      footer={
        <>
          <div className="sede-window__spacer" />
          <button type="button" className="rf-btn rf-btn--ghost" onClick={onCancel}>
            {t("actions.cancel")}
          </button>
          <button type="button" className="rf-btn rf-btn--primary" onClick={onConfirm}>
            {t("sede.confirm.continue")}
          </button>
        </>
      }
    >
      <div className="rf-stack sede-confirm">
        <div className="rf-row rf-gap-xs sede-confirm__heading">
          <span className="sede-confirm__icon">
            <AlertIcon size={18} />
          </span>
          <p className="rf-title">{t("sede.confirm.title")}</p>
        </div>
        <p className="rf-prose">{confirmationMessage(t, messageCode)}</p>
      </div>
    </SedeBody>
  );
}

// Lo que pregunta el original, ya traducido: `t()` no admite una clave armada.
function confirmationMessage(t: TFunction, messageCode: string): string {
  switch (messageCode) {
    case "pdfShadowAttackSuspect":
      return t("sede.confirm.messages.pdfShadowAttackSuspect");
    case "signingModifiedPdfForm":
      return t("sede.confirm.messages.signingModifiedPdfForm");
    default:
      return t("sede.confirm.messages.unknown", { code: messageCode });
  }
}
