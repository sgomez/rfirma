//! El aviso de error: la situación traducida y, aparte, el texto original crudo en un detalle plegado.

import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useActionKeys } from "../design-system/actionKeys";
import { Button } from "../design-system/Button";
import { AlertIcon, ExternalLinkIcon } from "../design-system/icons";
import { Row } from "../design-system/Row";
import { useDefaultButton } from "../design-system/useDefaultButton";
import type { ExternalDestinationOpener } from "../desktop/externalDestination";
import { type ErrorSituation, errorText, MESSAGE_OF } from "./errorMessage";
import "./ErrorNotice.css";

/** Las situaciones sin detalle técnico ni acciones: el crudo sería jerga que no ayuda a nadie. */
const WITHOUT_DETAIL: readonly ErrorSituation[] = ["keyKindUnsupported", "pkcs12NoPrivateKey"];

/** Los fallos propios de rFirma, o los que no sabe explicar, llevan enlace a «Comentarios y ayuda». */
function hasHelpLink(situation: ErrorSituation): boolean {
  return MESSAGE_OF[situation] === "retry" || situation === "renderFailed";
}

interface ErrorNoticeProps {
  /** La situación que mandó el backend; el mensaje sale de `errorText`. */
  situation: ErrorSituation;
  /** El texto original tal cual llegó, sin traducir ni recortar, para pegarlo en un informe de fallo. */
  technicalDetail?: string;
  onOpenHelp?: () => void;
  externalDestinations?: ExternalDestinationOpener;
  /** Con este botón, el aviso ya no es solo informativo: además recarga la ventana. */
  onReload?: () => void;
  /** Vacía el Almacén de rFirma, ofrecido solo con `keyringPinMissing` y con confirmación (ADR-0034). */
  onEmptyStore?: () => void;
  /** El error boundary de cada ventana quiere el foco encima al aparecer; nadie más lo pide. */
  focusOnMount?: boolean;
  /** La tarjeta del error de firma: título fijo, la situación como causa y la tranquilidad de que nada se ha guardado. */
  documentUnchanged?: boolean;
  /** Un título propio en lugar del de la situación, sin su cuerpo, con el detalle a la vista y «Copiar detalle». */
  title?: string;
}

/** Un fallo clasificado (ADR-0009): su mensaje traducido y, plegado, el detalle técnico crudo. */
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
  const confirmEmptyStoreButton = useDefaultButton(confirmingEmptyStore);
  useActionKeys(
    confirmingEmptyStore
      ? { primary: confirmEmptyStoreButton, onSecondary: () => setConfirmingEmptyStore(false) }
      : {},
  );

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
      <Button variant="ghost" onClick={() => setConfirmingEmptyStore(false)}>
        {t("actions.cancel")}
      </Button>
      <Button
        ref={confirmEmptyStoreButton}
        variant="primary"
        onClick={() => {
          setConfirmingEmptyStore(false);
          onEmptyStore?.();
        }}
      >
        {t("errors.emptyStore.button")}
      </Button>
    </>
  ) : (
    <Button
      variant="ghost"
      className="error-notice__empty-store"
      onClick={() => setConfirmingEmptyStore(true)}
    >
      {t("errors.emptyStore.button")}
    </Button>
  );

  const text = errorText(situation, t);
  const showsDetail = !WITHOUT_DETAIL.includes(situation);

  return (
    <div className="error-notice" role="alert" ref={notice} tabIndex={-1}>
      <p className="error-notice__title">
        <AlertIcon />
        <span className="rf-title">
          {title ?? (documentUnchanged ? t("errors.signingFailedTitle") : text.title)}
        </span>
      </p>
      {documentUnchanged && MESSAGE_OF[situation] !== "retry" && (
        <p className="rf-prose">{text.title}</p>
      )}
      {text.body !== undefined && !documentUnchanged && title === undefined && (
        <p className="rf-prose">{text.body}</p>
      )}
      {documentUnchanged && <p className="rf-prose">{t("errors.documentUnchanged")}</p>}
      {showsDetail && (
        <>
          <details className="error-notice__detail" open={title !== undefined || undefined}>
            <summary className="rf-body rf-text-muted">{t("errors.technicalDetail")}</summary>
            <pre className="error-notice__raw">{technicalDetail}</pre>
          </details>
          {(documentUnchanged || title !== undefined) && (
            <Row gap="xs" className="error-notice__actions">
              <Button variant="ghost" className="error-notice__copy" onClick={copyDetail}>
                {t("errors.copyDetail")}
              </Button>
              {offersToEmptyStore && emptyStoreAction}
            </Row>
          )}
          {!documentUnchanged &&
            title === undefined &&
            (hasHelpLink(situation) || onReload || offersToEmptyStore) && (
              <Row gap="xs" className="error-notice__actions">
                {hasHelpLink(situation) && (
                  <Button variant="ghost" className="error-notice__help" onClick={openHelp}>
                    <ExternalLinkIcon size={14} />
                    {t("header.help")}
                  </Button>
                )}
                {onReload && (
                  <Button variant="primary" onClick={onReload}>
                    {t("errors.reload")}
                  </Button>
                )}
                {offersToEmptyStore && emptyStoreAction}
              </Row>
            )}
        </>
      )}
    </div>
  );
}
