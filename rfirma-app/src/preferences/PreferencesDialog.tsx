import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLanguage } from "../i18n/LanguageProvider";
import { completeLanguages } from "../i18n/languages";
import "./PreferencesDialog.css";
import type { Preferences } from "./preferences";
import { Select } from "./Select";
import { Switch } from "./Switch";
import { THEMES } from "./theme";

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
 * El desplegable no es un `<select>` nativo sino [`Select`]: la lista que
 * despliega el elemento nativo la pinta el sistema de ventanas y no la hoja de
 * estilos, así que las opciones salían con los colores del escritorio dentro
 * de un diálogo hecho con los tokens del sistema de diseño.
 *
 * Dos diferencias con el artboard, decididas y anotadas en la ficha (ID-44):
 * el desplegable de destino **no ofrece «Junto al documento original»**, porque
 * el ADR-0011 midió que bajo el arenero eso deja un `.xdp-…` huérfano sin dar
 * error; y «Recordar mi actividad» con su «Vaciar la lista» **sí están**,
 * aunque el canvas no los dibuje, porque el ID-34 los exige.
 *
 * Y una tercera que llegó después: **el tema**, que el canvas tampoco dibuja.
 * Los tokens del bundle ya traían los dos temas y `data-theme` para forzar
 * cualquiera de ellos; lo que faltaba era dónde elegirlo.
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
          wide
          onChange={(checked) => onChange({ ...preferences, rememberVisibleSignature: checked })}
        />

        <hr className="rf-divider" />

        <Switch
          checked={preferences.rememberActivity}
          label={t("preferences.rememberActivity.label")}
          hint={t("preferences.rememberActivity.hint")}
          wide
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

        <Select
          label={t("preferences.destination.label")}
          value={preferences.destination}
          options={destinations.map((name) => ({ value: name, label: name }))}
          onChange={(destination) => onChange({ ...preferences, destination })}
        />

        <hr className="rf-divider" />

        <Select
          label={t("preferences.theme.label")}
          value={preferences.theme}
          options={THEMES.map((theme) => ({
            value: theme,
            label: t(`preferences.theme.${theme}`),
          }))}
          onChange={(theme) => onChange({ ...preferences, theme })}
        />

        <hr className="rf-divider" />

        <Select
          label={t("preferences.language.label")}
          value={language}
          options={completeLanguages().map((tag) => ({ value: tag, label: t(`languages.${tag}`) }))}
          onChange={(chosen) => void setLanguage(chosen)}
        />

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
