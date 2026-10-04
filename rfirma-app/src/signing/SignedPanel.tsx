//! La columna derecha con todas las firmas del documento, tras firmarlo o al abrirlo para verlas (`verify --gui`), con sus tres salidas: abrir el documento, abrir la carpeta y volver a firmar.

import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { formatSignedTime } from "../App.signingOrder";
import { AlertIcon, CheckCircleIcon, FileIcon } from "../design-system/icons";
import type { NamedFailure } from "../errors/classify";
import { ErrorNotice } from "../errors/ErrorNotice";
import type { Destination } from "./destination";
import { PanelFooter } from "./PanelFooter";
import type { DocumentFinding, PreviousSignature, SignatureFormat } from "./previousSignatures";
import { SignatureCards } from "./SignatureCards";
import "./SigningPanel.css";
import "./SignedPanel.css";

interface SignedPanelProps {
  /** El nombre del fichero firmado. La ruta no se enseña nunca (ADR-0011). */
  documentName: string;
  /** El instante estampado en el recuadro, para «Firmado a las 11:04». Sin él es `verify --gui`. */
  signedAt?: Date;
  /** Las firmas del documento tal y como ha quedado, la propia la última. */
  signatures: readonly PreviousSignature[];
  /** Los hallazgos del documento, que no son de ninguna firma. */
  findings?: readonly DocumentFinding[];
  destination: Destination;
  /** Abre el PDF firmado con el visor del sistema. */
  onOpenDocument: () => void;
  /** Abre la carpeta donde quedó, con las firmas anteriores dentro. */
  onOpenFolder: () => void;
  /** Vuelve al panel de firma **con el original releído del disco**. */
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

const FORMAT_BADGES = { pades: "PAdES", cades: "CAdES", xades: "XAdES" } as const;

/** La columna derecha cuando la firma ya está escrita: todas las firmas del documento. */
export function SignedPanel({
  documentName,
  signedAt,
  signatures,
  findings = [],
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
                  <span className="rf-badge">{FORMAT_BADGES[format]}</span>
                )}
                <span className="rf-badge">{countBadge(signatures, t)}</span>
              </div>
            )}
            <SignatureCards
              signatures={signatures}
              findings={findings}
              justSigned={signedAt !== undefined}
            />
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
