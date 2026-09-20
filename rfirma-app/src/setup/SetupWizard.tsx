import type { TFunction } from "i18next";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertIcon, CheckCircleIcon, CheckingIcon } from "../design-system/icons";
import { Header } from "../shell/Header";
import { type MenuAnchor, menuAnchorFor } from "../shell/menuAnchor";
import "./SetupWizard.css";
import {
  memoryStatus,
  type StatusPort,
  type StoreBrand,
  type StoreDetail,
  withLocalCaCertificateMeasured,
} from "../status/status";

/** La versión de AutoFirma con la que rFirma se anuncia compatible (docs/afirma/1.9.2/). */
const AUTOFIRMA_VERSION = "1.9.2";

interface SetupWizardProps {
  /** Si el asistente ya se ha visto en un arranque anterior: entonces no se monta. */
  seen: boolean;
  statusPort?: StatusPort;
  /** Se llama una vez, al pulsar «Terminar», pase lo que pase con las dos acciones. */
  onFinish: () => void;
  /** Dónde va el menú de la cabecera. Ver [`MenuAnchor`]. */
  menuAnchor?: MenuAnchor;
  /** Las cuatro entradas del menú de la cabecera (ADR-0007), iguales a las de la ventana principal. */
  onOpenStatus?: () => void;
  onOpenPreferences?: () => void;
  onOpenHelp?: () => void;
  onOpenAbout?: () => void;
}

type CertificateStatus =
  | { kind: "idle" }
  | { kind: "declined" }
  | { kind: "working" }
  | { kind: "done"; restartNotice: boolean }
  | { kind: "failed"; detail: StoreDetail[] };

type HandlerStatus =
  | { kind: "idle"; target: string }
  | { kind: "unavailable" }
  | { kind: "declined" }
  | { kind: "working" }
  | { kind: "done" };

/**
 * El asistente del primer arranque: la ventana principal antes de que haya
 * documento, con la bienvenida y las dos acciones de configuración
 * (docs/design/primer-arranque.md). Sustituye entero al antiguo aviso de
 * confianza.
 *
 * Usa los mismos casos de uso que el panel de estado —`StatusPort`—: no hay
 * un camino de escritura propio. El estado de las dos tarjetas no se guarda
 * entre sesiones, se mide al montar.
 */
