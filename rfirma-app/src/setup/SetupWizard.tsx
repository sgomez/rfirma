//! El asistente del primer arranque, en dos pasos —bienvenida y configuración—, que no se monta una vez visto (`Preferences.setupWizardSeen`); usa los casos de uso del panel de estado.

import type { TFunction } from "i18next";
import { type ReactNode, useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import { Card } from "../design-system/Card";
import { AlertIcon, CheckIcon, SpinnerIcon } from "../design-system/icons";
import { Row } from "../design-system/Row";
import { Select } from "../design-system/Select";
import { Stack } from "../design-system/Stack";
import { Switch } from "../design-system/Switch";
import { classify } from "../errors/classify";
import { ErrorNotice } from "../errors/ErrorNotice";
import { useLanguage } from "../i18n/LanguageProvider";
import { LANGUAGES, type LanguageTag } from "../i18n/languages";
import type { PreferencesStore } from "../preferences/preferences";
import { Header } from "../shell/Header";
import { type MenuAnchor, menuAnchorFor } from "../shell/menuAnchor";
import "./SetupWizard.css";
import {
  type SignalRow,
  type StatusPort,
  type StoreDetail,
  storeBrandLabel,
  withLocalCaCertificateMeasured,
} from "../status/status";

/** La versión de AutoFirma con la que rFirma se anuncia compatible (docs/afirma/1.9.2/). */
const AUTOFIRMA_VERSION = "1.9.2";

interface SetupWizardProps {
  /** Si el asistente ya se ha visto en un arranque anterior: entonces no se monta. */
  seen: boolean;
  preferences: PreferencesStore;
  statusPort: StatusPort;
  /** Se llama una vez, al pulsar «Terminar», pase lo que pase con las dos acciones. */
  onFinish: () => void;
  /** Dónde va el menú de la cabecera. Ver [`MenuAnchor`]. */
  menuAnchor?: MenuAnchor;
  /** Las cuatro entradas del menú de la cabecera (ADR-0007), iguales a las de la ventana principal. */
  onOpenStatus?: () => void;
  onOpenPreferences?: () => void;
  onOpenHelp?: () => void;
  onOpenAbout?: () => void;
  /** El paso con el que se abre; por omisión, la bienvenida. */
  initialStep?: 1 | 2;
}

type CertificateStatus =
  | { kind: "reading" }
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
  preferences,
  statusPort,
  onFinish,
  menuAnchor,
  onOpenStatus = () => {},
  onOpenPreferences = () => {},
  onOpenHelp = () => {},
  onOpenAbout = () => {},
  initialStep = 1,
}: SetupWizardProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState<1 | 2>(initialStep);
  const [certificate, setCertificate] = useState<CertificateStatus>({ kind: "reading" });
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
        setCertificate(
          certificateRow?.verdict === "correct"
            ? { kind: "done", restartNotice: certificateRow.restartFirefoxNotice }
            : { kind: "idle" },
        );
        const handlerRow = rows.find((row) => row.signal === "siteSignature");
        if (handlerRow) {
          setAutoFirmaAppears(
            handlerRow.candidates === null ||
              handlerRow.candidates.some((candidate) => candidate.name === "AutoFirma"),
          );
          setHandler(initialHandlerStatus(handlerRow));
        }
      })
      .catch(() => {
        if (!cancelled) setCertificate({ kind: "idle" });
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
          : { kind: "failed", detail: trustedStoresOf(row) },
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
            : { kind: "failed", detail: trustedStoresOf(certificateRow) },
        );
      }
    });
  };

  return (
    <div className="setup-wizard">
      <Header
        menuAnchor={menuAnchor ?? menuAnchorFor(navigator.userAgent)}
        onOpenStatus={onOpenStatus}
        onOpenPreferences={onOpenPreferences}
        onOpenHelp={onOpenHelp}
        onOpenAbout={onOpenAbout}
      />

      <div className="setup-wizard__body">
        <Stack gap="md" className="setup-wizard__column">
          <Row gap="xs">
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
          </Row>

          {step === 1 && <WelcomeScreen t={t} />}
          {step === 2 && (
            <div className="setup-wizard__steps">
              <CertificateStep
                t={t}
                status={certificate}
                onInstall={installCertificate}
                onDecline={() => setCertificate({ kind: "declined" })}
              />
              <HandlerStep
                t={t}
                status={handler}
                autoFirmaAppears={autoFirmaAppears}
                onUse={useRfirmaAsHandler}
                onDecline={() => setHandler({ kind: "declined" })}
              />
              <ProtectionSetting t={t} preferences={preferences} />
            </div>
          )}
        </Stack>
      </div>

      <Row gap="sm" className="setup-wizard__footer">
        {step === 1 && (
          <Button variant="ghost" onClick={onFinish}>
            {t("setup.actions.skip")}
          </Button>
        )}
        {step === 2 && (
          <Button variant="secondary" onClick={() => setStep(1)}>
            {t("actions.back")}
          </Button>
        )}
        {step === 1 ? (
          <Button ref={continueButton} variant="primary" onClick={() => setStep(2)}>
            {t("actions.continue")}
          </Button>
        ) : (
          <Button variant="primary" onClick={onFinish}>
            {t("setup.actions.finish")}
          </Button>
        )}
      </Row>
    </div>
  );
}

