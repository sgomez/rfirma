//! La confirmación con la versión, la instalación y el mensaje de cada resultado fallido.

import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import "./InstallUpdateDialog.css";
import type { Installation, NewVersion, VersionCheck } from "./newVersion";

interface InstallUpdateDialogProps {
  newVersion: NewVersion;
  versions: VersionCheck;
  onClose: () => void;
}

type FailedInstallation = Exclude<Installation, "installed">;

type Phase =
  | { kind: "confirming" }
  | { kind: "installing" }
  | { kind: "failed"; reason: FailedInstallation };

/** Pide confirmación con la versión anunciada, instala y, si falla, lo explica sin cerrar la aplicación. */
export function InstallUpdateDialog({ newVersion, versions, onClose }: InstallUpdateDialogProps) {
  const { t } = useTranslation();
  const titleId = useId();
  const [phase, setPhase] = useState<Phase>({ kind: "confirming" });
  const { version } = newVersion;

  async function install() {
    setPhase({ kind: "installing" });
    let result: Installation;
    try {
      result = await versions.install();
    } catch {
      result = "notAvailable";
    }
    if (result !== "installed") setPhase({ kind: "failed", reason: result });
  }

  return (
    <div className="rf-scrim">
      <div
        className="rf-dialog install-update-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        {phase.kind === "confirming" && (
          <>
            <p className="rf-title" id={titleId}>
              {t("updates.install.confirmTitle", { version })}
            </p>
            <p className="rf-prose">{t("updates.install.confirmBody", { version })}</p>
            <div className="rf-row install-update-dialog__actions">
              <button type="button" className="rf-btn rf-btn--ghost" onClick={onClose}>
                {t("updates.install.postpone")}
              </button>
              <button
                type="button"
                className="rf-btn rf-btn--primary"
                onClick={() => void install()}
              >
                {t("updates.install.confirm")}
              </button>
            </div>
          </>
        )}
        {phase.kind === "installing" && (
          <p className="rf-title" id={titleId} role="status">
            {t("updates.install.installing")}
          </p>
        )}
        {phase.kind === "failed" && (
          <>
            <p className="rf-title" id={titleId}>
              {t("updates.install.failedTitle")}
            </p>
            <p className="rf-prose" role="alert">
              {t(`updates.install.failed.${phase.reason}`)}
            </p>
            <div className="rf-row install-update-dialog__actions">
              <button type="button" className="rf-btn rf-btn--primary" onClick={onClose}>
                {t("actions.close")}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