export function SetupWizard({
  seen,
  statusPort = memoryStatus(),
  onFinish,
  menuAnchor,
  onOpenStatus = () => {},
  onOpenPreferences = () => {},
  onOpenHelp = () => {},
  onOpenAbout = () => {},
}: SetupWizardProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState<1 | 2>(1);
  const [certificate, setCertificate] = useState<CertificateStatus>({ kind: "idle" });
  const [handler, setHandler] = useState<HandlerStatus>({ kind: "unavailable" });
  const [autoFirmaAppears, setAutoFirmaAppears] = useState(true);
  const continueButton = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (seen) return;
    continueButton.current?.focus();
  }, [seen]);

  useEffect(() => {
    if (seen) return;
    let cancelled = false;
    statusPort
      .readStatus()
      .then((rows) => withLocalCaCertificateMeasured(rows, statusPort))
      .then((rows) => {
        if (cancelled) return;
        const certificateRow = rows.find((row) => row.signal === "localCaCertificate");
        if (certificateRow) {
          setCertificate(
            certificateRow.verdict === "correct"
              ? { kind: "done", restartNotice: certificateRow.restartFirefoxNotice }
              : { kind: "idle" },
          );
        }
        const handlerRow = rows.find((row) => row.signal === "siteSignature");
        if (handlerRow) {
          setAutoFirmaAppears(
            handlerRow.candidates === null ||
              handlerRow.candidates.some((candidate) => candidate.name === "AutoFirma"),
          );
          setHandler(
            handlerRow.action?.kind === "choice"
              ? { kind: "idle", target: handlerRow.action.target }
              : { kind: "done" },
          );
        }
      });
    return () => {
      cancelled = true;
    };
  }, [seen, statusPort]);

  if (seen) return null;

  const installCertificate = () => {
    setCertificate({ kind: "working" });
    statusPort.installLocalCaCertificate().then((row) => {
      setCertificate(
        row.verdict === "correct"
          ? { kind: "done", restartNotice: row.restartFirefoxNotice }
          : { kind: "failed", detail: row.detail ?? [] },
      );
    });
  };

  const useRfirmaAsHandler = (target: string) => {
    setHandler({ kind: "working" });
    statusPort.chooseSiteSignatureHandler(target).then((rows) => {
      const handlerRow = rows.find((row) => row.signal === "siteSignature");
      setHandler(
        handlerRow && handlerRow.verdict === "correct" ? { kind: "done" } : { kind: "unavailable" },
      );
      const certificateRow = rows.find((row) => row.signal === "localCaCertificate");
      if (certificateRow) {
        setCertificate(
          certificateRow.verdict === "correct"
            ? { kind: "done", restartNotice: certificateRow.restartFirefoxNotice }
            : { kind: "failed", detail: certificateRow.detail ?? [] },
        );
      }
    });
  };

  return (
    <div className="setup-wizard">
      <Header
        status={null}
        menuAnchor={menuAnchor ?? menuAnchorFor(navigator.userAgent)}
        onOpenStatus={onOpenStatus}
        onOpenPreferences={onOpenPreferences}
        onOpenHelp={onOpenHelp}
        onOpenAbout={onOpenAbout}
      />

      <div className="setup-wizard__body">
        <div className="setup-wizard__column rf-stack rf-gap-md">
          <div className="rf-row rf-gap-xs">
            <span
              aria-hidden="true"
              className="setup-wizard__progress-bar setup-wizard__progress-bar--active"
            />
            <span
              aria-hidden="true"
              className={`setup-wizard__progress-bar ${step >= 2 ? "setup-wizard__progress-bar--active" : ""}`}
            />
            <span className="rf-body rf-text-muted setup-wizard__step">
              {t("setup.step", { current: step, total: 2 })}
            </span>
          </div>

          {step === 1 && <WelcomeScreen t={t} />}
          {step === 2 && (
            <div className="rf-stack rf-gap-md">
              <CertificateCard
                t={t}
                status={certificate}
                onInstall={installCertificate}
                onDecline={() => setCertificate({ kind: "declined" })}
              />
              <HandlerCard
                t={t}
                status={handler}
                autoFirmaAppears={autoFirmaAppears}
                onUse={useRfirmaAsHandler}
                onDecline={() => setHandler({ kind: "declined" })}
              />
            </div>
          )}
        </div>
      </div>

      <div className="setup-wizard__footer rf-row rf-gap-sm">
        {step === 2 && (
          <button type="button" className="rf-btn rf-btn--secondary" onClick={() => setStep(1)}>
            {t("setup.actions.back")}
          </button>
        )}
        {step === 1 ? (
          <button
            ref={continueButton}
            type="button"
            className="rf-btn rf-btn--primary"
            onClick={() => setStep(2)}
          >
            {t("setup.actions.continue")}
          </button>
        ) : (
          <button type="button" className="rf-btn rf-btn--primary" onClick={onFinish}>
            {t("setup.actions.finish")}
          </button>
        )}
      </div>
    </div>
  );
}

function WelcomeScreen({ t }: { t: TFunction }) {
  const independence = t("about.independence");
  const separator = independence.indexOf(". ");
  const independenceTitle = separator === -1 ? independence : independence.slice(0, separator);
  const independenceBody = separator === -1 ? "" : independence.slice(separator + 2);

  return (
    <div className="rf-stack rf-gap-md">
      <div className="rf-stack setup-wizard__intro">
        <p className="rf-title setup-wizard__title">{t("setup.welcome.title")}</p>
        <p className="rf-prose">{t("setup.welcome.body", { version: AUTOFIRMA_VERSION })}</p>
      </div>
      <div className="rf-card">
        <p className="rf-title setup-wizard__notice-title">{independenceTitle}</p>
        {independenceBody && <p className="rf-prose">{independenceBody}</p>}
      </div>
    </div>
  );
}

