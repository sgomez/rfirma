//! La vista del cuerpo con el estado de rFirma: una fila por señal, su detalle, sus acciones y «Volver a comprobar», sin puertos.

import type { TFunction } from "i18next";
import type { ReactNode } from "react";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import "./StatusView.css";
import {
  AlertIcon,
  CheckCircleIcon,
  CheckingIcon,
  ChevronDownIcon,
  ChevronRightIcon,
  CrossCircleIcon,
  NotApplicableIcon,
} from "../design-system/icons";
import { Select } from "../design-system/Select";
import {
  type Signal,
  type SignalDetail,
  type SignalRow,
  storeBrandLabel,
  type Verdict,
} from "./status";

export interface StatusViewProps {
  rows: SignalRow[];
  isRechecking?: boolean;
  onClose: () => void;
  onRecheck: () => void;
  onAction: (row: SignalRow) => void;
  onChooseSiteSignatureHandler: (handlerId: string) => void;
  onWithdraw: () => void;
  /** Las señales cuyo detalle empieza desplegado. */
  initiallyExpanded?: Signal[];
}

export function StatusView({
  rows,
  isRechecking = false,
  onClose,
  onRecheck,
  onAction,
  onChooseSiteSignatureHandler,
  onWithdraw,
  initiallyExpanded = [],
}: StatusViewProps) {
  const { t } = useTranslation();
  const [expandedDetail, setExpandedDetail] = useState<Set<Signal>>(
    () => new Set(initiallyExpanded),
  );

  const toggleDetail = useCallback((signal: Signal) => {
    setExpandedDetail((current) => {
      const next = new Set(current);
      if (next.has(signal)) {
        next.delete(signal);
      } else {
        next.add(signal);
      }
      return next;
    });
  }, []);

  return (
    <section className="status-view" aria-label={t("status.title")}>
      <div className="status-view__header">
        <h1 className="rf-title status-view__title">{t("status.title")}</h1>
        <Button
          variant="secondary"
          className="status-view__recheck"
          onClick={onRecheck}
          disabled={isRechecking}
        >
          {t("status.recheck")}
        </Button>
      </div>

      <div className="status-view__body">
        {rows.map((row) => (
          <div key={row.signal} className="status-view__row" role="status">
            <div className="status-view__row-main">
              <p className="rf-prose status-view__cell-signal">{signalLabel(t, row.signal)}</p>

              <div className="status-view__cell-value">
                {row.candidates && row.candidates.length >= 2 ? (
                  <Select
                    label={signalLabel(t, row.signal)}
                    hideLabel
                    value={row.candidates.find((candidate) => candidate.selected)?.id ?? ""}
                    options={row.candidates.map((candidate) => ({
                      value: candidate.id,
                      label: candidate.name,
                    }))}
                    onChange={onChooseSiteSignatureHandler}
                  />
                ) : (
                  <p className="rf-prose status-view__cell-value-text">{valueLabel(t, row)}</p>
                )}
              </div>

              <div
                className={`status-view__cell-verdict ${
                  demandsAttention(row.verdict)
                    ? "status-view__cell-verdict--strong"
                    : "status-view__cell-verdict--muted"
                }`}
              >
                {row.verdict !== "notApplicable" && (
                  <>
                    <span className="status-view__verdict-icon">
                      {renderVerdictIcon(row.verdict)}
                    </span>
                    <span
                      className={`rf-body status-view__verdict-text ${
                        demandsAttention(row.verdict) ? "status-view__verdict-text--strong" : ""
                      }`}
                    >
                      {verdictLabel(t, row.verdict)}
                    </span>
                  </>
                )}
              </div>

              <div className="status-view__cell-action">
                {row.signal === "localCaCertificate" && row.verdict === "correct" ? (
                  <Button
                    variant="secondary"
                    className="status-view__action-btn"
                    onClick={onWithdraw}
                  >
                    {t("status.actions.withdraw")}
                  </Button>
                ) : (
                  row.action && (
                    <Button
                      variant="secondary"
                      className="status-view__action-btn"
                      onClick={() => onAction(row)}
                    >
                      {actionLabel(t, row)}
                    </Button>
                  )
                )}
              </div>
            </div>

            {row.signal === "siteSignature" && row.verdict === "notApplicable" && (
              <SiteSignatureDiagnosis
                expanded={expandedDetail.has(row.signal)}
                onToggle={() => toggleDetail(row.signal)}
              />
            )}

            {row.detail && (
              <div className="status-view__detail">
                <Button
                  variant="ghost"
                  className="status-view__detail-toggle"
                  aria-expanded={expandedDetail.has(row.signal)}
                  aria-controls={`status-view__detail-${row.signal}`}
                  onClick={() => toggleDetail(row.signal)}
                >
                  {expandedDetail.has(row.signal) ? (
                    <ChevronDownIcon size={14} />
                  ) : (
                    <ChevronRightIcon size={14} />
                  )}
                  {detailToggleLabel(t, row.detail)}
                </Button>

                {expandedDetail.has(row.signal) && (
                  <ul id={`status-view__detail-${row.signal}`} className="status-view__detail-list">
                    {row.detail.kind === "trust"
                      ? row.detail.stores.map((store, index) => (
                          // biome-ignore lint/suspicious/noArrayIndexKey: la vista no trae la ruta del almacén, solo su marca, y el orden no cambia entre pintadas.
                          <li key={`${store.brand}-${index}`} className="status-view__detail-item">
                            {store.trusted ? (
                              <CheckCircleIcon size={14} />
                            ) : (
                              <CrossCircleIcon size={14} />
                            )}
                            <span className="rf-prose">{storeBrandLabel(t, store.brand)}</span>
                            <span className="rf-body status-view__detail-note">
                              {store.trusted
                                ? t("status.detail.installed")
                                : t("status.detail.notInstalled")}
                            </span>
                          </li>
                        ))
                      : row.detail.stores.map((store) => (
                          <li key={store.brand} className="status-view__detail-item">
                            <span className="rf-prose status-view__detail-place">
                              {storeBrandLabel(t, store.brand)}
                            </span>
                            <span className="rf-body status-view__detail-note">
                              {store.certificates}
                            </span>
                          </li>
                        ))}
                  </ul>
                )}
              </div>
            )}

            {row.restartFirefoxNotice && (
              <p className="rf-body status-view__restart-notice">
                {t("status.notices.restartFirefox")}
              </p>
            )}
          </div>
        ))}
      </div>

      <div className="status-view__footer">
        <Button variant="secondary" className="status-view__close" onClick={onClose}>
          {t("actions.close")}
        </Button>
      </div>
    </section>
  );
}

