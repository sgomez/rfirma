import { useState } from "react";
import { useTranslation } from "react-i18next";
import { CopyIcon } from "../design-system/icons";
import { CHROME_LOCAL_NETWORK_SETTINGS } from "./errand";
import { SedeBody } from "./SedeFrame";

interface SedeWaitingProps {
  /** En cuál de los dos lados del único umbral está el reloj de la ventana. */
  moment: "connecting" | "unreachable";
  onInstallLocalCa: () => void;
  onCancel: () => void;
}

/** Cuál de las dos recetas se está leyendo. */
type Browser = "chrome" | "firefox";

/**
 * **1 · Esperando el canal.** Lo que se ve mientras el canal no se abre, y lo
 * que se ve cuando ya no va a abrirse.
 *
 * Un solo umbral, y **nunca se cierra sola**: cerrar aquí es abandonar el
 * trámite, y eso lo decide la persona.
 *
 * El camino de reparación **no diagnostica**. rFirma no puede saber si el
 * permiso se denegó, así que no hay mensaje de causa: hay un conmutador entre
 * dos recetas y la persona elige la suya. Sólo texto, sin capturas — el aviso
 * del navegador se describe **por su forma** y se cita el botón `Permitir`.
 *
 * La frase obligatoria vive en el **pie** y no como tercer paso de cada receta:
 * es el desenlace de las dos, y ahí se queda visible aunque el cuerpo se
 * desplace. `Reintentar` es un botón **de la sede**, y por eso esta ventana no
 * lo tiene.
 */
export function SedeWaiting({ moment, onInstallLocalCa, onCancel }: SedeWaitingProps) {
  const { t } = useTranslation();
  const [browser, setBrowser] = useState<Browser>("chrome");
  const unreachable = moment === "unreachable";

  const localCa = (
    <div className="rf-row rf-gap-xs sede-waiting__ca">
      <p className="rf-body sede-waiting__ca-text">{t("sede.repair.caMissing")}</p>
      {/* `--primary`, y es el único de la pantalla: la tabla «Estados» de la
          ficha da instalar la CA como la **acción principal** de este estado.
          Sin ella el navegador ni llega a preguntar por el permiso. */}
      <button type="button" className="rf-btn rf-btn--primary" onClick={onInstallLocalCa}>
        {t("sede.repair.installCa")}
      </button>
    </div>
  );

  return (
    <SedeBody
      footer={
        <>
          {unreachable && <p className="rf-hint sede-waiting__retry">{t("sede.repair.retry")}</p>}
          <div className="sede-window__spacer" />
          <button type="button" className="rf-btn rf-btn--ghost" onClick={onCancel}>
            {unreachable ? t("actions.close") : t("actions.cancel")}
          </button>
        </>
      }
    >
      <div className={`rf-stack sede-waiting${unreachable ? " sede-waiting--repair" : ""}`}>
        <p className="rf-title sede-waiting__title">
          {unreachable ? t("sede.unreachable.title") : t("sede.waiting.title")}
        </p>
        <p className="rf-prose rf-text-muted">
          {unreachable ? t("sede.unreachable.lead") : t("sede.waiting.lead")}
        </p>

        {unreachable && (
          <>
            {/* En Chrome la CA va **primero**: sin ella el navegador ni llega a
                preguntar por el permiso, así que la receta sobra hasta que esté.
                En Firefox va después. */}
            {browser === "chrome" && localCa}

            {/* Dos botones normales y no `tablist`: no gobiernan ningún
                `tabpanel`, y unas pestañas que no controlan nada le mienten al
                lector de pantalla. `aria-pressed` dice la verdad con menos. */}
            <div className="rf-row rf-gap-xs sede-waiting__tabs">
              <BrowserTab id="chrome" chosen={browser} onChoose={setBrowser} />
              <BrowserTab id="firefox" chosen={browser} onChoose={setBrowser} />
            </div>

            {browser === "chrome" ? <ChromeRecipe /> : <FirefoxRecipe />}

            {browser === "firefox" && localCa}
          </>
        )}
      </div>
    </SedeBody>
  );
}

function BrowserTab({
  id,
  chosen,
  onChoose,
}: {
  id: Browser;
  chosen: Browser;
  onChoose: (browser: Browser) => void;
}) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      aria-pressed={id === chosen}
      className={`rf-btn sede-waiting__tab${id === chosen ? " sede-waiting__tab--chosen" : ""}`}
      onClick={() => onChoose(id)}
    >
      {id === "chrome" ? t("sede.repair.chrome") : t("sede.repair.firefox")}
    </button>
  );
}

function ChromeRecipe() {
  const { t } = useTranslation();
  return (
    <ol className="rf-stack sede-waiting__steps">
      <li className="rf-prose">{t("sede.repair.chromeAllow")}</li>
      <li className="rf-prose">
        {t("sede.repair.chromeGone")}
        <span className="rf-row rf-gap-xs sede-waiting__address">
          {/* Un `chrome://` no es navegable desde fuera: se copia, no se pulsa. */}
          <code className="rf-body">{CHROME_LOCAL_NETWORK_SETTINGS}</code>
          <button
            type="button"
            className="rf-btn rf-btn--ghost"
            onClick={() => void navigator.clipboard.writeText(CHROME_LOCAL_NETWORK_SETTINGS)}
          >
            <CopyIcon size={14} />
            {t("actions.copy")}
          </button>
        </span>
        <span className="rf-hint">{t("sede.repair.chromePadlock")}</span>
      </li>
    </ol>
  );
}

function FirefoxRecipe() {
  const { t } = useTranslation();
  return (
    <ol className="rf-stack sede-waiting__steps">
      <li className="rf-prose">{t("sede.repair.firefoxAllow")}</li>
      <li className="rf-prose">{t("sede.repair.firefoxGone")}</li>
    </ol>
  );
}
