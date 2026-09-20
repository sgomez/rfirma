import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import "./AboutDialog.css";
import { ArrowUpIcon, CheckIcon } from "../design-system/icons";
import type { NewVersion } from "../updates/newVersion";

interface AboutDialogProps {
  /** La versión que se enseña. Sale de `package.json` en tiempo de compilación. */
  version: string;
  /**
   * Lo que contestó la comprobación de versión, o `null` si no hay una más
   * nueva —o no se ha podido preguntar—.
   */
  newVersion: NewVersion | null;
  onClose: () => void;
}

/**
 * Identidad de la aplicación, estado de la versión, aviso de independencia y
 * licencias.
 *
 * El requisito de esta pantalla es de **contenido**, no de estética: dice que
 * esto **no es el cliente oficial**. Una aplicación que firma ante la
 * Administración con la misma criptografía que la oficial se puede confundir
 * con ella, y esa confusión hay que deshacerla en el sitio donde la gente va a
 * preguntar qué es esto.
 *
 * Del estado de la versión se dice **si hay una nueva**, y nada más: quien
 * tiene la aplicación abierta ya la instaló, y repetirle las órdenes de alta
 * del repositorio no le sirve para actualizar.
 *
 * El aviso de independencia va **como párrafo, sin icono ni recuadro**: es un
 * hecho sobre el proyecto, no una advertencia sobre un riesgo del usuario, y
 * enmarcarlo como alarma le daría un peso que no le corresponde.
 *
 * El artboard dibuja las dos líneas de licencia **desplegadas**; eso es el
 * estado congelado del canvas y aquí es lo que revela «Ver las licencias».
 * Lo que se ve siempre es la dirección del repositorio, que es adónde va quien
 * quiera comprobar cualquiera de las dos.
 */
export function AboutDialog({ version, newVersion, onClose }: AboutDialogProps) {
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

        <hr className="rf-divider" />

        <div className="rf-row rf-gap-xs about__updateStatus">
          {newVersion !== null ? (
            <ArrowUpIcon size={18} />
          ) : (
            <span className="about__upToDateIcon">
              <CheckIcon size={18} strokeWidth={1.5} />
            </span>
          )}
          <p className="rf-prose">
            {newVersion !== null
              ? t("about.update.newVersion", { version: newVersion.version })
              : t("about.update.upToDate")}
          </p>
        </div>

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
