import { useTranslation } from "react-i18next";
import { InfoIcon } from "../design-system/icons";
import "./UrlHandlerBanner.css";

interface UrlHandlerBannerProps {
  /** Deja a rFirma atendiendo los enlaces. Es el «Sí». */
  onAccept: () => void;
  /** Cierra el banner **para esta sesión**, sin guardar nada. Es «Ahora no». */
  onLater: () => void;
  /**
   * Apaga la pregunta para siempre. Es «No volver a preguntar», y quien nos
   * monta lo escribe en `Preferences.askAboutUrlHandler`, donde se puede
   * volver a encender.
   */
  onNever: () => void;
}

/**
 * El banner del arranque: **quién atiende los enlaces `afirma://`** (ID-239).
 *
 * Es **preventivo por narices**: cuando el trámite lo atiende la otra
 * aplicación, rFirma ni se ejecuta y no puede enseñar nada. Por eso pregunta al
 * arrancar y no cuando falla algo.
 *
 * Es un atajo del control de Preferencias, no un ajuste aparte: «Sí» escribe lo
 * mismo que el desplegable, y «No volver a preguntar» apaga el único ajuste que
 * este banner tiene, que se deshace en esa misma pantalla.
 *
 * No es modal, por lo mismo que la franja de notificación (ID-181, ID-207): no
 * bloquea nada, así que no interrumpe el recorrido. Ocupa el mismo hueco entre
 * la cabecera y las regiones, y cuando no hay nada que preguntar **no se
 * monta**.
 *
 * Quién decide si hay algo que preguntar es la composición, con
 * [`theBannerHasSomethingToAsk`](./urlHandlers): dentro del flatpak no hay
 * banner que valga (ID-240).
 */
export function UrlHandlerBanner({ onAccept, onLater, onNever }: UrlHandlerBannerProps) {
  const { t } = useTranslation();

  return (
    <div className="url-handler-banner" role="status">
      <span className="url-handler-banner__icon" aria-hidden="true">
        <InfoIcon size={18} />
      </span>
      <p className="rf-body url-handler-banner__message">{t("urlHandler.banner.message")}</p>
      <button
        type="button"
        className="rf-btn rf-btn--secondary url-handler-banner__answer"
        onClick={onAccept}
      >
        {t("urlHandler.banner.accept")}
      </button>
      <button
        type="button"
        className="rf-btn rf-btn--ghost url-handler-banner__answer"
        onClick={onLater}
      >
        {t("urlHandler.banner.later")}
      </button>
      <button
        type="button"
        className="rf-btn rf-btn--ghost url-handler-banner__answer"
        onClick={onNever}
      >
        {t("urlHandler.banner.never")}
      </button>
    </div>
  );
}
