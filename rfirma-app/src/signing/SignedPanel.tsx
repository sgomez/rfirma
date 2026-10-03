//! La columna derecha con todas las firmas del documento, tras firmarlo o al abrirlo para verlas (`verify --gui`), con sus tres salidas: abrir el documento, abrir la carpeta y volver a firmar.

import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { formatSignedAt, formatSignedTime } from "../App.signingOrder";
import { AlertIcon, CheckCircleIcon, FileIcon } from "../design-system/icons";
import type { NamedFailure } from "../errors/classify";
import { ErrorNotice } from "../errors/ErrorNotice";
import type { Destination } from "./destination";
import { PanelFooter } from "./PanelFooter";
import type { PreviousSignature, SignatureFormat } from "./previousSignatures";
import "./SigningPanel.css";
import "./SignedPanel.css";

interface SignedPanelProps {
  /** El nombre del fichero firmado. La ruta no se enseña nunca (ADR-0011). */
  documentName: string;
  /** El instante estampado en el recuadro, para «Firmado a las 11:04». Sin él es `verify --gui`. */
  signedAt?: Date;
  /** Las firmas del documento tal y como ha quedado, la propia la última. */
  signatures: readonly PreviousSignature[];
  destination: Destination;
  /** Abre el PDF firmado con el visor del sistema. */
  onOpenDocument: () => void;
  /** Abre la carpeta donde quedó, con las firmas anteriores dentro (ID-81). */
  onOpenFolder: () => void;
  /** Vuelve al panel de firma **con el original releído del disco** (ID-80). */
  onSign: () => void;
  /** Mueve el destino; ausente en `verify --gui`, donde el pie no tiene «Cambiar». */
  onChangeDestination?: () => void;
  /** El formato de firma del documento; por omisión, PAdES. */
  format?: SignatureFormat;
  /** Si se puede firmar desde aquí: en el escritorio, solo un PDF. */
  signable?: boolean;
  /** Se están leyendo las firmas del documento abierto. */
  reading?: boolean;
  /** Por qué no se pudieron leer las firmas del documento abierto. */
  readFailure?: NamedFailure | null;
  /** Por qué no se pudo abrir lo que se pidió, si es que se pidió algo y falló. */
  failure?: NamedFailure | null;
  onOpenHelp?: () => void;
}

const FORMAT_BADGES = {
  pades: "panel.signed.formatPades",
  cades: "panel.signed.formatCades",
  xades: "panel.signed.formatXades",
} as const;

/** La columna derecha cuando la firma ya está escrita: todas las firmas del documento. */
export function SignedPanel({
  documentName,
  signedAt,
  signatures,
  destination,
  onOpenDocument,
  onOpenFolder,
  onSign,
  onChangeDestination,
  format = "pades",
  signable = true,
  reading = false,
  readFailure = null,
  failure = null,
  onOpenHelp,
}: SignedPanelProps) {
  const { t, i18n } = useTranslation();
  const locale = i18n.language;

  return (
    <div className="panel">
      <div className="panel__scroll">
        {signedAt !== undefined && (
          <div className="rf-row rf-gap-xs signed-panel__signed-at">
            <CheckCircleIcon size={18} />
            <span className="rf-body">
              {t("panel.signed.signedAt", { time: formatSignedTime(signedAt, locale) })}
            </span>
          </div>
        )}

        {readFailure ? (
          <ErrorNotice
            situation={readFailure.situation}
            technicalDetail={readFailure.detail}
            title={t("panel.signed.readFailed.title")}
            onOpenHelp={onOpenHelp}
          />
        ) : !reading && format === "unrecognized" ? (
          <div className="panel__no-certificates">
            <div className="panel__notice-title">
              <AlertIcon size={18} />
              <span className="rf-title">{t("panel.signed.unrecognized.title")}</span>
            </div>
            <p className="rf-body rf-text-muted panel__notice-body">
              {t("panel.signed.unrecognized.body")}
            </p>
          </div>
        ) : !reading && signedAt === undefined && signatures.length === 0 ? (
          <div className="panel__no-certificates">
            <div className="panel__notice-title">
              <FileIcon size={18} />
              <span className="rf-title">{t("panel.signed.none.title")}</span>
            </div>
            <p className="rf-body rf-text-muted panel__notice-body">
              {t("panel.signed.none.body")}
            </p>
          </div>
        ) : (
          <section className="panel__section" aria-label={t("panel.signed.title")}>
            <p className="rf-row rf-gap-xs signed-panel__title">
              <FileIcon size={16} />
              <span>{t("panel.signed.title")}</span>
            </p>
            {!reading && (
              <div className="rf-row rf-gap-xs">
                {format !== "unrecognized" && (
                  <span className="rf-badge">{t(FORMAT_BADGES[format])}</span>
                )}
                <span className="rf-badge">{countBadge(signatures, t)}</span>
              </div>
            )}
            <ol className="signed-panel__cards">
              {signatures.map((signature, index) => (
                <SignatureCard
                  // biome-ignore lint/suspicious/noArrayIndexKey: el orden es la identidad de la firma.
                  key={index}
                  label={t("panel.signed.signature", { number: index + 1 })}
                  numbering={String(index + 1)}
                  signature={signature}
                  isNew={signedAt !== undefined && index === signatures.length - 1}
                  locale={locale}
                />
              ))}
            </ol>
          </section>
        )}

        {failure && (
          <ErrorNotice
            situation={failure.situation}
            technicalDetail={failure.detail}
            onOpenHelp={onOpenHelp}
          />
        )}
      </div>

      <PanelFooter
        signed
        destination={destination}
        documentName={documentName}
        onOpenDocument={onOpenDocument}
        onOpenFolder={onOpenFolder}
        onSign={onSign}
        onChangeDestination={onChangeDestination}
        signable={signable}
      />
    </div>
  );
}

