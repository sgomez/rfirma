import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import "./AboutDialog.css";

interface AboutDialogProps {
  /** La versión que se enseña. Sale de `package.json` en tiempo de compilación. */
  version: string;
  onClose: () => void;
}

/**
 * Identidad de la aplicación, aviso de independencia y licencias.
 *
 * El requisito de esta pantalla es de **contenido**, no de estética: dice que
 * esto **no es el cliente oficial**. Una aplicación que firma ante la
 * Administración con la misma criptografía que la oficial se puede confundir
 * con ella, y esa confusión hay que deshacerla en el sitio donde la gente va a
 * preguntar qué es esto.
 *
 * El aviso va **como párrafo, sin icono ni recuadro**: es un hecho sobre el
 * proyecto, no una advertencia sobre un riesgo del usuario, y enmarcarlo como
 * alarma le daría un peso que no le corresponde.
 */
export function AboutDialog({ version, onClose }: AboutDialogProps) {
  const { t } = useTranslation();
  const [showingLicenses, setShowingLicenses] = useState(false);
  const titleId = useId();

  return (
    <div className="rf-scrim">
      <div className="rf-dialog about" role="dialog" aria-modal="true" aria-labelledby={titleId}>
        <div>
          <p className="rf-title" id={titleId}>
            {t("app.name")}
          </p>
          <p className="rf-body rf-text-muted">{t("about.version", { version })}</p>
        </div>

        <p className="rf-prose">{t("about.whatItDoes")}</p>

        <p className="rf-prose">{t("about.independence")}</p>

        <hr className="rf-divider" />

        <p className="rf-label">{t("about.licenses.title")}</p>
        {showingLicenses && (
          <ul className="about__licenses">
            <li className="rf-prose">{t("about.licenses.rfirma")}</li>
            <li className="rf-prose">{t("about.licenses.afirma")}</li>
          </ul>
        )}

        <div className="rf-row about__footer">
          <button
            type="button"
            className="rf-btn rf-btn--ghost"
            aria-expanded={showingLicenses}
            onClick={() => setShowingLicenses((shown) => !shown)}
          >
            {t("about.licenses.view")}
          </button>
          <button type="button" className="rf-btn rf-btn--primary" onClick={onClose}>
            {t("actions.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
