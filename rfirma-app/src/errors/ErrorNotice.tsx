//! El aviso de error: la situación traducida y, aparte, el texto original crudo en un detalle plegado.

import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertIcon, ExternalLinkIcon } from "../design-system/icons";
import type { ExternalDestinationOpener } from "../desktop/externalDestination";
import type { Catalog } from "../i18n/catalog";
import "./ErrorNotice.css";

/**
 * Las situaciones que sabemos nombrar. Hoy solo la genérica: el mapeo de los
 * `CKR_*` de `cryptoki` y de las excepciones del puente es de otro sub-issue,
 * y cada situación que añada entra aquí y en `po/messages.pot`.
 */
export type ErrorSituation = keyof Catalog["errors"]["situations"];

/**
 * Las situaciones que se cuentan en **un solo renglón**: el título lo dice
 * todo, y lo que iría debajo sería jerga o el remedio obvio (ID-211).
 *
 * Son las que no tienen `body` en el catálogo, así que la lista no es un gusto:
 * `tsc` la obliga a cuadrar con las claves que existen.
 */
const ONE_LINE = ["keyKindUnsupported", "pkcs12NoPrivateKey"] as const;

type OneLineSituation = (typeof ONE_LINE)[number];

function isOneLine(situation: ErrorSituation): situation is OneLineSituation {
  return (ONE_LINE as readonly string[]).includes(situation);
}

/**
 * Las situaciones de error de rFirma que llevan enlace a «Comentarios y ayuda».
 *
 * Es una lista cerrada (ID-371): los fallos propios de rFirma o donde no sabe
 * qué ha pasado. Los fallos del entorno (PIN incorrecto, tarjeta ausente,
 * certificado caducado, etc.) no llevan enlace para no mandar a la persona al
 * sitio equivocado.
 */
const ERROR_SITUATIONS_WITH_HELP = [
  "bridgeFailed",
  "sealMismatch",
  "unknown",
  "renderFailed",
] as const;

function hasHelpLink(situation: ErrorSituation): boolean {
  return (ERROR_SITUATIONS_WITH_HELP as readonly string[]).includes(situation);
}

interface ErrorNoticeProps {
  /** Nuestra situación, que sí está traducida. */
  situation: ErrorSituation;
  /**
   * El texto original tal cual llegó: el `CKR_*` de `cryptoki` o el mensaje
   * incrustado de la excepción del puente. **No se traduce ni se recorta**:
   * está para pegarlo en un informe de fallo.
   *
   * Sobra —y no se pone— en una situación de un solo renglón: el detalle crudo
   * de una clave elíptica es la curva, que no le sirve a nadie que esté delante
   * de esta pantalla.
   */
  technicalDetail?: string;
  onOpenHelp?: () => void;
  externalDestinations?: ExternalDestinationOpener;
  /** Con este botón, el aviso ya no es solo informativo: además recarga la ventana. */
  onReload?: () => void;
  /** Vacía el Almacén de rFirma, ofrecido solo con `keyringPinMissing` y con confirmación (ADR-0034). */
  onEmptyStore?: () => void;
  /** El error boundary de cada ventana quiere el foco encima al aparecer; nadie más lo pide. */
  focusOnMount?: boolean;
  /**
   * Añade la tranquilidad de que nada se ha guardado (docs/design/panel-de-firma.md
   * § Error al firmar): un fallo a mitad de una operación que escribe disco dice
   * además que el documento sigue como estaba.
   */
  documentUnchanged?: boolean;
  /** Un título propio en lugar del de la situación, sin su cuerpo, con el detalle a la vista y «Copiar detalle». */
  title?: string;
}

/**
 * Un error, como manda el ID-29: una **situación** nuestra traducida y, aparte,
 * el texto original crudo en un detalle plegado.
 *
 * Los errores no se traducen, se clasifican. `cryptoki` devuelve códigos y el
 * puente Java devuelve excepciones cuyo texto está incrustado en el código
 * —`afirma-crypto-pdf` no tiene ni un `.properties` localizado—, así que
 * ninguno de los dos se enseña como mensaje. Lo que no sepamos clasificar cae
 * en `unknown` más su detalle técnico crudo (ADR-0009).
 *
 * El artboard del error de firma dibuja el detalle **desplegado**. Eso es un
 * estado congelado, no el inicial (ID-43): aquí sigue plegado, porque el
 * `CKR_*` crudo debajo del mensaje ocupa el pie entero y solo lo necesita quien
 * va a escribir un informe de fallo.
 *
 * Con `documentUnchanged` la tarjeta es la del error de firma
 * (docs/design/panel-de-firma.md § Estados → Error al firmar) y nada más: título
 * fijo, la situación como causa, la tranquilidad, el detalle y «Copiar detalle».
 */
