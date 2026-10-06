//! El contenido de cada sección de Preferencias como componente propio: `GeneralSection`, `SigningSection`, `CertificatesSection` y `AppearanceSection`.

import type { TFunction } from "i18next";
import type { ReactNode } from "react";
import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import { Row } from "../design-system/Row";
import { Select } from "../design-system/Select";
import { Stack } from "../design-system/Stack";
import { Switch } from "../design-system/Switch";
import type { NamedFailure } from "../errors/classify";
import { ErrorNotice } from "../errors/ErrorNotice";
import { LANGUAGES, type LanguageTag } from "../i18n/languages";
import { CertificateCard } from "../signing/CertificateCard";
import type { Certificate } from "../signing/certificate";
import type { DestinationMode } from "./destinationMode";
import type { SaveFailure, Section } from "./PreferencesView";
import type { Preferences } from "./preferences";
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
      <Row className="preferences__heading-row">
        <p className="rf-label preferences__heading" id={headingId}>
          {t(`preferences.sections.${label}`)}
        </p>
        {action}
      </Row>
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
        <div className="preferences__options">
          <div className="preferences__option">
            <Switch
              checked={rememberActivity}
              label={t("preferences.rememberActivity.label")}
              hint={t("preferences.rememberActivity.hint")}
              wide
              onChange={onRememberActivityChange}
            />
            <Button variant="secondary" className="preferences__clear" onClick={onForgetClick}>
              {t("recents.clear")}
            </Button>
            {forgetFailure !== null && (
              <ErrorNotice situation="activityNotForgotten" technicalDetail={forgetFailure} />
            )}
          </div>
          <Switch
            checked={notifyNewVersion}
            label={t("preferences.notifyNewVersion.label")}
            wide
            onChange={onNotifyNewVersionChange}
          />
        </div>
      </fieldset>
      <SaveNotice section="general" saveFailure={saveFailure} />
    </>
  );
}

/** La carpeta de destino, con el botón que abre el selector del sistema. */
function DestinationFolderRow({
  destination,
  onChooseDestinationClick,
  t,
}: {
  destination: string;
  onChooseDestinationClick: () => void;
  t: TFunction;
}) {
  return (
    <Row gap="sm" className="preferences__destination-row">
      <p className="rf-prose preferences__destination-folder">{destination}</p>
      <Button variant="secondary" onClick={onChooseDestinationClick}>
        {t("preferences.destination.change")}
      </Button>
    </Row>
  );
}

/**
 * Dónde cae el siguiente firmado (ADR-0011): con `offersOriginalFolder`, un
 * grupo de radios entre los dos modos; sin él, solo la carpeta y su botón.
 */
function DestinationField({
  titleId,
  destinationName,
  preferences,
  onChooseDestinationClick,
  onDestinationModeChange,
}: {
  titleId: string;
  destinationName: string;
  preferences: Preferences;
  onChooseDestinationClick: () => void;
  onDestinationModeChange: (mode: DestinationMode) => void;
}) {
  const { t } = useTranslation();
  const labelId = `${titleId}-destination`;
  const folderRow = (
    <DestinationFolderRow
      destination={preferences.destination}
      onChooseDestinationClick={onChooseDestinationClick}
      t={t}
    />
  );
  return (
    <div className="preferences__destination">
      <p className="rf-prose preferences__destination-title" id={labelId}>
        {t("preferences.destination.label")}
      </p>
      {preferences.offersOriginalFolder ? (
        <Stack gap="xs" role="radiogroup" aria-labelledby={labelId}>
          <Row as="label" gap="xs" className="preferences__destination-radio">
            <input
              type="radio"
              name={destinationName}
              checked={preferences.destinationMode === "next_to_the_original"}
              onChange={() => onDestinationModeChange("next_to_the_original")}
            />
            <span className="rf-prose">{t("preferences.destination.nextToOriginal")}</span>
          </Row>
          <div className="preferences__destination-option">
            <Row as="label" gap="xs" className="preferences__destination-radio">
              <input
                type="radio"
                name={destinationName}
                checked={preferences.destinationMode === "in_the_destination_folder"}
                onChange={() => onDestinationModeChange("in_the_destination_folder")}
              />
              <span className="rf-prose">{t("preferences.destination.inThisFolder")}</span>
            </Row>
            <div className="preferences__destination-suboption">{folderRow}</div>
          </div>
        </Stack>
      ) : (
        folderRow
      )}
    </div>
  );
}

