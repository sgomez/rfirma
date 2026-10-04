//! El velo que confirma, ejecuta y cuenta la retirada del certificado de rFirma (docs/design/retirar-certificado.md).

import type { ReactNode } from "react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import { Dialog } from "../design-system/Dialog";
import { CheckCircleIcon, CheckingIcon, CrossCircleIcon } from "../design-system/icons";
import { Row } from "../design-system/Row";
import { Stack } from "../design-system/Stack";
import "./WithdrawCertificateDialog.css";
import {
  type StoreBrand,
  type StoreDetail,
  storeBrandLabel,
  type WithdrawalOutcome,
  type WithdrawalReport,
} from "./status";

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
  const [moment, setMoment] = useState<Moment>("question");
  const [report, setReport] = useState<WithdrawalReport | null>(null);

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

  const title =
    moment === "question"
      ? t("status.withdrawal.title.question")
      : moment === "working"
        ? t("status.withdrawal.title.working")
        : t(success ? "status.withdrawal.title.done" : "status.withdrawal.title.partial");

  return (
    <Dialog
      role="alertdialog"
      label={title}
      onClose={moment === "working" ? undefined : onClose}
      className="withdraw-certificate-dialog"
      scrimClassName="withdraw-certificate-dialog__scrim"
    >
      <p className="rf-title">{title}</p>

      {moment === "question" && (
        <Stack gap="xs">
          <p className="rf-prose">{t("status.withdrawal.body.certificate")}</p>
          <p className="rf-prose">{t("status.withdrawal.body.handler")}</p>
          <p className="rf-hint">{t("status.withdrawal.hint")}</p>
        </Stack>
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

      <Row className="withdraw-certificate-dialog__actions">
        {moment === "question" && (
          <>
            <Button variant="ghost" onClick={onClose}>
              {t("actions.cancel")}
            </Button>
            <Button variant="primary" onClick={withdraw}>
              {t("status.withdrawal.confirm")}
            </Button>
          </>
        )}
        {moment === "working" && (
          <Button variant="ghost" disabled>
            {t("actions.close")}
          </Button>
        )}
        {moment === "result" && (
          <>
            {!success && (
              <Button variant="ghost" onClick={onClose}>
                {t("actions.close")}
              </Button>
            )}
            <Button variant="primary" onClick={success ? onClose : withdraw}>
              {success ? t("actions.close") : t("actions.retry")}
            </Button>
          </>
        )}
      </Row>
    </Dialog>
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
