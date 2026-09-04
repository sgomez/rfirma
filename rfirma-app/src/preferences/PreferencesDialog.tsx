import { type KeyboardEvent, useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { classify } from "../errors/classify";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useLanguage } from "../i18n/LanguageProvider";
import { LANGUAGES } from "../i18n/languages";
import "./PreferencesDialog.css";
import type { Preferences } from "./preferences";
import { Select } from "./Select";
import { Switch } from "./Switch";
import { THEMES } from "./theme";

/** Las tres secciones del índice, en el orden en que se apilan (ID-69). */
const SECTIONS = ["signing", "privacy", "appearance"] as const;

type Section = (typeof SECTIONS)[number];

/** Lo que puede recibir el foco dentro de la pantalla. */
const FOCUSABLE = 'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';

/**
 * El tabulador da la vuelta dentro del diálogo en vez de salirse a la cabecera
 * que queda detrás. Es lo que distingue un diálogo modal de una región más de
 * la ventana, y `aria-modal` lo promete al lector de pantalla pero no lo
 * cumple por sí solo: el foco del teclado lo mueve el navegador.
 */
function trapFocus(screen: HTMLElement | null, event: KeyboardEvent<HTMLDivElement>) {
  if (screen === null) return;
  const focusable = [...screen.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
    (element) => !element.hasAttribute("disabled"),
  );
  const first = focusable.at(0);
  const last = focusable.at(-1);
  if (first === undefined || last === undefined) return;
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

/** Un ajuste que el disco no aceptó, y en qué sección se pulsó (ID-70). */
interface SaveFailure {
  section: Section;
  /** El texto original del rechazo, para el detalle técnico del aviso. */
  detail: string;
}

interface PreferencesDialogProps {
  preferences: Preferences;
  /**
   * Abre el **selector de directorio** del sistema y guarda lo que conceda.
   * Rechaza si el ajuste no se pudo guardar, como cualquier otro (ID-70).
   */
  onChooseDestination: () => Promise<void>;
  /**
   * Guarda el ajuste. **Rechaza** si el disco no lo acepta, y quien lo llama
   * ya ha repuesto el valor anterior: el rechazo no es para deshacer nada,
   * sino para tener qué enseñar y dónde (ID-70).
   */
  onChange: (preferences: Preferences) => Promise<void>;
  /** Olvida los recientes y el certificado. Rechaza si el borrado falla. */
  onForgetActivity: () => Promise<void>;
  onClose: () => void;
}

/**
 * Los ajustes de la aplicación: un diálogo **a pantalla completa** por debajo
 * de la cabecera, que se queda intacta detrás con su estado de documento
 * (ID-68).
 *
 * Es un diálogo y no una ruta de un router: con guardado automático y `Cerrar`
 * como única salida no hay ningún estado al que navegar ni nada que confirmar,
 * así que `Escape` sigue valiendo y `Cmd+,` sigue prometiendo lo que abre. Lo
 * único que cambia respecto al modal de 480 px es el tamaño: con cinco ajustes
 * ya iba justo y lo que viene no cabe.
 *
 * **Los cambios se aplican al hacerlos**: no hay «Guardar» ni «Cancelar», solo
 * «Cerrar», y va en un **pie fijo** porque en una pantalla que se desplaza un
 * botón de cierre que se va con el desplazamiento es un botón que no está
 * (ID-69).
 *
 * El único paso intermedio es apagar «Recordar mi actividad», que pide
 * confirmación en un `.rf-dialog` pequeño **encima** de la pantalla porque
 * **borra** lo ya recordado (ID-34, ID-71): el interruptor no se mueve hasta
 * que se confirma.
 *
 * **Los dos fallos se pintan en su sección** (ID-70): el de guardar el ajuste,
 * donde se pulsó; el de vaciar la lista, siempre en *Privacidad* y pegado a su
 * botón. No hay un aviso común arriba: con tres secciones obligaría a leer el
 * texto para saber qué se rompió.
 *
 * El idioma sale de `LanguageProvider` y no de estos ajustes porque ya vivía
 * ahí, y solo se ofrecen los catálogos **completos**: caer al castellano a
 * mitad de pantalla no es una degradación aceptable (ADR-0009). Su guardado
 * puede fallar como el de cualquier otro ajuste, así que pasa por el mismo
 * aviso, en *Apariencia*.
 *
 * El desplegable no es un `<select>` nativo sino [`Select`]: la lista que
 * despliega el elemento nativo la pinta el sistema de ventanas y no la hoja de
 * estilos, así que las opciones salían con los colores del escritorio dentro
 * de una pantalla hecha con los tokens del sistema de diseño.
 *
 * **La carpeta de destino no es un desplegable**: lo fue, con una sola opción
 * dentro, que es un control que finge elegir. Es una fila con el **nombre** de
 * la carpeta —no su ruta— y un botón que abre el selector de directorio del
 * sistema, que devuelve exactamente ese último segmento en los cuatro canales
 * (ID-65, ADR-0011).
 */
export function PreferencesDialog({
  preferences,
  onChooseDestination,
  onChange,
  onForgetActivity,
  onClose,
}: PreferencesDialogProps) {
  const { t } = useTranslation();
  const { language, setLanguage } = useLanguage();
  const [confirmingPurge, setConfirmingPurge] = useState(false);
  const [saveFailure, setSaveFailure] = useState<SaveFailure | null>(null);
  const [forgetFailure, setForgetFailure] = useState<string | null>(null);
  const [current, setCurrent] = useState<Section>("signing");
  const titleId = useId();
  const screen = useRef<HTMLDivElement>(null);
  const confirm = useRef<HTMLDivElement>(null);
  const sections = useRef(new Map<Section, HTMLElement | null>());

  // El foco entra en la pantalla al abrirla, que es lo que la hace un diálogo
  // y no una región más de la ventana: sin esto el teclado seguiría donde
  // estaba —en el menú de la cabecera— y `Escape` cerraría lo que hay detrás.
  useEffect(() => {
    screen.current?.focus();
  }, []);

  // La confirmación es a su vez un diálogo modal, así que cuando se pone
  // delante el foco entra en ella y el tabulador deja de pasear por los
  // ajustes que quedan detrás: `aria-modal` lo promete a quien escucha, y
  // esto es lo que lo cumple para quien teclea.
  useEffect(() => {
    if (confirmingPurge) confirm.current?.focus();
  }, [confirmingPurge]);

  /**
   * Guarda un ajuste y, si el disco lo rechaza, deja el aviso **en la sección
   * donde se pulsó**. Quien nos llama ya ha repuesto el valor anterior, así que
   * el control vuelve solo: aquí solo se recoge qué contar.
   */
  const change = async (section: Section, save: () => Promise<void>) => {
    setSaveFailure(null);
    try {
      await save();
    } catch (thrown) {
      setSaveFailure({ section, detail: classify(thrown).detail });
    }
  };

  const rememberActivity = (checked: boolean) => {
    if (!checked) {
      setConfirmingPurge(true);
      return;
    }
    void change("privacy", () => onChange({ ...preferences, rememberActivity: true }));
  };

  /** Vaciar la lista sin apagar el interruptor: «hoy no, mañana sí». */
  const forget = async () => {
    setForgetFailure(null);
    try {
      await onForgetActivity();
    } catch (thrown) {
      setForgetFailure(classify(thrown).detail);
    }
  };

  /**
   * «Borrar y apagar», en ese orden y hasta el final: la lista se vacía
   * **aunque el ajuste no se haya podido guardar**. Lo que se acaba de
   * confirmar es un borrado, y no hacerlo porque el interruptor no cupo en el
   * disco dejaría los recientes a la vista después de haber dicho que sí.
   *
   * De ahí la pareja de avisos que puede salir: el de *Privacidad* dice que
   * «Recordar mi actividad» sigue encendido —y es verdad, el ajuste no se
   * guardó— mientras la lista ya está vacía. Cada uno cuenta lo suyo, y lo
   * único que sobra es la promesa que no se cumple, que no la hay.
   */
  const purge = async () => {
    setConfirmingPurge(false);
    await change("privacy", () => onChange({ ...preferences, rememberActivity: false }));
    await forget();
  };

  /**
   * `Escape` cierra la pantalla, y cierra antes la confirmación si está
   * delante. Un `Escape` que ya haya consumido un desplegable llega aquí con
   * `defaultPrevented`, así que abrir una lista y cerrarla no cierra además
   * los ajustes.
   */
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Tab") {
      trapFocus(confirmingPurge ? confirm.current : screen.current, event);
      return;
    }
    if (event.key !== "Escape" || event.defaultPrevented) return;
    event.preventDefault();
    if (confirmingPurge) {
      setConfirmingPurge(false);
      return;
    }
    onClose();
  };

  const show = (section: Section) => {
    setCurrent(section);
    // `scrollIntoView` no existe en jsdom; en la ventana de verdad es lo que
    // lleva la columna a la sección elegida.
    sections.current.get(section)?.scrollIntoView?.({ block: "start" });
  };

  /** El aviso de guardado de una sección, o nada si el fallo fue en otra. */
  const saveNotice = (section: Section) =>
    saveFailure?.section === section ? (
      <ErrorNotice situation="settingNotSaved" technicalDetail={saveFailure.detail} />
    ) : null;

  const heading = (section: Section) => (
    <>
      <p className="rf-title preferences__heading" id={`${titleId}-${section}`}>
        {t(`preferences.sections.${section}`)}
      </p>
      <hr className="rf-divider" />
    </>
  );

  const register = (section: Section) => (element: HTMLElement | null) => {
    sections.current.set(section, element);
  };

  return (
    <div
      className="preferences"
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      tabIndex={-1}
      ref={screen}
      onKeyDown={onKeyDown}
    >
      <nav className="preferences__index" aria-label={t("preferences.sections.label")}>
        <p className="rf-title" id={titleId}>
          {t("preferences.title")}
        </p>
        {SECTIONS.map((section) => (
          <button
            key={section}
            type="button"
            className={
              section === current
                ? "rf-btn preferences__section preferences__section--current"
                : "rf-btn preferences__section rf-text-muted"
            }
            aria-current={section === current || undefined}
            onClick={() => show(section)}
          >
            {t(`preferences.sections.${section}`)}
          </button>
        ))}
      </nav>

      <div className="preferences__content">
        <div className="preferences__column">
          <section
            className="preferences__section-body"
            aria-labelledby={`${titleId}-signing`}
            ref={register("signing")}
          >
            {heading("signing")}
            <Switch
              checked={preferences.rememberVisibleSignature}
              label={t("preferences.rememberVisibleSignature.label")}
              hint={t("preferences.rememberVisibleSignature.hint")}
              wide
              onChange={(checked) =>
                void change("signing", () =>
                  onChange({ ...preferences, rememberVisibleSignature: checked }),
                )
              }
            />
            <div className="preferences__destination">
              <p className="rf-label" id={`${titleId}-destination`}>
                {t("preferences.destination.label")}
              </p>
              {preferences.offersOriginalFolder && (
                <p className="rf-prose preferences__destination-note">
                  {t("preferences.destination.nextToOriginal")}
                </p>
              )}
              <div className="rf-row rf-gap-sm preferences__destination-row">
                {preferences.offersOriginalFolder && (
                  <span className="rf-prose preferences__destination-mode-label">
                    {t("preferences.destination.inThisFolder")}
                  </span>
                )}
                <p className="rf-prose preferences__destination-folder">
                  {preferences.destination}
                </p>
                <button
                  type="button"
                  className="rf-btn rf-btn--secondary"
                  onClick={() => void change("signing", onChooseDestination)}
                >
                  {t("preferences.destination.change")}
                </button>
              </div>
            </div>
            {saveNotice("signing")}
          </section>

          <section
            className="preferences__section-body"
            aria-labelledby={`${titleId}-privacy`}
            ref={register("privacy")}
          >
            {heading("privacy")}
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
              onClick={() => void forget()}
            >
              {t("preferences.rememberActivity.clear")}
            </button>
            {forgetFailure !== null && (
              <ErrorNotice situation="activityNotForgotten" technicalDetail={forgetFailure} />
            )}
            <Switch
              checked={preferences.notifyNewVersion}
              label={t("preferences.notifyNewVersion.label")}
              wide
              onChange={(checked) =>
                void change("privacy", () =>
                  onChange({ ...preferences, notifyNewVersion: checked }),
                )
              }
            />
            {saveNotice("privacy")}
          </section>

          <section
            className="preferences__section-body"
            aria-labelledby={`${titleId}-appearance`}
            ref={register("appearance")}
          >
            {heading("appearance")}
            <Select
              label={t("preferences.theme.label")}
              value={preferences.theme}
              options={THEMES.map((theme) => ({
                value: theme,
                label: t(`preferences.theme.${theme}`),
              }))}
              onChange={(theme) =>
                void change("appearance", () => onChange({ ...preferences, theme }))
              }
            />
            <Select
              label={t("preferences.language.label")}
              value={language}
              options={LANGUAGES.map((tag) => ({
                value: tag,
                label: t(`languages.${tag}`),
              }))}
              onChange={(chosen) => void change("appearance", () => setLanguage(chosen))}
            />
            {saveNotice("appearance")}
          </section>
        </div>
      </div>

      <div className="preferences__footer">
        <button type="button" className="rf-btn rf-btn--primary" onClick={onClose}>
          {t("actions.close")}
        </button>
      </div>

      {confirmingPurge && (
        <div className="rf-scrim">
          <div
            className="rf-dialog preferences__confirm"
            role="dialog"
            aria-modal="true"
            tabIndex={-1}
            ref={confirm}
            aria-labelledby={`${titleId}-confirm`}
          >
            <p className="rf-prose" id={`${titleId}-confirm`}>
              {t("preferences.rememberActivity.confirm.body")}
            </p>
            <div className="rf-row preferences__confirm-actions">
              <button
                type="button"
                className="rf-btn rf-btn--ghost"
                onClick={() => setConfirmingPurge(false)}
              >
                {t("actions.cancel")}
              </button>
              <button type="button" className="rf-btn rf-btn--primary" onClick={() => void purge()}>
                {t("preferences.rememberActivity.confirm.accept")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