interface OutcomeProps {
  icon: ReactNode;
  title: string;
  children?: ReactNode;
  action?: ReactNode;
}

function Outcome({ icon, title, children, action }: OutcomeProps) {
  return (
    <div className="rf-row rf-gap-xs setup-wizard__outcome">
      <span className="setup-wizard__outcome-icon">{icon}</span>
      <div className="rf-stack setup-wizard__outcome-text">
        <p className="rf-prose setup-wizard__outcome-title">{title}</p>
        {children}
      </div>
      {action}
    </div>
  );
}

interface CertificateCardProps {
  t: TFunction;
  status: CertificateStatus;
  onInstall: () => void;
  onDecline: () => void;
}

function CertificateCard({ t, status, onInstall, onDecline }: CertificateCardProps) {
  return (
    <div className="rf-card setup-wizard__card">
      <p className="rf-title setup-wizard__card-title">{t("setup.certificate.title")}</p>
      <p className="rf-prose">{t("setup.certificate.body")}</p>

      {status.kind === "idle" && (
        <div className="rf-row rf-gap-xs setup-wizard__actions">
          <button type="button" className="rf-btn rf-btn--primary" onClick={onInstall}>
            {t("status.actions.install")}
          </button>
          <button type="button" className="rf-btn rf-btn--secondary" onClick={onDecline}>
            {t("setup.actions.notNow")}
          </button>
        </div>
      )}

      <div role="status">
        {status.kind === "working" && (
          <Outcome icon={<CheckingIcon size={18} />} title={t("setup.certificate.installing")} />
        )}
        {status.kind === "done" && (
          <Outcome
            icon={<CheckCircleIcon size={18} />}
            title={t("setup.certificate.installedTitle")}
          >
            {status.restartNotice && (
              <p className="rf-hint">{t("setup.certificate.installedRestartNotice")}</p>
            )}
          </Outcome>
        )}
        {status.kind === "failed" && (
          <Outcome
            icon={<AlertIcon size={18} />}
            title={t("setup.certificate.failedTitle")}
            action={
              <button type="button" className="rf-btn rf-btn--secondary" onClick={onInstall}>
                {t("status.withdrawal.retry")}
              </button>
            }
          >
            <ul className="rf-stack setup-wizard__stores">
              {status.detail.map((store) => (
                <li key={store.brand} className="rf-row rf-gap-xs rf-hint">
                  <span className="setup-wizard__store-mark">{store.trusted ? "✓" : "✗"}</span>
                  {storeBrandLabel(t, store.brand)}
                </li>
              ))}
            </ul>
          </Outcome>
        )}
      </div>
    </div>
  );
}

interface HandlerCardProps {
  t: TFunction;
  status: HandlerStatus;
  autoFirmaAppears: boolean;
  onUse: (target: string) => void;
  onDecline: () => void;
}

function HandlerCard({ t, status, autoFirmaAppears, onUse, onDecline }: HandlerCardProps) {
  return (
    <div className="rf-card setup-wizard__card">
      <p className="rf-title setup-wizard__card-title">{t("setup.handler.title")}</p>
      <p className="rf-prose">
        {t(autoFirmaAppears ? "setup.handler.body" : "setup.handler.bodyNoAutofirma")}
      </p>

      {status.kind === "idle" && (
        <div className="rf-row rf-gap-xs setup-wizard__actions">
          <button
            type="button"
            className="rf-btn rf-btn--primary"
            onClick={() => onUse(status.target)}
          >
            {t("setup.actions.useRfirma")}
          </button>
          <button type="button" className="rf-btn rf-btn--secondary" onClick={onDecline}>
            {t("setup.actions.notNow")}
          </button>
        </div>
      )}

      <div role="status">
        {status.kind === "done" && (
          <Outcome icon={<CheckCircleIcon size={18} />} title={t("setup.handler.done")} />
        )}
      </div>
    </div>
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