export function ErrorNotice({
  situation,
  technicalDetail,
  onOpenHelp,
  externalDestinations,
  onReload,
  onEmptyStore,
  focusOnMount,
  documentUnchanged,
  title,
}: ErrorNoticeProps) {
  const { t } = useTranslation();
  const notice = useRef<HTMLDivElement>(null);
  const [confirmingEmptyStore, setConfirmingEmptyStore] = useState(false);

  useEffect(() => {
    if (focusOnMount) notice.current?.focus();
  }, [focusOnMount]);

  const openHelp = () => {
    onOpenHelp?.();
    void externalDestinations?.open("discussions");
  };

  const copyDetail = () => {
    if (technicalDetail !== undefined) void navigator.clipboard.writeText(technicalDetail);
  };

  const offersToEmptyStore = situation === "keyringPinMissing" && onEmptyStore !== undefined;

  const emptyStoreAction = confirmingEmptyStore ? (
    <>
      <span className="rf-body error-notice__empty-store-question">
        {t("errors.emptyStore.confirmQuestion")}
      </span>
      <button
        type="button"
        className="rf-btn rf-btn--ghost"
        onClick={() => setConfirmingEmptyStore(false)}
      >
        {t("actions.cancel")}
      </button>
      <button
        type="button"
        className="rf-btn rf-btn--primary"
        onClick={() => {
          setConfirmingEmptyStore(false);
          onEmptyStore?.();
        }}
      >
        {t("errors.emptyStore.confirmButton")}
      </button>
    </>
  ) : (
    <button
      type="button"
      className="rf-btn rf-btn--ghost error-notice__empty-store"
      onClick={() => setConfirmingEmptyStore(true)}
    >
      {t("errors.emptyStore.button")}
    </button>
  );

  return (
    <div className="error-notice" role="alert" ref={notice} tabIndex={-1}>
      <p className="error-notice__title">
        <AlertIcon />
        <span className="rf-title">
          {title ??
            (documentUnchanged
              ? t("errors.signingFailedTitle")
              : t(`errors.situations.${situation}.title`))}
        </span>
      </p>
      {documentUnchanged && <p className="rf-prose">{t(`errors.situations.${situation}.title`)}</p>}
      {!isOneLine(situation) && !documentUnchanged && title === undefined && (
        <p className="rf-prose">
          {t(`errors.situations.${situation as Exclude<ErrorSituation, OneLineSituation>}.body`)}
        </p>
      )}
      {documentUnchanged && <p className="rf-prose">{t("errors.documentUnchanged")}</p>}
      {!isOneLine(situation) && (
        <>
          <details className="error-notice__detail" open={title !== undefined || undefined}>
            <summary className="rf-body rf-text-muted">{t("errors.technicalDetail")}</summary>
            <pre className="error-notice__raw">{technicalDetail}</pre>
          </details>
          {(documentUnchanged || title !== undefined) && (
            <div className="rf-row rf-gap-xs error-notice__actions">
              <button
                type="button"
                className="rf-btn rf-btn--ghost error-notice__copy"
                onClick={copyDetail}
              >
                {t("errors.copyDetail")}
              </button>
              {offersToEmptyStore && emptyStoreAction}
            </div>
          )}
          {!documentUnchanged &&
            title === undefined &&
            (hasHelpLink(situation) || onReload || offersToEmptyStore) && (
              <div className="rf-row rf-gap-xs error-notice__actions">
                {hasHelpLink(situation) && (
                  <button
                    type="button"
                    className="rf-btn rf-btn--ghost error-notice__help"
                    onClick={openHelp}
                  >
                    <ExternalLinkIcon size={14} />
                    {t("errors.help")}
                  </button>
                )}
                {onReload && (
                  <button type="button" className="rf-btn rf-btn--primary" onClick={onReload}>
                    {t("errors.reload")}
                  </button>
                )}
                {offersToEmptyStore && emptyStoreAction}
              </div>
            )}
        </>
      )}
    </div>
  );
}