function WelcomeScreen({ t }: { t: TFunction }) {
  return (
    <Stack gap="md">
      <Stack className="setup-wizard__intro">
        <p className="rf-title setup-wizard__title">{t("setup.welcome.title")}</p>
        <p className="rf-prose">{t("setup.welcome.body", { version: AUTOFIRMA_VERSION })}</p>
      </Stack>
      <Card>
        <p className="rf-title setup-wizard__notice-title">{t("about.independenceLead")}</p>
        <p className="rf-prose">{t("about.independence")}</p>
      </Card>
      <LanguageCard t={t} />
    </Stack>
  );
}

function LanguageCard({ t }: { t: TFunction }) {
  const { language, setLanguage } = useLanguage();
  const [saveFailure, setSaveFailure] = useState<string | null>(null);

  const choose = async (chosen: LanguageTag) => {
    setSaveFailure(null);
    try {
      await setLanguage(chosen);
    } catch (thrown) {
      setSaveFailure(classify(thrown).detail);
    }
  };

  return (
    <Card className="setup-wizard__card">
      <p className="rf-title setup-wizard__card-title">{t("preferences.language.label")}</p>
      <Select
        label={t("preferences.language.label")}
        hideLabel
        opens="up"
        value={language}
        options={LANGUAGES.map((tag) => ({ value: tag, label: t(`languages.${tag}`) }))}
        onChange={(chosen) => void choose(chosen)}
      />
      {saveFailure !== null && (
        <ErrorNotice situation="settingNotSaved" technicalDetail={saveFailure} />
      )}
    </Card>
  );
}

type StepMarkerState = "pending" | "working" | "done" | "failed";

interface StepProps {
  number: 1 | 2;
  markerState: StepMarkerState;
  title: string;
  hint?: string;
  last?: boolean;
  children: ReactNode;
}

function Step({ number, markerState, title, hint, last = false, children }: StepProps) {
  return (
    <Row gap="sm" className="setup-wizard__step-row">
      <div className="setup-wizard__step-marker-column">
        <StepMarker number={number} state={markerState} />
        {!last && <span aria-hidden="true" className="setup-wizard__step-line" />}
      </div>
      <Stack className="setup-wizard__step-content">
        <p className="rf-prose setup-wizard__step-title">{title}</p>
        {hint && <p className="rf-hint">{hint}</p>}
        <div className="setup-wizard__step-action">{children}</div>
      </Stack>
    </Row>
  );
}

function StepMarker({ number, state }: { number: 1 | 2; state: StepMarkerState }) {
  if (state === "done") {
    return (
      <span className="setup-wizard__step-marker setup-wizard__step-marker--done">
        <CheckIcon size={14} />
      </span>
    );
  }
  if (state === "working") {
    return (
      <span className="setup-wizard__step-marker setup-wizard__step-marker--working">
        <SpinnerIcon size={14} />
      </span>
    );
  }
  if (state === "failed") {
    return (
      <span className="setup-wizard__step-marker">
        <AlertIcon size={14} />
      </span>
    );
  }
  return <span className="setup-wizard__step-marker">{number}</span>;
}

interface CertificateStepProps {
  t: TFunction;
  status: CertificateStatus;
  onInstall: () => void;
  onDecline: () => void;
}

