import type { TFunction } from "i18next";
import type { ReactNode } from "react";
import { type KeyboardEvent, useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { CheckCircleIcon, CheckingIcon, CrossCircleIcon } from "../design-system/icons";
import "./WithdrawCertificateDialog.css";
import type { StoreBrand, StoreDetail, WithdrawalOutcome, WithdrawalReport } from "./status";

interface WithdrawCertificateDialogProps {
  /** Los almacenes donde está hoy el certificado, tal como los cuenta la fila. */
  stores: StoreDetail[];
  /**
   * Ejecuta la retirada de verdad: el manejador de sedes y la CA de cada
   * almacén. Con el resultado del intento anterior, solo repite lo que
   * falló.
   */
  onWithdraw: (previous: WithdrawalReport | null) => Promise<WithdrawalReport>;
  /** Cierra el velo, con o sin retirada hecha; quien nos monta vuelve a medir. */
  onClose: () => void;
}

type Moment = "question" | "working" | "result";

/** Lo que puede recibir el foco dentro del velo. */
const FOCUSABLE = 'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';

/** El tabulador da la vuelta dentro del velo en vez de salirse a la ventana de detrás. */
function trapTabWithinCurrentTarget(event: KeyboardEvent<HTMLDivElement>) {
  if (event.key !== "Tab") return;
  const modal = event.currentTarget;
  const focusable = [...modal.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
    (element) => !element.hasAttribute("disabled") && element.tabIndex !== -1,
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

/**
 * El velo que confirma, ejecuta y cuenta la retirada del certificado de
 * rFirma (docs/design/retirar-certificado.md, ADR-0005).
 *
 * **No tapa la cabecera**: su velo empieza a los 56 px del marco
 * (`WithdrawCertificateDialog.css`), no en el borde de la ventana como
 * `.rf-scrim`, así que la cabecera sigue alcanzable mientras trabaja.
 *
 * Cubre el camino en el que todo se retira; «Retirado a medias» solo aparece
 * si `onWithdraw` devuelve algún fallo, y `Reintentar` solo vuelve a tocar lo
 * que falló, con el informe anterior como referencia.
 */
export function WithdrawCertificateDialog({
  stores,
  onWithdraw,
  onClose,
}: WithdrawCertificateDialogProps) {
  const { t } = useTranslation();
  const titleId = useId();
  const [moment, setMoment] = useState<Moment>("question");
  const [report, setReport] = useState<WithdrawalReport | null>(null);
  const dialog = useRef<HTMLDivElement>(null);

  useEffect(() => {
    dialog.current?.focus();
  }, []);

  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || moment === "working") return;
      event.preventDefault();
      onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose, moment]);

  const withdraw = () => {
    setMoment("working");
    void onWithdraw(report).then((result) => {
      setReport(result);
      setMoment("result");
    });
  };

  const failed = (outcome: WithdrawalOutcome) => outcome.kind === "failed";
  const success =
    report !== null &&
    !failed(report.handler) &&
    !report.stores.some((store) => failed(store.outcome));

  return (
    <div className="rf-scrim withdraw-certificate-dialog__scrim">
      <div
        className="rf-dialog withdraw-certificate-dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        ref={dialog}
        onKeyDown={trapTabWithinCurrentTarget}
      >
        <p className="rf-title" id={titleId}>
          {moment === "question" && t("status.withdrawal.title.question")}
          {moment === "working" && t("status.withdrawal.title.working")}
          {moment === "result" &&
            t(success ? "status.withdrawal.title.done" : "status.withdrawal.title.partial")}
        </p>

        {moment === "question" && (
          <div className="rf-stack rf-gap-xs">
            <p className="rf-prose">{t("status.withdrawal.body.certificate")}</p>
            <p className="rf-prose">{t("status.withdrawal.body.handler")}</p>
            <p className="rf-hint">{t("status.withdrawal.hint")}</p>
          </div>
        )}

        {moment === "working" && (
          <ul className="rf-stack rf-gap-xs withdraw-certificate-dialog__list">
            {stores.map((store) => (
              <StoreLine key={store.brand} brand={store.brand} icon={<CheckingIcon size={14} />}>
                {t("status.withdrawal.waiting")}
              </StoreLine>
            ))}
          </ul>
        )}

        {moment === "result" && report && (
          <>
            <ul className="rf-stack rf-gap-xs withdraw-certificate-dialog__list">
              {report.stores.map((store) => (
                <StoreLine
                  key={store.brand}
                  brand={store.brand}
                  icon={
                    failed(store.outcome) ? (
                      <CrossCircleIcon size={14} />
                    ) : (
                      <CheckCircleIcon size={14} />
                    )
                  }
                >
                  {store.outcome.kind === "failed"
                    ? store.outcome.reason
                    : t("status.withdrawal.outcome.withdrawn")}
                </StoreLine>
              ))}
              <li className="rf-row rf-gap-xs">
                {failed(report.handler) ? (
                  <CrossCircleIcon size={14} />
                ) : (
                  <CheckCircleIcon size={14} />
                )}
                <span className="rf-prose">{t("status.signals.siteSignature")}</span>
                <span className="rf-body rf-text-muted">
                  {report.handler.kind === "failed"
                    ? report.handler.reason
                    : t("status.withdrawal.outcome.withdrawn")}
                </span>
              </li>
            </ul>
            <p className="rf-body">{t("status.withdrawal.restartBrowserNotice")}</p>
          </>
        )}

        <div className="rf-row withdraw-certificate-dialog__actions">
          {moment === "question" && (
            <>
              <button type="button" className="rf-btn rf-btn--ghost" onClick={onClose}>
                {t("actions.cancel")}
              </button>
              <button type="button" className="rf-btn rf-btn--primary" onClick={withdraw}>
                {t("status.withdrawal.confirm")}
              </button>
            </>
          )}
          {moment === "working" && (
            <button type="button" className="rf-btn rf-btn--ghost" disabled>
              {t("actions.close")}
            </button>
          )}
          {moment === "result" && (
            <>
              {!success && (
                <button type="button" className="rf-btn rf-btn--ghost" onClick={onClose}>
                  {t("actions.close")}
                </button>
              )}
              <button
                type="button"
                className="rf-btn rf-btn--primary"
                onClick={success ? onClose : withdraw}
              >
                {success ? t("actions.close") : t("status.withdrawal.retry")}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

interface StoreLineProps {
  brand: StoreBrand;
  icon: ReactNode;
  children: ReactNode;
}

function StoreLine({ brand, icon, children }: StoreLineProps) {
  const { t } = useTranslation();
  return (
    <li className="rf-row rf-gap-xs">
      <span aria-hidden="true">{icon}</span>
      <span className="rf-prose">{storeBrandLabel(t, brand)}</span>
      <span className="rf-body rf-text-muted">{children}</span>
    </li>
  );
}

function storeBrandLabel(t: TFunction, brand: StoreBrand): string {
  switch (brand) {
    case "firefox":
      return t("status.storeBrands.firefox");
    case "chrome":
      return t("status.storeBrands.chrome");
    case "nssdb":
      return t("status.storeBrands.nssdb");
  }
}
