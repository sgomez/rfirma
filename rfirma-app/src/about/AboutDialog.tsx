//! El diálogo Acerca de: identidad de la aplicación, estado de la versión, licencias y aviso de independencia.

import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import "./AboutDialog.css";
import { Badge } from "../design-system/Badge";
import { Button } from "../design-system/Button";
import { Dialog } from "../design-system/Dialog";
import { ExternalLinkIcon, InfoIcon, NewVersionIcon, UpToDateIcon } from "../design-system/icons";
import { Row } from "../design-system/Row";
import { Stack } from "../design-system/Stack";
import { InstallUpdateDialog } from "../updates/InstallUpdateDialog";
import type { NewVersion, VersionCheck } from "../updates/newVersion";

interface AboutDialogProps {
  /** La versión del binario, resuelta una vez en `main.tsx`. */
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
  const [currentVersion, setCurrentVersion] = useState(newVersion);
  const [updating, setUpdating] = useState(false);
  const close = useRef<HTMLButtonElement>(null);

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
    <>
      <Dialog label={t("app.name")} onClose={onClose} primary={close} className="about">
        <div className="about__header">
          <div className="about__identity">
            <p className="rf-heading about__name">{t("app.name")}</p>
            <Badge className="about__version">{t("about.version", { version })}</Badge>
          </div>
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

        <Row className="about__independence">
          <span className="about__independenceIcon">
            <InfoIcon size={16} />
          </span>
          <Stack className="about__independenceText">
            <p className="rf-title">{t("about.independenceLead")}</p>
            <p className="rf-prose">{t("about.independence")}</p>
          </Stack>
        </Row>

        <Row className="about__footer">
          <Button ref={close} variant="primary" className="about__close" onClick={onClose}>
            {t("actions.close")}
          </Button>
        </Row>
      </Dialog>
      {updating && currentVersion !== null && (
        <InstallUpdateDialog
          newVersion={currentVersion}
          versions={versions}
          onClose={() => setUpdating(false)}
        />
      )}
    </>
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
      <Row className="about__updateStatus about__updateStatus--new">
        <NewVersionIcon />
        <span>{t("updates.newVersion", { version: newVersion.version })}</span>
        {offerUpdate && newVersion.installable && (
          <Button variant="primary" onClick={onUpdate}>
            {t("updates.install.action")}
          </Button>
        )}
      </Row>
    );
  }
  return (
    <Row className="about__updateStatus">
      <span className="about__upToDateIcon">
        <UpToDateIcon />
      </span>
      <span>{t("about.update.upToDate")}</span>
    </Row>
  );
}
