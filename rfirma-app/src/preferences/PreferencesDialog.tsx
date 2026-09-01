import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLanguage } from "../i18n/LanguageProvider";
import { completeLanguages, isLanguageTag } from "../i18n/languages";
import "./PreferencesDialog.css";
import type { Preferences } from "./preferences";
import { Switch } from "./Switch";

interface PreferencesDialogProps {
  preferences: Preferences;
  /**
   * Las carpetas de destino que se pueden elegir, **por su nombre**. Bajo el
   * arenero hay exactamente una, la de documentos del usuario: *junto al
   * documento original* no existe ahí, y enseñarlo atenuado sería contarle al
   * usuario nuestros problemas de empaquetado (ADR-0011).
   */
  destinations: readonly string[];
  onChange: (preferences: Preferences) => void;
  /** Olvida los recientes y el certificado. */
  onForgetActivity: () => void;
  onClose: () => void;
}

/**
 * Los ajustes de la aplicación, sobre la ventana y sin desmontarla.
 *
 * **Los cambios se aplican al hacerlos**: no hay «Guardar» ni «Cancelar», solo
 * «Cerrar». El único paso intermedio es apagar «Recordar mi actividad», que
 * pide confirmación porque **borra** lo ya recordado (ID-34): conservar el
 * fichero mientras la preferencia dice que no se recuerda nada incumpliría lo
 * que promete el rótulo.
 *
 * El idioma sale de `LanguageProvider` y no de estos ajustes porque ya vivía
 * ahí, y solo se ofrecen los catálogos **completos**: caer al castellano a
 * mitad de pantalla no es una degradación aceptable (ADR-0009).
 *
 * Dos diferencias con el artboard, decididas y anotadas en la ficha (ID-44):
 * el desplegable de destino **no ofrece «Junto al documento original»**, porque
 * el ADR-0011 midió que bajo el arenero eso deja un `.xdp-…` huérfano sin dar
 * error; y «Recordar mi actividad» con su «Vaciar la lista» **sí están**,
 * aunque el canvas no los dibuje, porque el ID-34 los exige.
 */
export function PreferencesDialog({
  preferences,
  destinations,
  onChange,
  onForgetActivity,
  onClose,
}: PreferencesDialogProps) {
  const { t } = useTranslation();
  const { language, setLanguage } = useLanguage();
  const [confirmingPurge, setConfirmingPurge] = useState(false);
  const titleId = useId();
  const destinationId = useId();
  const languageId = useId();

  const rememberActivity = (checked: boolean) => {
    if (!checked) {
      setConfirmingPurge(true);
      return;
    }
    onChange({ ...preferences, rememberActivity: true });
  };

  const purge = () => {
    setConfirmingPurge(false);
    onChange({ ...preferences, rememberActivity: false });
    onForgetActivity();
  };

  return (
    <div className="rf-scrim">
      <div
        className="rf-dialog preferences"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <p className="rf-title" id={titleId}>
          {t("preferences.title")}
        </p>

        <Switch
          checked={preferences.rememberVisibleSignature}
          label={t("preferences.rememberVisibleSignature.label")}
          hint={t("preferences.rememberVisibleSignature.hint")}
          onChange={(checked) => onChange({ ...preferences, rememberVisibleSignature: checked })}
        />

        <hr className="rf-divider" />

        <Switch
          checked={preferences.rememberActivity}
          label={t("preferences.rememberActivity.label")}
          hint={t("preferences.rememberActivity.hint")}
          onChange={rememberActivity}
        />
        <button
          type="button"
          className="rf-btn rf-btn--secondary preferences__clear"
          onClick={onForgetActivity}
        >
          {t("preferences.rememberActivity.clear")}
        </button>
        {confirmingPurge && (
          <div className="rf-card preferences__confirm">
            <p className="rf-prose">{t("preferences.rememberActivity.confirm.body")}</p>
            <div className="rf-row preferences__confirm-actions">
              <button
                type="button"
                className="rf-btn rf-btn--ghost"
                onClick={() => setConfirmingPurge(false)}
              >
                {t("actions.cancel")}
              </button>
              <button type="button" className="rf-btn rf-btn--secondary" onClick={purge}>
                {t("preferences.rememberActivity.confirm.accept")}
              </button>
            </div>
          </div>
        )}

        <hr className="rf-divider" />

        <div className="rf-field">
          <label className="rf-label" htmlFor={destinationId}>
            {t("preferences.destination.label")}
          </label>
          <select
            className="rf-input"
            id={destinationId}
            value={preferences.destination}
            onChange={(event) => onChange({ ...preferences, destination: event.target.value })}
          >
            {destinations.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </div>

        <hr className="rf-divider" />

        <div className="rf-field">
          <label className="rf-label" htmlFor={languageId}>
            {t("preferences.language.label")}
          </label>
          <select
            className="rf-input"
            id={languageId}
            value={language}
            onChange={(event) => {
              const chosen = event.target.value;
              if (isLanguageTag(chosen)) void setLanguage(chosen);
            }}
          >
            {completeLanguages().map((tag) => (
              <option key={tag} value={tag}>
                {t(`languages.${tag}`)}
              </option>
            ))}
          </select>
        </div>

        <hr className="rf-divider" />

        <div className="preferences__footer">
          <button type="button" className="rf-btn rf-btn--primary" onClick={onClose}>
            {t("actions.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