interface SignatureCardProps {
  label: string;
  numbering: string;
  signature: PreviousSignature;
  isNew: boolean;
  locale: string;
}

function SignatureCard({ label, numbering, signature, isNew, locale }: SignatureCardProps) {
  return (
    <li className="rf-card signed-panel__card">
      <div className="rf-row signed-panel__card-head">
        <span className="rf-label">{label}</span>
        {isNew && <NewBadge />}
      </div>
      <SignatureRows signature={signature} locale={locale} />
      <Countersignatures signature={signature} numbering={numbering} locale={locale} />
    </li>
  );
}

function NewBadge() {
  const { t } = useTranslation();
  return <span className="rf-badge rf-badge--primary">{t("panel.signed.new")}</span>;
}

function Countersignatures({
  signature,
  numbering,
  locale,
}: {
  signature: PreviousSignature;
  numbering: string;
  locale: string;
}) {
  const { t } = useTranslation();
  return signature.countersignatures.map((countersignature, index) => {
    const path = `${numbering}.${index + 1}`;
    return (
      // biome-ignore lint/suspicious/noArrayIndexKey: el orden es la identidad de la firma.
      <div className="signed-panel__countersignature" key={index}>
        <span className="rf-label">{t("panel.signed.countersignature", { number: path })}</span>
        <SignatureRows signature={countersignature} locale={locale} />
        <Countersignatures signature={countersignature} numbering={path} locale={locale} />
      </div>
    );
  });
}

function SignatureRows({ signature, locale }: { signature: PreviousSignature; locale: string }) {
  const { t } = useTranslation();
  const signer = t("panel.signed.field.signer");
  const rows: [string, string][] = [
    [signer, signerOf(signature)],
    [t("panel.signed.field.onBehalfOf"), signature.organizationIdentifier ?? ""],
    [t("panel.signed.field.issuer"), signature.issuer],
    [
      t("panel.signed.field.declaredDate"),
      signature.signingTime === null ? "" : formatSignedAt(new Date(signature.signingTime), locale),
    ],
  ];

  return rows
    .filter(([, value]) => value !== "")
    .map(([label, value]) => (
      <div className="rf-row signed-panel__field" key={label}>
        <span className="rf-body rf-text-muted signed-panel__field-label">{label}</span>
        <span
          className={
            "rf-body signed-panel__field-value" +
            (label === signer ? " signed-panel__field-value--signer" : "")
          }
        >
          {value}
        </span>
      </div>
    ));
}

function countersignaturesIn(signatures: readonly PreviousSignature[]): number {
  return signatures.reduce(
    (total, signature) =>
      total + signature.countersignatures.length + countersignaturesIn(signature.countersignatures),
    0,
  );
}

function countBadge(signatures: readonly PreviousSignature[], t: TFunction): string {
  const signatureCount = t("panel.signed.count", { count: signatures.length });
  const countersignatureCount = countersignaturesIn(signatures);
  return countersignatureCount === 0
    ? signatureCount
    : `${signatureCount} · ${t("panel.signed.countersignatureCount", { count: countersignatureCount })}`;
}

function signerOf(signature: PreviousSignature): string {
  return signature.idNumber === "" ? signature.name : `${signature.name} (${signature.idNumber})`;
}
