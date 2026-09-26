import { useId } from "react";
import { useTranslation } from "react-i18next";
import { formatSignedAt } from "../App.signingOrder";
import { reasonLabel, statusIcon, statusLabel } from "./PreviousSignaturesNotice";
import type { PreviousSignature } from "./previousSignatures";
import "./InvalidPreviousSignaturesDialog.css";

interface InvalidPreviousSignaturesDialogProps {
  /** Solo las que no son válidas: las demás ya están en el aviso del panel. */
  signatures: readonly PreviousSignature[];
  locale: string;
  onConfirm: () => void;
  onCancel: () => void;
}

/** El recuento en negrita, dentro de la frase que lo trae. */
function boldCount(message: string, count: number) {
  const text = String(count);
  const at = message.indexOf(text);
  if (at < 0) {
    return message;
  }
  return (
    <>
      {message.slice(0, at)}
      <strong>{text}</strong>
      {message.slice(at + text.length)}
    </>
  );
}

/**
 * El diálogo «¿Firmar de todos modos?» (docs/design/dialogo-firmar-de-todos-modos.md).
 *
 * Aparece **justo antes de firmar**, al pulsar «Firmar como…» con alguna
 * firma previa no válida: certificado caducado, aún no válido, rota o que no
 * se puede validar. No lo abren el cambio del documento ni una firma sin
 * comprobar del todo, que suben el tono del aviso pero no bloquean.
 */
export function InvalidPreviousSignaturesDialog({
  signatures,
  locale,
  onConfirm,
  onCancel,
}: InvalidPreviousSignaturesDialogProps) {
  const { t } = useTranslation();
  const titleId = useId();

  return (
    <div className="rf-scrim">
      <div
        className="rf-dialog invalid-previous-signatures-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <p className="rf-title" id={titleId}>
          {t("signAnyway.title")}
        </p>

        <p className="rf-prose">
          {boldCount(t("signAnyway.body", { count: signatures.length }), signatures.length)}
        </p>

        <ul className="invalid-previous-signatures-dialog__list">
          {signatures.map((signature) => (
            <InvalidSignatureRow
              key={`${signature.certificateSerialNumber}-${signature.signingTime ?? ""}`}
              signature={signature}
              locale={locale}
            />
          ))}
        </ul>

        <p className="rf-prose">{t("signAnyway.warning")}</p>

        <hr className="rf-divider" />

        <div className="rf-row invalid-previous-signatures-dialog__actions">
          <button type="button" className="rf-btn rf-btn--ghost" onClick={onCancel}>
            {t("actions.cancel")}
          </button>
          <button type="button" className="rf-btn rf-btn--primary" onClick={onConfirm}>
            {t("signAnyway.confirm")}
          </button>
        </div>
      </div>
    </div>
  );
}

function InvalidSignatureRow({
  signature,
  locale,
}: {
  signature: PreviousSignature;
  locale: string;
}) {
  const { t } = useTranslation();
  const reason = reasonLabel(t, signature.status, signature.reason);
  return (
    <li className="invalid-previous-signatures-dialog__row">
      <div className="invalid-previous-signatures-dialog__row-top">
        <span className="rf-body">{signature.name}</span>
        <span className="invalid-previous-signatures-dialog__verdict">
          {statusIcon(signature.status)}
          <span className="rf-body">{statusLabel(t, signature.status)}</span>
        </span>
      </div>
      <div className="rf-body rf-text-muted">
        {signature.signingTime === null
          ? ""
          : formatSignedAt(new Date(signature.signingTime), locale)}
      </div>
      {reason !== null && <div className="rf-body rf-text-muted">{reason}</div>}
    </li>
  );
}
