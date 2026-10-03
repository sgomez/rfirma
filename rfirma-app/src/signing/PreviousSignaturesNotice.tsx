//! El aviso de firmas previas del panel y de la sede: una línea con cuántas hay y la peor validez, «Ver firmas →» y la franja de «ya lo firmaste tú».

import type { ReactNode } from "react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertIcon, CrossCircleIcon, InfoIcon, PersonIcon } from "../design-system/icons";
import type { Certificate } from "./certificate";
import type { PreviousSignaturesReport } from "./previousSignatures";
import { SignaturesDialog } from "./SignaturesDialog";
import { sameSignerNotice } from "./sameSignerNotice";

type Worst = "valid" | "expired" | "invalid";

interface Problems {
  worst: Worst;
  expired: number;
  /** Las no válidas y los hallazgos del documento. */
  invalid: number;
}

function problemsOf(report: PreviousSignaturesReport): Problems {
  const expired = report.signatures.filter((signature) => signature.validity === "expired").length;
  const invalid =
    report.signatures.filter((signature) => signature.validity === "invalid").length +
    report.findings.length;
  const worst = invalid > 0 ? "invalid" : expired > 0 ? "expired" : "valid";
  return { worst, expired, invalid };
}

function worstIcon(worst: Worst): ReactNode {
  switch (worst) {
    case "valid":
      return <InfoIcon />;
    case "expired":
      return <AlertIcon />;
    case "invalid":
      return <CrossCircleIcon size={20} />;
  }
}

/**
 * El aviso de firmas previas: una línea con el icono de la peor validez,
 * «Junto a N firmas» y, si hay problemas, cuántos; «Ver firmas →» abre el
 * diálogo, y al pie queda «ya lo firmaste tú» si el certificado elegido
 * coincide (docs/design/panel-de-firma.md § El aviso de firmas previas). El
 * llamador solo lo monta con firmas.
 */
export function PreviousSignaturesNotice({
  report,
  certificate,
  presentation = "panel",
}: {
  report: PreviousSignaturesReport;
  certificate: Certificate | null;
  presentation?: "panel" | "site";
}) {
  const { t } = useTranslation();
  const [dialogOpen, setDialogOpen] = useState(false);
  const { worst, expired, invalid } = problemsOf(report);
  const notice = sameSignerNotice(certificate, report.signatures);

  return (
    <div
      className={`panel__co-signature panel__co-signature--${worst} panel__co-signature--${presentation}`}
    >
      <div className="panel__co-signature-line">
        <span className="panel__notice-icon">{worstIcon(worst)}</span>
        <span className="rf-prose panel__co-signature-text">
          {t("panel.previousSignatures.alongside", { count: report.signatures.length })}
          {worst !== "valid" && (
            <>
              {" · "}
              <strong>
                {worst === "expired"
                  ? t("panel.previousSignatures.expired", { count: expired })
                  : t("panel.previousSignatures.problems", { count: expired + invalid })}
              </strong>
            </>
          )}
        </span>
        <button
          type="button"
          className="rf-btn rf-btn--ghost panel__co-signature-view"
          onClick={() => setDialogOpen(true)}
        >
          {t("panel.previousSignatures.view")}
        </button>
      </div>
      {notice !== null && (
        <div className="panel__co-signature-footer">
          <span className="panel__notice-icon">
            <PersonIcon />
          </span>
          <span className="rf-body">
            {notice === "sameCertificate"
              ? t("panel.previousSignatures.sameCertificate")
              : t("panel.previousSignatures.otherCertificate")}
          </span>
        </div>
      )}
      {dialogOpen && <SignaturesDialog report={report} onClose={() => setDialogOpen(false)} />}
    </div>
  );
}
