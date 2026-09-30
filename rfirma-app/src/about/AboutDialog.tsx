import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import "./AboutDialog.css";
import { ExternalLinkIcon, InfoIcon, NewVersionIcon, UpToDateIcon } from "../design-system/icons";
import { InstallUpdateDialog } from "../updates/InstallUpdateDialog";
import type { NewVersion, VersionCheck } from "../updates/newVersion";

interface AboutDialogProps {
  /** La versión que se enseña. Sale de `package.json` en tiempo de compilación. */
  version: string;
  /** Lo que se sabía desde el arranque, mientras no llegue la respuesta del puerto. */
  newVersion: NewVersion | null;
  versions: VersionCheck;
  /** Si se ofrece actualizar desde aquí: lo apaga `notify_new_version`. */
  offerUpdate: boolean;
  onOpenSourceCode: () => void;
  onClose: () => void;
}

/** Identidad de la aplicación, estado de la versión, licencias y aviso de independencia. */
export function AboutDialog({
  version,
  newVersion,
  versions,
  offerUpdate,
  onOpenSourceCode,
  onClose,
}: AboutDialogProps) {
  const { t } = useTranslation();
  const titleId = useId();
  const [currentVersion, setCurrentVersion] = useState(newVersion);
  const [updating, setUpdating] = useState(false);

  useEffect(() => {
    let current = true;
    versions
      .latest()
      .then((published) => {
        if (current) setCurrentVersion(published);
      })
      .catch(() => {
        // Igual que en el arranque: sin respuesta la tarjeta se queda como
        // estaba, sin un tercer estado ni mensaje de error.
      });
    return () => {
      current = false;
    };
  }, [versions]);

  return (
    <div className="rf-scrim">
      <div className="rf-dialog about" role="dialog" aria-modal="true" aria-labelledby={titleId}>
        <div className="about__header">
          <div className="about__identity">
            <p className="rf-heading about__name" id={titleId}>
              {t("app.name")}
            </p>
            <span className="rf-badge about__version">{t("about.version", { version })}</span>
          </div>
          <p className="rf-prose rf-text-muted">{t("about.whatItDoes")}</p>
          <UpdateStatus
            newVersion={currentVersion}
            offerUpdate={offerUpdate}
            onUpdate={() => setUpdating(true)}
          />
        </div>

        <dl className="about__facts">
          <dt className="rf-label">{t("about.facts.license")}</dt>
          <dd>{t("about.licenses.rfirma")}</dd>
          <dt className="rf-label">{t("about.facts.signsWith")}</dt>
          <dd className="about__stackedFact">
            <span>{t("about.licenses.afirma")}</span>
            <span className="rf-hint">{t("about.licenses.afirmaTerms")}</span>
          </dd>
          <dt className="rf-label">{t("about.facts.sourceCode")}</dt>
          <dd>
            <a
              className="about__sourceLink"
              href={`https://${t("about.repository")}`}
              onClick={(event) => {
                event.preventDefault();
                onOpenSourceCode();
              }}
            >
              <span>{t("about.repository")}</span>
              <ExternalLinkIcon size={13} />
            </a>
          </dd>
        </dl>

        <div className="rf-row about__independence">
          <span className="about__independenceIcon">
            <InfoIcon size={16} />
          </span>
          <p className="rf-prose">{t("about.independence")}</p>
        </div>

        <div className="rf-row about__footer">
          <button type="button" className="rf-btn rf-btn--primary about__close" onClick={onClose}>
            {t("actions.close")}
          </button>
        </div>
      </div>
      {updating && currentVersion !== null && (
        <InstallUpdateDialog
          newVersion={currentVersion}
          versions={versions}
          onClose={() => setUpdating(false)}
        />
      )}
    </div>
  );
}

function UpdateStatus({
  newVersion,
  offerUpdate,
  onUpdate,
}: {
  newVersion: NewVersion | null;
  offerUpdate: boolean;
  onUpdate: () => void;
}) {
  const { t } = useTranslation();

  if (newVersion !== null) {
    return (
      <div className="rf-row about__updateStatus about__updateStatus--new">
        <NewVersionIcon />
        <span>{t("about.update.newVersion", { version: newVersion.version })}</span>
        {offerUpdate && newVersion.installable && (
          <button type="button" className="rf-btn rf-btn--primary" onClick={onUpdate}>
            {t("updates.install.action")}
          </button>
        )}
      </div>
    );
  }
  return (
    <div className="rf-row about__updateStatus">
      <span className="about__upToDateIcon">
        <UpToDateIcon />
      </span>
      <span>{t("about.update.upToDate")}</span>
    </div>
  );
}
