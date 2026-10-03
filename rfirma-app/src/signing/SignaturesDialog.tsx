//! El diálogo «Ver firmas»: las firmas que ya trae el documento, con su validez, que se mira y se cierra.

import { useId } from "react";
import { useTranslation } from "react-i18next";
import type { PreviousSignaturesReport } from "./previousSignatures";
import { SignatureCards } from "./SignatureCards";
import "./SignaturesDialog.css";

const FORMAT_NAMES = { pades: "PAdES", cades: "CAdES", xades: "XAdES" } as const;

interface SignaturesDialogProps {
  report: PreviousSignaturesReport;
  onClose: () => void;
}

/** El diálogo «Ver firmas» (docs/design/dialogo-ver-firmas.md), con una sola salida. */
export function SignaturesDialog({ report, onClose }: SignaturesDialogProps) {
  const { t } = useTranslation();
  const titleId = useId();
  const format = report.format ?? "pades";
  const count = t("panel.signed.count", { count: report.signatures.length });

  return (
    <div className="rf-scrim signatures-dialog__scrim">
      <div
        className="rf-dialog signatures-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <div className="signatures-dialog__head">
          <p className="rf-title" id={titleId}>
            {t("panel.signed.title")}
          </p>
          <p className="rf-body rf-text-muted">
            {format === "unrecognized" ? count : `${FORMAT_NAMES[format]} · ${count}`}
          </p>
        </div>

        <div className="signatures-dialog__scroll">
          <SignatureCards signatures={report.signatures} findings={report.findings} />
        </div>

        <hr className="rf-divider" />

        <div className="rf-row signatures-dialog__actions">
          <button type="button" className="rf-btn rf-btn--primary" onClick={onClose}>
            {t("actions.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
