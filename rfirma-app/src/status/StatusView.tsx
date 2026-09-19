import type { TFunction } from "i18next";
import type { ReactNode } from "react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
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
import type {
  ExternalDestination,
  ExternalDestinationOpener,
} from "../desktop/externalDestination";
import { unavailableExternalDestinationOpener } from "../desktop/externalDestination";
import {
  memoryStatus,
  type Signal,
  type SignalRow,
  type StatusPort,
  type StoreBrand,
  type Verdict,
} from "./status";

interface StatusViewProps {
  onClose: () => void;
  statusPort?: StatusPort;
  externalDestinations?: ExternalDestinationOpener;
}

export function StatusView({
  onClose,
  statusPort = memoryStatus(),
  externalDestinations = unavailableExternalDestinationOpener(),
}: StatusViewProps) {
  const { t } = useTranslation();
  const [rows, setRows] = useState<SignalRow[]>([]);
  const [isRechecking, setIsRechecking] = useState(false);
  const [expandedDetail, setExpandedDetail] = useState<Set<Signal>>(new Set());

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

  useEffect(() => {
    let cancelled = false;
    statusPort.readStatus().then((initialRows) => {
      if (!cancelled) {
        setRows(initialRows);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [statusPort]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) {
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [onClose]);

  const handleRecheck = useCallback(() => {
    setIsRechecking(true);
    setRows((current) =>
      current.map((row) => ({
        ...row,
        verdict: "checking",
        action: null,
        detail: null,
      })),
    );
    statusPort.recheck().then((updatedRows) => {
      setRows(updatedRows);
      setIsRechecking(false);
    });
  }, [statusPort]);

  const handleAction = useCallback(
    (row: SignalRow) => {
      if (!row.action) return;
      if (row.action.kind === "link") {
        void externalDestinations.open(row.action.target as ExternalDestination);
      }
      setRows((current) =>
        current.map((r) =>
          r.signal === row.signal
            ? { ...r, verdict: "checking", action: null, detail: null, restartFirefoxNotice: false }
            : r,
        ),
      );
      if (row.action.kind === "repair") {
        statusPort.installLocalCaCertificate().then((updatedRow) => {
          setRows((current) =>
            current.map((r) => (r.signal === updatedRow.signal ? updatedRow : r)),
          );
        });
        return;
      }
      statusPort.recheck().then((updatedRows) => {
        setRows(updatedRows);
      });
    },
    [externalDestinations, statusPort],
  );

  return (
    <section className="status-view" aria-label={t("status.title")}>
      <div className="status-view__header">
        <h1 className="rf-title status-view__title">{t("status.title")}</h1>
        <button
          type="button"
          className="rf-btn rf-btn--secondary status-view__recheck"
          onClick={handleRecheck}
          disabled={isRechecking}
        >
          {t("status.recheck")}
        </button>
      </div>

      <div className="status-view__body">
        <div className="status-view__table-header">
          <p className="rf-label status-view__col-signal">{t("status.columns.signal")}</p>
          <p className="rf-label status-view__col-value">{t("status.columns.value")}</p>
          <p className="rf-label status-view__col-verdict">{t("status.columns.verdict")}</p>
          <p className="rf-label status-view__col-action">{t("status.columns.action")}</p>
        </div>

        {rows.map((row) => (
          <div key={row.signal} className="status-view__row" role="status">
            <div className="status-view__row-main">
              <p className="rf-prose status-view__cell-signal">{signalLabel(t, row.signal)}</p>

              <div className="status-view__cell-value">
                <p className="rf-prose status-view__cell-value-text">{valueLabel(t, row)}</p>
              </div>

              <div className="status-view__cell-verdict">
                <span
                  className={`status-view__verdict-icon ${
                    row.verdict === "notApplicable" || row.verdict === "checking"
                      ? "status-view__verdict-icon--muted"
                      : "status-view__verdict-icon--default"
                  }`}
                >
                  {renderVerdictIcon(row.verdict)}
                </span>
                <span
                  className={`rf-body status-view__verdict-text ${
                    row.verdict === "attention" || row.verdict === "incorrect"
                      ? "status-view__verdict-text--strong"
                      : "status-view__verdict-text--muted"
                  }`}
                >
                  {verdictLabel(t, row.verdict)}
                </span>
              </div>

              <div className="status-view__cell-action">
                {row.action && (
                  <button
                    type="button"
                    className="rf-btn rf-btn--secondary status-view__action-btn"
                    onClick={() => handleAction(row)}
                  >
                    {actionLabel(t, row)}
                  </button>
                )}
              </div>
            </div>

            {row.detail && (
              <div className="status-view__detail">
                <button
                  type="button"
                  className="rf-btn rf-btn--ghost status-view__detail-toggle"
                  aria-expanded={expandedDetail.has(row.signal)}
                  aria-controls={`status-view__detail-${row.signal}`}
                  onClick={() => toggleDetail(row.signal)}
                >
                  {expandedDetail.has(row.signal) ? (
                    <ChevronDownIcon size={14} />
                  ) : (
                    <ChevronRightIcon size={14} />
                  )}
                  {t("status.detail.toggle")}
                </button>

                {expandedDetail.has(row.signal) && (
                  <ul id={`status-view__detail-${row.signal}`} className="status-view__detail-list">
                    {row.detail.map((store, index) => (
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
                            ? t("status.detail.trusted")
                            : t("status.detail.untrusted")}
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
        <button
          type="button"
          className="rf-btn rf-btn--secondary status-view__close"
          onClick={onClose}
        >
          {t("actions.close")}
        </button>
      </div>
    </section>
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
      return "";
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
      return "";
  }
}

function valueLabel(t: TFunction, row: SignalRow): string {
  switch (row.signal) {
    case "version":
    case "siteSignature":
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
        : t("status.values.userCertificates.stores", { count });
    }
  }
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