interface SigningSectionProps {
  titleId: string;
  saveFailure: SaveFailure | null;
  preferences: Preferences;
  onRememberVisibleSignatureChange: (checked: boolean) => void;
  onChooseDestinationClick: () => void;
  onDestinationModeChange: (mode: DestinationMode) => void;
  onConsentCountdownChange: (checked: boolean) => void;
  onHonourAutomaticSelectionChange: (checked: boolean) => void;
  onAllowSha1Change: (checked: boolean) => void;
}

export function SigningSection({
  titleId,
  saveFailure,
  preferences,
  onRememberVisibleSignatureChange,
  onChooseDestinationClick,
  onDestinationModeChange,
  onConsentCountdownChange,
  onHonourAutomaticSelectionChange,
  onAllowSha1Change,
}: SigningSectionProps) {
  const { t } = useTranslation();
  const destinationName = useId();
  return (
    <>
      <Heading label="signing" headingId={`${titleId}-heading-signing`} />
      <div className="preferences__options">
        <Switch
          checked={preferences.rememberVisibleSignature}
          label={t("preferences.rememberVisibleSignature.label")}
          wide
          onChange={onRememberVisibleSignatureChange}
        />
        <DestinationField
          titleId={titleId}
          destinationName={destinationName}
          preferences={preferences}
          onChooseDestinationClick={onChooseDestinationClick}
          onDestinationModeChange={onDestinationModeChange}
        />
        <Switch
          checked={preferences.consentCountdown}
          label={t("preferences.consentCountdown.label")}
          wide
          onChange={onConsentCountdownChange}
        />
        <Switch
          checked={preferences.honourAutomaticSelection}
          label={t("preferences.honourAutomaticSelection.label")}
          wide
          onChange={onHonourAutomaticSelectionChange}
        />
        <Switch
          checked={preferences.allowSha1}
          label={t("preferences.allowSha1.label")}
          hint={t("preferences.allowSha1.hint")}
          wide
          onChange={onAllowSha1Change}
        />
      </div>
      <SaveNotice section="signing" saveFailure={saveFailure} />
    </>
  );
}

interface CertificatesSectionProps {
  titleId: string;
  certificateFailure: NamedFailure | null;
  installedCertificates: readonly Certificate[];
  onAddClick: () => void;
  onRemoveClick: (certificate: Certificate) => void;
  onEmptyStoreClick: () => void;
}

export function CertificatesSection({
  titleId,
  certificateFailure,
  installedCertificates,
  onAddClick,
  onRemoveClick,
  onEmptyStoreClick,
}: CertificatesSectionProps) {
  const { t } = useTranslation();
  return (
    <>
      <Heading
        label="certificates"
        headingId={`${titleId}-heading-certificates`}
        action={
          <Button variant="secondary" className="preferences__add-certificate" onClick={onAddClick}>
            {t("preferences.certificates.add")}
          </Button>
        }
      />
      {certificateFailure !== null && (
        <ErrorNotice
          situation={certificateFailure.situation}
          onEmptyStore={onEmptyStoreClick}
          technicalDetail={
            certificateFailure.situation === "keyKindUnsupported" ||
            certificateFailure.situation === "pkcs12NoPrivateKey"
              ? undefined
              : certificateFailure.detail
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
            <Row as="li" className="preferences__certificate" key={certificate.id}>
              <CertificateCard certificate={certificate} />
              <Button
                variant="ghost"
                className="preferences__remove-certificate"
                aria-label={t("preferences.certificates.remove", {
                  holder: certificate.holderName,
                })}
                onClick={() => onRemoveClick(certificate)}
              >
                {t("actions.remove")}
              </Button>
            </Row>
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
      <div className="preferences__options">
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
      </div>
      <SaveNotice section="appearance" saveFailure={saveFailure} />
    </>
  );
}
