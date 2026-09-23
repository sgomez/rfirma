import type { TFunction } from "i18next";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { NamedFailure } from "../errors/classify";
import { ErrorNotice } from "../errors/ErrorNotice";
import { LANGUAGES, type LanguageTag } from "../i18n/languages";
import type { Certificate } from "../signing/certificate";
import type { SaveFailure, Section } from "./PreferencesView";
import type { Preferences } from "./preferences";
import { Select } from "./Select";
import { Switch } from "./Switch";
import { THEMES } from "./theme";

/** El título de página que abre cada panel: versalitas, con su divisoria debajo. */
function Heading({
  label,
  headingId,
  action,
}: {
  label: Section;
  headingId: string;
  action?: ReactNode;
}) {
  const { t } = useTranslation();
  return (
    <>
      <div className="rf-row preferences__heading-row">
        <p className="rf-label preferences__heading" id={headingId}>
          {t(`preferences.sections.${label}`)}
        </p>
        {action}
      </div>
      <hr className="rf-divider" />
    </>
  );
}

/** El encabezado de un grupo dentro de un panel: sin divisoria, en caja baja. */
function GroupHeading({ label, headingId }: { label: "privacy"; headingId: string }) {
  const { t } = useTranslation();
  return (
    <p className="rf-title preferences__group-heading" id={headingId}>
      {t(`preferences.sections.${label}`)}
    </p>
  );
}

/** El aviso de guardado de una sección, o nada si el fallo fue en otra. */
function SaveNotice({
  section,
  saveFailure,
}: {
  section: Section;
  saveFailure: SaveFailure | null;
}) {
  if (saveFailure?.section !== section) return null;
  return <ErrorNotice situation="settingNotSaved" technicalDetail={saveFailure.detail} />;
}

interface GeneralSectionProps {
  titleId: string;
  saveFailure: SaveFailure | null;
  rememberActivity: boolean;
  onRememberActivityChange: (checked: boolean) => void;
  onForgetClick: () => void;
  forgetFailure: string | null;
  notifyNewVersion: boolean;
  onNotifyNewVersionChange: (checked: boolean) => void;
}

export function GeneralSection({
  titleId,
  saveFailure,
  rememberActivity,
  onRememberActivityChange,
  onForgetClick,
  forgetFailure,
  notifyNewVersion,
  onNotifyNewVersionChange,
}: GeneralSectionProps) {
  const { t } = useTranslation();
  return (
    <>
      <Heading label="general" headingId={`${titleId}-heading-general`} />
      <fieldset className="preferences__group" aria-labelledby={`${titleId}-heading-privacy`}>
        <GroupHeading label="privacy" headingId={`${titleId}-heading-privacy`} />
        <Switch
          checked={rememberActivity}
          label={t("preferences.rememberActivity.label")}
          hint={t("preferences.rememberActivity.hint")}
          wide
          onChange={onRememberActivityChange}
        />
        <button
          type="button"
          className="rf-btn rf-btn--secondary preferences__clear"
          onClick={onForgetClick}
        >
          {t("preferences.rememberActivity.clear")}
        </button>
        {forgetFailure !== null && (
          <ErrorNotice situation="activityNotForgotten" technicalDetail={forgetFailure} />
        )}
        <Switch
          checked={notifyNewVersion}
          label={t("preferences.notifyNewVersion.label")}
          wide
          onChange={onNotifyNewVersionChange}
        />
      </fieldset>
      <SaveNotice section="general" saveFailure={saveFailure} />
    </>
  );
}

interface SigningSectionProps {
  titleId: string;
  saveFailure: SaveFailure | null;
  preferences: Preferences;
  onRememberVisibleSignatureChange: (checked: boolean) => void;
  onChooseDestinationClick: () => void;
  onConsentCountdownChange: (checked: boolean) => void;
}

export function SigningSection({
  titleId,
  saveFailure,
  preferences,
  onRememberVisibleSignatureChange,
  onChooseDestinationClick,
  onConsentCountdownChange,
}: SigningSectionProps) {
  const { t } = useTranslation();
  return (
    <>
      <Heading label="signing" headingId={`${titleId}-heading-signing`} />
      <Switch
        checked={preferences.rememberVisibleSignature}
        label={t("preferences.rememberVisibleSignature.label")}
        hint={t("preferences.rememberVisibleSignature.hint")}
        wide
        onChange={onRememberVisibleSignatureChange}
      />
      <div className="preferences__destination">
        <p className="rf-label" id={`${titleId}-destination`}>
          {t("preferences.destination.label")}
        </p>
        {preferences.offersOriginalFolder && (
          <p className="rf-prose preferences__destination-note">
            {t("preferences.destination.nextToOriginal")}
          </p>
        )}
        <div className="rf-row rf-gap-sm preferences__destination-row">
          {preferences.offersOriginalFolder && (
            <span className="rf-prose preferences__destination-mode-label">
              {t("preferences.destination.inThisFolder")}
            </span>
          )}
          <p className="rf-prose preferences__destination-folder">{preferences.destination}</p>
          <button
            type="button"
            className="rf-btn rf-btn--secondary"
            onClick={onChooseDestinationClick}
          >
            {t("preferences.destination.change")}
          </button>
        </div>
      </div>
      <Switch
        checked={preferences.consentCountdown}
        label={t("preferences.consentCountdown.label")}
        wide
        onChange={onConsentCountdownChange}
      />
      <SaveNotice section="signing" saveFailure={saveFailure} />
    </>
  );
}

