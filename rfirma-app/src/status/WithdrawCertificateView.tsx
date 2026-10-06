//! La vista del velo de la retirada del certificado de rFirma en cada momento, sin estado ni puertos (docs/design/retirar-certificado.md).

import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import { Dialog } from "../design-system/Dialog";
import { CheckCircleIcon, CheckingIcon, CrossCircleIcon } from "../design-system/icons";
import { Row } from "../design-system/Row";
import { Stack } from "../design-system/Stack";
import "./WithdrawCertificateView.css";
import {
  type StoreBrand,
  type StoreDetail,
  storeBrandLabel,
  type WithdrawalOutcome,
  type WithdrawalReport,
} from "./status";

export type WithdrawalMoment = "question" | "working" | "result";

interface WithdrawCertificateViewProps {
  /** Los almacenes donde está hoy el certificado, tal como los cuenta la fila. */
  stores: StoreDetail[];
  moment: WithdrawalMoment;
  /** El resultado del último intento; solo se pinta en el momento `result`. */
  report: WithdrawalReport | null;
  /** Lanza la retirada, o la repite sobre lo que falló. */
  onWithdraw: () => void;
  /** Cierra el velo, con o sin retirada hecha. */
  onClose: () => void;
}

/** El velo de la retirada en el momento `moment`, empezando bajo la cabecera para dejarla alcanzable (ADR-0005). */
export function WithdrawCertificateView({
  stores,
  moment,
  report,
  onWithdraw,
  onClose,
}: WithdrawCertificateViewProps) {
  const { t } = useTranslation();

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
        <Stack as="ul" gap="xs" className="withdraw-certificate-dialog__list">
          {stores.map((store) => (
            <StoreLine key={store.brand} brand={store.brand} icon={<CheckingIcon size={14} />}>
              {t("status.withdrawal.waiting")}
            </StoreLine>
          ))}
        </Stack>
      )}

      {moment === "result" && report && (
        <>
          <Stack as="ul" gap="xs" className="withdraw-certificate-dialog__list">
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
            <Row as="li" gap="xs">
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
            </Row>
          </Stack>
          <p className="rf-body">{t("status.withdrawal.restartBrowserNotice")}</p>
        </>
      )}

      <Row className="withdraw-certificate-dialog__actions">
        {moment === "question" && (
          <>
            <Button variant="ghost" onClick={onClose}>
              {t("actions.cancel")}
            </Button>
            <Button variant="primary" onClick={onWithdraw}>
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
            <Button variant="primary" onClick={success ? onClose : onWithdraw}>
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
    <Row as="li" gap="xs">
      <span aria-hidden="true">{icon}</span>
      <span className="rf-prose">{storeBrandLabel(t, brand)}</span>
      <span className="rf-body rf-text-muted">{children}</span>
    </Row>
  );
}
