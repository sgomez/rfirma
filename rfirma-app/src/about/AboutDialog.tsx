import { useId } from "react";
import { useTranslation } from "react-i18next";
import "./AboutDialog.css";
import { ExternalLinkIcon, InfoIcon, NewVersionIcon, UpToDateIcon } from "../design-system/icons";
import type { NewVersion } from "../updates/newVersion";

interface AboutDialogProps {
  /** La versión que se enseña. Sale de `package.json` en tiempo de compilación. */
  version: string;
  /**
   * Lo que contestó la comprobación de versión, o `null` si no hay una más
   * nueva —o no se ha podido preguntar—.
   */
  newVersion: NewVersion | null;
  onOpenSourceCode: () => void;
  onClose: () => void;
}

/** Identidad de la aplicación, estado de la versión, licencias y aviso de independencia. */
export function AboutDialog({ version, newVersion, onOpenSourceCode, onClose }: AboutDialogProps) {
  const { t } = useTranslation();
  const titleId = useId();

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
          <UpdateStatus newVersion={newVersion} />
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
    </div>
  );
}

function UpdateStatus({ newVersion }: { newVersion: NewVersion | null }) {
  const { t } = useTranslation();

  if (newVersion !== null) {
    return (
      <div className="rf-row about__updateStatus about__updateStatus--new">
        <NewVersionIcon />
        <span>{t("about.update.newVersion", { version: newVersion.version })}</span>
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