/**
 * Lo que identifica cada fila **es el certificado, no el fichero**: del
 * `.p12` no se recuerda nada, ni la ruta (ID-196), así que aquí no hay ni
 * ruta ni «volver a localizar». La fecha de caducidad va en la misma línea
 * que el DNI y el emisor; un caducado la cambia por su insignia.
 */
function certificateLine(certificate: Certificate, t: TFunction, locale: string) {
  return [
    certificate.idNumber,
    t("panel.certificate.issuer", { issuer: certificate.issuer }),
    certificate.status.kind === "valid"
      ? t("preferences.certificates.expires", {
          date: new Intl.DateTimeFormat(locale, { dateStyle: "long" }).format(
            certificate.status.notAfter * 1000,
          ),
        })
      : null,
  ]
    .filter((piece) => piece !== null && piece !== "")
    .join(" · ");
}

interface CertificatesSectionProps {
  titleId: string;
  certificateFailure: NamedFailure | null;
  installedCertificates: readonly Certificate[];
  onAddClick: () => void;
  onRemoveClick: (certificate: Certificate) => void;
}

export function CertificatesSection({
  titleId,
  certificateFailure,
  installedCertificates,
  onAddClick,
  onRemoveClick,
}: CertificatesSectionProps) {
  const { t, i18n } = useTranslation();
  return (
    <>
      <Heading
        label="certificates"
        headingId={`${titleId}-heading-certificates`}
        action={
          <button
            type="button"
            className="rf-btn rf-btn--secondary preferences__add-certificate"
            onClick={onAddClick}
          >
            {t("preferences.certificates.add")}
          </button>
        }
      />
      {certificateFailure !== null && (
        <ErrorNotice
          situation={certificateFailure.situation}
          technicalDetail={
            certificateFailure.situation === "keyNotRsa" ? undefined : certificateFailure.detail
          }
        />
      )}
      {installedCertificates.length === 0 ? (
        <p className="rf-prose preferences__certificates-empty">
          {t("preferences.certificates.empty")}
        </p>
      ) : (
        <ul className="preferences__certificates">
          {installedCertificates.map((certificate) => (
            <li className="rf-row preferences__certificate" key={certificate.id}>
              <span className="preferences__certificate-text">
                <span className="rf-title preferences__certificate-holder">
                  {certificate.holderName}
                  {certificate.status.kind === "expired" && (
                    <span className="rf-badge">{t("preferences.certificates.expired")}</span>
                  )}
                </span>
                <span className="rf-body rf-text-muted">
                  {certificateLine(certificate, t, i18n.language)}
                </span>
              </span>
              <button
                type="button"
                className="rf-btn rf-btn--ghost preferences__remove-certificate"
                aria-label={t("preferences.certificates.remove", {
                  holder: certificate.holderName,
                })}
                onClick={() => onRemoveClick(certificate)}
              >
                {t("actions.remove")}
              </button>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}

interface AppearanceSectionProps {
  titleId: string;
  saveFailure: SaveFailure | null;
  theme: Preferences["theme"];
  onThemeChange: (theme: Preferences["theme"]) => void;
  language: LanguageTag;
  onLanguageChange: (language: LanguageTag) => void;
}

export function AppearanceSection({
  titleId,
  saveFailure,
  theme,
  onThemeChange,
  language,
  onLanguageChange,
}: AppearanceSectionProps) {
  const { t } = useTranslation();
  return (
    <>
      <Heading label="appearance" headingId={`${titleId}-heading-appearance`} />
      <Select
        label={t("preferences.theme.label")}
        value={theme}
        options={THEMES.map((option) => ({
          value: option,
          label: t(`preferences.theme.${option}`),
        }))}
        onChange={onThemeChange}
      />
      <Select
        label={t("preferences.language.label")}
        value={language}
        options={LANGUAGES.map((tag) => ({
          value: tag,
          label: t(`languages.${tag}`),
        }))}
        onChange={onLanguageChange}
      />
      <SaveNotice section="appearance" saveFailure={saveFailure} />
    </>
  );
}
