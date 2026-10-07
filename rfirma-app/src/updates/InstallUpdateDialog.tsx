//! La confirmación con la versión, la instalación y el mensaje de cada resultado fallido.

import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import "./InstallUpdateDialog.css";
import { Button } from "../design-system/Button";
import { Dialog } from "../design-system/Dialog";
import { Row } from "../design-system/Row";
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
  const [phase, setPhase] = useState<Phase>({ kind: "confirming" });
  const { version } = newVersion;
  const primary = useRef<HTMLButtonElement>(null);

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

  const label = {
    confirming: t("updates.install.confirmTitle", { version }),
    installing: t("updates.install.installing"),
    failed: t("updates.install.failedTitle"),
  }[phase.kind];

  return (
    <Dialog
      label={label}
      onClose={phase.kind === "installing" ? undefined : onClose}
      primary={primary}
      className="install-update-dialog"
    >
      {phase.kind === "confirming" && (
        <>
          <p className="rf-title">{t("updates.install.confirmTitle", { version })}</p>
          <p className="rf-prose">{t("updates.install.confirmBody", { version })}</p>
          <Row className="install-update-dialog__actions">
            <Button variant="ghost" onClick={onClose}>
              {t("actions.notNow")}
            </Button>
            <Button ref={primary} variant="primary" onClick={() => void install()}>
              {t("updates.install.confirm")}
            </Button>
          </Row>
        </>
      )}
      {phase.kind === "installing" && (
        <p className="rf-title" role="status">
          {t("updates.install.installing")}
        </p>
      )}
      {phase.kind === "failed" && (
        <>
          <p className="rf-title">{t("updates.install.failedTitle")}</p>
          <p className="rf-prose" role="alert">
            {t(`updates.install.failed.${phase.reason}`)}
          </p>
          <Row className="install-update-dialog__actions">
            <Button ref={primary} variant="primary" onClick={onClose}>
              {t("actions.close")}
            </Button>
          </Row>
        </>
      )}
    </Dialog>
  );
}
