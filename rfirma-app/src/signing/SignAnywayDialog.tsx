//! El diálogo «¿Firmar de todos modos?», justo antes de firmar, con una fila por problema del documento.

import { useId } from "react";
import { useTranslation } from "react-i18next";
import { AlertIcon, CrossCircleIcon } from "../design-system/icons";
import type { SigningProblem } from "./previousSignatures";
import { findingText, validityReasonText } from "./validityReasons";
import "./SignAnywayDialog.css";

interface SignAnywayDialogProps {
  problems: readonly SigningProblem[];
  locale: string;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * El diálogo «¿Firmar de todos modos?» (docs/design/dialogo-firmar-de-todos-modos.md).
 *
 * Aparece **justo antes de firmar** con cualquier ⚠, ✗ o hallazgo, con una
 * fila por problema —los hallazgos primero— y sin recuento: la lista ya lo
 * cuenta. Las firmas válidas no salen.
 */
export function SignAnywayDialog({ problems, locale, onConfirm, onCancel }: SignAnywayDialogProps) {
  const { t } = useTranslation();
  const titleId = useId();

  return (
    <div className="rf-scrim">
      <div
        className="rf-dialog sign-anyway-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <p className="rf-title" id={titleId}>
          {t("signAnyway.title")}
        </p>

        <ul className="sign-anyway-dialog__list">
          {problems.map((problem) => (
            <ProblemRow key={problemKey(problem)} problem={problem} locale={locale} />
          ))}
        </ul>

        <p className="rf-prose">{t("signAnyway.warning")}</p>

        <hr className="rf-divider" />

        <div className="rf-row sign-anyway-dialog__actions">
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

function problemKey(problem: SigningProblem): string {
  switch (problem.kind) {
    case "finding":
      return problem.finding;
    case "signature":
      return `signature-${problem.number}`;
  }
}

function ProblemRow({ problem, locale }: { problem: SigningProblem; locale: string }) {
  const { t } = useTranslation();
  if (problem.kind === "finding") {
    return (
      <li className="sign-anyway-dialog__row">
        <div className="sign-anyway-dialog__heading">
          <CrossCircleIcon size={15} />
          <span>{findingText(t, problem.finding)}</span>
        </div>
      </li>
    );
  }
  const { signature, number } = problem;
  const reason =
    signature.validityReason === null
      ? null
      : validityReasonText(t, signature.validityReason, locale);
  return (
    <li className="sign-anyway-dialog__row">
      <div className="sign-anyway-dialog__heading">
        {signature.validity === "expired" ? <AlertIcon size={15} /> : <CrossCircleIcon size={15} />}
        <span>{t("signAnyway.signature", { number, name: signature.name })}</span>
      </div>
      {reason !== null && (
        <div className="rf-body rf-text-muted sign-anyway-dialog__reason">{reason}</div>
      )}
    </li>
  );
}
