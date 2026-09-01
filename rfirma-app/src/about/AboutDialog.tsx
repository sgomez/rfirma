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
 *
 * El artboard dibuja las dos líneas de licencia **desplegadas**; eso es el
 * estado congelado del canvas y aquí es lo que revela «Ver las licencias»
 * (ID-43). Lo que se ve siempre es la dirección del repositorio, que es adónde
 * va quien quiera comprobar cualquiera de las dos.
 */
export function AboutDialog({ version, onClose }: AboutDialogProps) {
  const { t } = useTranslation();
  const [showingLicenses, setShowingLicenses] = useState(false);
  const titleId = useId();

  return (
    <div className="rf-scrim">
      <div className="rf-dialog about" role="dialog" aria-modal="true" aria-labelledby={titleId}>
        <div className="about__identity">
          <p className="rf-heading about__name" id={titleId}>
            {t("app.name")}
          </p>
          <p className="rf-body rf-text-muted">{t("about.version", { version })}</p>
        </div>

        <p className="rf-prose">{t("about.whatItDoes")}</p>

        <p className="rf-prose">{t("about.independence")}</p>

        <hr className="rf-divider" />

        <div className="about__licenses">
          {showingLicenses && (
            <>
              <p className="rf-body rf-text-muted">{t("about.licenses.afirma")}</p>
              <p className="rf-body rf-text-muted">{t("about.licenses.rfirma")}</p>
            </>
          )}
          <p className="rf-body">{t("about.repository")}</p>
        </div>

        <hr className="rf-divider" />

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
