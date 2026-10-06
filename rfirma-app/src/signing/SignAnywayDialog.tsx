//! El diálogo «¿Firmar de todos modos?», justo antes de firmar, con una fila por problema del documento.

import { useTranslation } from "react-i18next";
import { useDefaultButton } from "../design-system/actionKeys";
import { Button } from "../design-system/Button";
import { Dialog } from "../design-system/Dialog";
import { AlertIcon, CrossCircleIcon } from "../design-system/icons";
import { Row } from "../design-system/Row";
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
  const primary = useDefaultButton();

  return (
    <Dialog
      label={t("signAnyway.title")}
      onClose={onCancel}
      primary={primary}
      className="sign-anyway-dialog"
    >
      <p className="rf-title">{t("signAnyway.title")}</p>

      <ul className="sign-anyway-dialog__list">
        {problems.map((problem) => (
          <ProblemRow key={problemKey(problem)} problem={problem} locale={locale} />
        ))}
      </ul>

      <p className="rf-prose">{t("signAnyway.warning")}</p>

      <hr className="rf-divider" />

      <Row className="sign-anyway-dialog__actions">
        <Button variant="ghost" onClick={onCancel}>
          {t("actions.cancel")}
        </Button>
        <Button variant="primary" onClick={onConfirm} ref={primary}>
          {t("actions.signAnyway")}
        </Button>
      </Row>
    </Dialog>
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