function CertificateStep({ t, status, onInstall, onDecline }: CertificateStepProps) {
  const markerState: StepMarkerState =
    status.kind === "working" || status.kind === "done" || status.kind === "failed"
      ? status.kind
      : "pending";

  return (
    <Step
      number={1}
      markerState={markerState}
      title={t("status.signals.localCaCertificate")}
      hint={t("setup.certificate.body")}
    >
      {status.kind === "idle" && (
        <Row gap="xs">
          <Button variant="primary" onClick={onInstall}>
            {t("status.actions.install")}
          </Button>
          <Button variant="secondary" onClick={onDecline}>
            {t("actions.notNow")}
          </Button>
        </Row>
      )}
      {(status.kind === "working" || status.kind === "done" || status.kind === "failed") && (
        <Stack gap="xs" role="status">
          {status.kind === "working" && (
            <p className="rf-prose setup-wizard__step-outcome">
              {t("setup.certificate.installing")}
            </p>
          )}
          {status.kind === "done" && (
            <>
              <p className="rf-prose setup-wizard__step-outcome">
                {t("setup.certificate.installedTitle")}
              </p>
              {status.restartNotice && (
                <p className="rf-hint">{t("status.notices.restartFirefox")}</p>
              )}
            </>
          )}
          {status.kind === "failed" && (
            <>
              <p className="rf-prose setup-wizard__step-outcome">
                {t("setup.certificate.failedTitle")}
              </p>
              <ul className="rf-stack setup-wizard__stores">
                {status.detail.map((store) => (
                  <li key={store.brand} className="rf-row rf-gap-xs rf-hint">
                    <span className="setup-wizard__store-mark">{store.trusted ? "✓" : "✗"}</span>
                    {storeBrandLabel(t, store.brand)}
                  </li>
                ))}
              </ul>
              <Row>
                <Button variant="secondary" onClick={onInstall}>
                  {t("actions.retry")}
                </Button>
              </Row>
            </>
          )}
        </Stack>
      )}
    </Step>
  );
}

interface HandlerStepProps {
  t: TFunction;
  status: HandlerStatus;
  autoFirmaAppears: boolean;
  onUse: (target: string) => void;
  onDecline: () => void;
}

function HandlerStep({ t, status, autoFirmaAppears, onUse, onDecline }: HandlerStepProps) {
  const markerState: StepMarkerState =
    status.kind === "working" ? "working" : status.kind === "done" ? "done" : "pending";

  return (
    <Step
      number={2}
      markerState={markerState}
      title={t("setup.handler.title")}
      hint={autoFirmaAppears ? t("setup.handler.body") : undefined}
      last
    >
      {status.kind === "idle" && (
        <Row gap="xs">
          <Button variant="primary" onClick={() => onUse(status.target)}>
            {t("status.actions.useRfirma")}
          </Button>
          <Button variant="secondary" onClick={onDecline}>
            {t("actions.notNow")}
          </Button>
        </Row>
      )}
      {status.kind === "done" && (
        <p className="rf-prose setup-wizard__step-outcome" role="status">
          {t("setup.handler.done")}
        </p>
      )}
    </Step>
  );
}

function initialHandlerStatus(row: SignalRow): HandlerStatus {
  if (row.verdict === "correct") return { kind: "done" };
  if (row.action?.kind === "choice") return { kind: "idle", target: row.action.target };
  return { kind: "unavailable" };
}

function ProtectionSetting({ t, preferences }: { t: TFunction; preferences: PreferencesStore }) {
  const [enabled, setEnabled] = useState(true);
  const titleId = useId();

  useEffect(() => {
    let cancelled = false;
    preferences.read().then((read) => {
      if (!cancelled) setEnabled(read.consentCountdown);
    });
    return () => {
      cancelled = true;
    };
  }, [preferences]);

  const change = async (next: boolean) => {
    setEnabled(next);
    try {
      const current = await preferences.read();
      await preferences.save({ ...current, consentCountdown: next });
    } catch {
      setEnabled(!next);
    }
  };

  return (
    <>
      <hr className="rf-divider setup-wizard__divider" />
      <Row gap="sm" className="setup-wizard__protection">
        <Stack className="setup-wizard__protection-text">
          <p className="rf-prose setup-wizard__step-title" id={titleId}>
            {t("preferences.consentCountdown.label")}
          </p>
        </Stack>
        <Switch
          labelledBy={titleId}
          checked={enabled}
          className="setup-wizard__protection-switch"
          onChange={(next) => void change(next)}
        />
      </Row>
    </>
  );
}

function trustedStoresOf(row: SignalRow): StoreDetail[] {
  return row.detail?.kind === "trust" ? row.detail.stores : [];
}