const RFIRMA_DESKTOP_FILE = "me.sgomez.rfirma.desktop";

function SiteSignatureDiagnosis({
  expanded,
  onToggle,
}: {
  expanded: boolean;
  onToggle: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="status-view__detail">
      <Button
        variant="ghost"
        className="status-view__detail-toggle"
        aria-expanded={expanded}
        aria-controls="status-view__detail-siteSignature"
        onClick={onToggle}
      >
        {expanded ? <ChevronDownIcon size={14} /> : <ChevronRightIcon size={14} />}
        {t("status.detail.diagnose")}
      </Button>

      {expanded && (
        <ul id="status-view__detail-siteSignature" className="status-view__detail-list">
          <li className="status-view__detail-item">
            <span className="rf-prose">{t("status.detail.diagnoseIntro")}</span>
          </li>
          <li className="status-view__detail-item">
            <code className="status-view__detail-command">
              xdg-mime query default x-scheme-handler/afirma
            </code>
          </li>
          <li className="status-view__detail-item">
            <span className="rf-prose">
              {t("status.detail.diagnoseFix", { file: RFIRMA_DESKTOP_FILE })}
            </span>
          </li>
          <li className="status-view__detail-item">
            <code className="status-view__detail-command">
              xdg-mime default {RFIRMA_DESKTOP_FILE} x-scheme-handler/afirma
            </code>
          </li>
        </ul>
      )}
    </div>
  );
}

function actionLabel(t: TFunction, row: SignalRow): string {
  if (!row.action) return "";
  switch (row.signal) {
    case "version":
      return t("status.actions.update");
    case "userCertificates":
      return t("status.actions.howToInstall");
    case "localCaCertificate":
      return t("status.actions.install");
    case "siteSignature":
      return t("status.actions.useRfirma");
  }
}

function signalLabel(t: TFunction, signal: Signal): string {
  switch (signal) {
    case "version":
      return t("status.signals.version");
    case "userCertificates":
      return t("status.signals.userCertificates");
    case "localCaCertificate":
      return t("status.signals.localCaCertificate");
    case "siteSignature":
      return t("status.signals.siteSignature");
  }
}

function valueLabel(t: TFunction, row: SignalRow): string {
  switch (row.signal) {
    case "version":
      return row.value;
    case "siteSignature":
      if (row.verdict === "notApplicable") return t("status.values.siteSignature.unavailable");
      if (row.value === "") return t("status.values.siteSignature.notConfigured");
      return row.value;
    case "localCaCertificate": {
      if (row.value === "") return "";
      const [trusted, total] = row.value.split("/").map(Number);
      return t("status.values.localCaCertificate", { count: total, trusted });
    }
    case "userCertificates": {
      const count = Number(row.value);
      return count === 0
        ? t("status.values.userCertificates.none")
        : t("status.values.userCertificates.count", { count });
    }
  }
}

function detailToggleLabel(t: TFunction, detail: SignalDetail): string {
  return detail.kind === "trust" ? t("status.detail.toggle") : t("status.detail.where");
}

function demandsAttention(verdict: Verdict): boolean {
  return verdict === "attention" || verdict === "incorrect";
}

function verdictLabel(t: TFunction, verdict: Verdict): string {
  switch (verdict) {
    case "correct":
      return t("status.verdicts.correct");
    case "attention":
      return t("status.verdicts.attention");
    case "incorrect":
      return t("status.verdicts.incorrect");
    case "notApplicable":
      return t("status.verdicts.notApplicable");
    case "checking":
      return t("status.verdicts.checking");
  }
}

function renderVerdictIcon(verdict: Verdict): ReactNode {
  switch (verdict) {
    case "correct":
      return <CheckCircleIcon size={16} />;
    case "attention":
      return <AlertIcon size={16} />;
    case "incorrect":
      return <CrossCircleIcon size={16} />;
    case "notApplicable":
      return <NotApplicableIcon size={16} />;
    case "checking":
      return <CheckingIcon size={16} />;
  }
}
