import { type KeyboardEvent, type ReactNode, useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { classify, type NamedFailure } from "../errors/classify";
import { useLanguage } from "../i18n/LanguageProvider";
import type { Certificate } from "../signing/certificate";
import "./PreferencesView.css";
import { trapTabWithinCurrentTarget } from "./focusTrap";
import { PasswordPrompt } from "./PasswordPrompt";
import {
  AppearanceSection,
  CertificatesSection,
  GeneralSection,
  SigningSection,
} from "./PreferencesSections";
import type { Preferences } from "./preferences";

/** Las cuatro secciones del índice, en el orden en que se apilan. */
const SECTIONS = ["general", "signing", "certificates", "appearance"] as const;

export type Section = (typeof SECTIONS)[number];

/** Un ajuste que el disco no aceptó, y en qué sección se pulsó (ID-70). */
export interface SaveFailure {
  section: Section;
  /** El texto original del rechazo, para el detalle técnico del aviso. */
  detail: string;
}

interface PreferencesViewProps {
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
  /**
   * Los `.p12` que se han instalado en rFirma, y **sólo esos**: son los únicos
   * que esta pantalla puede quitar (ID-198). Un caducado sigue en la lista.
   */
  installedCertificates: readonly Certificate[];
  /**
   * Instala un `.p12` con la contraseña **del fichero** y responde si quedó
   * alguno instalado. Quien abre el selector de ficheros es el backend (ID-63),
   * así que la contraseña se teclea antes de elegirlo. Rechaza cuando el
   * fichero no se puede abrir o cuando su clave no es RSA ni de curva
   * elíptica.
   */
  onInstallCertificate: (password: string) => Promise<boolean>;
  /** Quita un `.p12` instalado, por el asa de su fila. */
  onRemoveCertificate: (id: string) => Promise<void>;
  onClose: () => void;
}

/**
 * Los ajustes de la aplicación: una **vista del cuerpo**, como
 * [`StatusView`](../status/StatusView.tsx), que sustituye la bandeja, el
 * visor y el panel bajo la cabecera, que se queda intacta detrás con su
 * estado de documento (ID-352).
 *
 * No es un diálogo ni una ruta de un router: con guardado automático y
 * `Cerrar` como única salida no hay ningún estado al que navegar ni nada que
 * confirmar, así que `Escape` sigue valiendo y `Cmd+,` sigue prometiendo lo
 * que abre. **El foco no se atrapa aquí dentro**: el menú de la cabecera se
 * abre y funciona con Preferencias delante, tanto por clic como por
 * teclado — a diferencia de los dos modales que se ponen encima de esta
 * pantalla, que sí lo atrapan.
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
 * **El índice es el patrón ARIA de pestañas** (`tablist` / `tab` / `tabpanel`):
 * solo el panel activo está en pantalla, con su propio scroll, y se entra
 * siempre en *General*. Las flechas arriba/abajo mueven la selección;
 * `Escape` cierra Preferencias entera por encima de todo, tanto si el foco
 * está en una pestaña como en cualquier otro control.
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
 *
 * Cada sección del índice es un componente propio de
 * [`PreferencesSections`](./PreferencesSections.tsx); esta vista solo guarda
 * el estado, gestiona los dos modales y reparte los cambios.
 */
export function PreferencesView({
  preferences,
  onChooseDestination,
  onChange,
  onForgetActivity,
  installedCertificates,
  onInstallCertificate,
  onRemoveCertificate,
  onClose,
}: PreferencesViewProps) {
  const { t } = useTranslation();
  const { language, setLanguage } = useLanguage();
  const [confirmingPurge, setConfirmingPurge] = useState(false);
  const [saveFailure, setSaveFailure] = useState<SaveFailure | null>(null);
  const [forgetFailure, setForgetFailure] = useState<string | null>(null);
  const [askingPassword, setAskingPassword] = useState(false);
  const [certificateFailure, setCertificateFailure] = useState<NamedFailure | null>(null);
  const [current, setCurrent] = useState<Section>("general");
  const titleId = useId();
  const confirm = useRef<HTMLDivElement>(null);
  const password = useRef<HTMLDivElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const tabs = useRef(new Map<Section, HTMLElement | null>());

  // `Escape` cierra Preferencias desde cualquier sitio, tenga el foco donde lo
  // tenga: como `StatusView`, no depende de que el foco esté dentro de la
  // pantalla. Un `Escape` que ya haya cerrado el menú de la cabecera llega
  // aquí con `defaultPrevented`, así que cerrar el menú no cierra además
  // Preferencias.
  useEffect(() => {
    const onWindowKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      event.preventDefault();
      if (askingPassword) {
        setAskingPassword(false);
        return;
      }
      if (confirmingPurge) {
        setConfirmingPurge(false);
        return;
      }
      onClose();
    };
    window.addEventListener("keydown", onWindowKeyDown);
    return () => window.removeEventListener("keydown", onWindowKeyDown);
  }, [askingPassword, confirmingPurge, onClose]);

  // La confirmación es a su vez un diálogo modal, así que cuando se pone
  // delante el foco entra en ella y el tabulador deja de pasear por los
  // ajustes que quedan detrás: `aria-modal` lo promete a quien escucha, y
  // esto es lo que lo cumple para quien teclea.
  useEffect(() => {
    if (confirmingPurge) confirm.current?.focus();
  }, [confirmingPurge]);

  // Lo mismo con el diálogo de la contraseña del `.p12`, que es el otro modal
  // que se pone delante de esta pantalla.
  useEffect(() => {
    if (askingPassword) password.current?.focus();
  }, [askingPassword]);

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
    void change("general", () => onChange({ ...preferences, rememberActivity: true }));
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
    await change("general", () => onChange({ ...preferences, rememberActivity: false }));
    await forget();
  };

  /**
   * Mete un `.p12` con la contraseña que se acaba de teclear.
   *
   * El selector de ficheros lo abre el backend **después** (ID-63), así que
   * cerrarlo sin elegir nada devuelve `false` y no es un fallo: deja la lista
   * como estaba y no pinta ningún aviso. Lo que sí lo es —la contraseña que no
   * abre el fichero, la clave que no es RSA ni de curva elíptica— se cuenta en
   * la sección.
   */
  const install = async (typed: string) => {
    setAskingPassword(false);
    setCertificateFailure(null);
    try {
      await onInstallCertificate(typed);
    } catch (thrown) {
      setCertificateFailure(classify(thrown));
    }
  };

  const remove = async (certificate: Certificate) => {
    setCertificateFailure(null);
    try {
      await onRemoveCertificate(certificate.id);
    } catch (thrown) {
      setCertificateFailure(classify(thrown));
    }
  };

  /** Enseña la sección elegida y le pasa el foco: por clic o por flecha, es la misma. */
  const show = (section: Section) => {
    setCurrent(section);
    // El panel vuelve a empezar por arriba: cambiar de sección no hereda el
    // desplazamiento en el que se había quedado la anterior, que ya no está
    // en pantalla.
    panel.current?.scrollTo?.({ top: 0 });
    tabs.current.get(section)?.focus();
  };

  /**
   * Arriba/abajo mueven la pestaña activa, con vuelta al llegar a un extremo.
   * El índice es vertical, así que no son las flechas izquierda/derecha del
   * patrón horizontal.
   */
  const onTabsKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    const from = SECTIONS.indexOf(current);
    const step = event.key === "ArrowDown" ? 1 : -1;
    // El índice cae siempre dentro de `SECTIONS`: el módulo lo envuelve.
    show(SECTIONS[(from + step + SECTIONS.length) % SECTIONS.length] as Section);
  };

  const registerTab = (section: Section) => (element: HTMLElement | null) => {
    tabs.current.set(section, element);
  };

  const tabId = (section: Section) => `${titleId}-tab-${section}`;

  const panels: Record<Section, ReactNode> = {
    general: (
      <GeneralSection
        titleId={titleId}
        saveFailure={saveFailure}
        rememberActivity={preferences.rememberActivity}
        onRememberActivityChange={rememberActivity}
        onForgetClick={() => void forget()}
        forgetFailure={forgetFailure}
        notifyNewVersion={preferences.notifyNewVersion}
        onNotifyNewVersionChange={(checked) =>
          void change("general", () => onChange({ ...preferences, notifyNewVersion: checked }))
        }
      />
    ),
    signing: (
      <SigningSection
        titleId={titleId}
        saveFailure={saveFailure}
        preferences={preferences}
        onRememberVisibleSignatureChange={(checked) =>
          void change("signing", () =>
            onChange({ ...preferences, rememberVisibleSignature: checked }),
          )
        }
        onChooseDestinationClick={() => void change("signing", onChooseDestination)}
        onConsentCountdownChange={(checked) =>
          void change("signing", () => onChange({ ...preferences, consentCountdown: checked }))
        }
        onHonourAutomaticSelectionChange={(checked) =>
          void change("signing", () =>
            onChange({ ...preferences, honourAutomaticSelection: checked }),
          )
        }
      />
    ),
    certificates: (
      <CertificatesSection
        titleId={titleId}
        certificateFailure={certificateFailure}
        installedCertificates={installedCertificates}
        onAddClick={() => {
          setCertificateFailure(null);
          setAskingPassword(true);
        }}
        onRemoveClick={(certificate) => void remove(certificate)}
      />
    ),
    appearance: (
      <AppearanceSection
        titleId={titleId}
        saveFailure={saveFailure}
        theme={preferences.theme}
        onThemeChange={(theme) =>
          void change("appearance", () => onChange({ ...preferences, theme }))
        }
        language={language}
        onLanguageChange={(chosen) => void change("appearance", () => setLanguage(chosen))}
      />
    ),
  };

  return (
    <section className="preferences" aria-labelledby={titleId}>
      <nav className="preferences__index" aria-label={t("preferences.sections.label")}>
        <p className="rf-title" id={titleId}>
          {t("preferences.title")}
        </p>
        <div
          className="preferences__tablist"
          role="tablist"
          aria-orientation="vertical"
          onKeyDown={onTabsKeyDown}
        >
          {SECTIONS.map((section) => (
            <button
              key={section}
              type="button"
              id={tabId(section)}
              role="tab"
              ref={registerTab(section)}
              className={
                section === current
                  ? "rf-btn preferences__section preferences__section--current"
                  : "rf-btn preferences__section rf-text-muted"
              }
              aria-selected={section === current}
              aria-controls={`${titleId}-panel`}
              tabIndex={section === current ? 0 : -1}
              onClick={() => show(section)}
            >
              {t(`preferences.sections.${section}`)}
            </button>
          ))}
        </div>
      </nav>

      <div
        className="preferences__content"
        role="tabpanel"
        id={`${titleId}-panel`}
        aria-labelledby={tabId(current)}
        // biome-ignore lint/a11y/noNoninteractiveTabindex: el patrón ARIA de pestañas exige que el tabpanel se pueda enfocar con teclado tras elegir una pestaña.
        tabIndex={0}
        ref={panel}
      >
        <div className="preferences__column">
          <div className="preferences__section-body">{panels[current]}</div>
        </div>
      </div>

      <div className="preferences__footer">
        <button type="button" className="rf-btn rf-btn--primary" onClick={onClose}>
          {t("actions.close")}
        </button>
      </div>

      {askingPassword && (
        <div className="rf-scrim">
          <PasswordPrompt
            ref={password}
            labelledBy={`${titleId}-password`}
            onCancel={() => setAskingPassword(false)}
            onSubmit={(typed) => void install(typed)}
          />
        </div>
      )}

      {confirmingPurge && (
        <div className="rf-scrim">
          <div
            className="rf-dialog preferences__confirm"
            role="dialog"
            aria-modal="true"
            tabIndex={-1}
            ref={confirm}
            aria-labelledby={`${titleId}-confirm`}
            onKeyDown={trapTabWithinCurrentTarget}
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
    </section>
  );
}
