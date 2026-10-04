//! El diálogo «Ver firmas»: las firmas que ya trae el documento, con su validez, que se mira y se cierra.

import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import { Dialog } from "../design-system/Dialog";
import { Row } from "../design-system/Row";
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
  const format = report.format ?? "pades";
  const count = t("panel.signed.count", { count: report.signatures.length });

  return (
    <Dialog
      label={t("panel.signed.title")}
      onClose={onClose}
      className="signatures-dialog"
      scrimClassName="signatures-dialog__scrim"
    >
      <div className="signatures-dialog__head">
        <p className="rf-title">{t("panel.signed.title")}</p>
        <p className="rf-body rf-text-muted">
          {format === "unrecognized" ? count : `${FORMAT_NAMES[format]} · ${count}`}
        </p>
      </div>

      <div className="signatures-dialog__scroll">
        <SignatureCards signatures={report.signatures} findings={report.findings} />
      </div>

      <hr className="rf-divider" />

      <Row className="signatures-dialog__actions">
        <Button variant="primary" onClick={onClose}>
          {t("actions.close")}
        </Button>
      </Row>
    </Dialog>
  );
}
