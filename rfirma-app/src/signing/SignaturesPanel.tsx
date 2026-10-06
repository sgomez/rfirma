//! La columna derecha con todas las firmas del documento en sus dos momentos, el acuse tras firmarlo y la lectura de firmas al abrirlo para verlas (`verify --gui`), con sus tres salidas: abrir el documento, abrir la carpeta y volver a firmar.

import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { Badge } from "../design-system/Badge";
import { AlertIcon, CheckCircleIcon, FileIcon } from "../design-system/icons";
import { Row } from "../design-system/Row";
import type { NamedFailure } from "../errors/classify";
import { ErrorNotice } from "../errors/ErrorNotice";
import type { Destination } from "./destination";
import { PanelFooter } from "./PanelFooter";
import type { DocumentFinding, PreviousSignature, SignatureFormat } from "./previousSignatures";
import { SignatureCards } from "./SignatureCards";
import { formatSignedTime } from "./signedAt";
import type { ReadingState } from "./useSignatureReading";
import "./SigningPanel.css";
import "./SignaturesPanel.css";

/** El acuse: el documento recién firmado aquí, con su destino movible. */
interface Acknowledgement {
  kind: "acknowledgement";
  /** El instante estampado en el recuadro, para «Firmado a las 11:04». */
  signedAt: Date;
  /** Las firmas del documento tal y como ha quedado, la propia la última. */
  signatures: readonly PreviousSignature[];
  /** Los hallazgos del documento, que no son de ninguna firma. */
  findings: readonly DocumentFinding[];
  /** Mueve el destino del documento firmado (ADR-0011). */
  onChangeDestination: () => void;
}

/** La lectura de firmas: el documento abierto solo para verlas, sin «Cambiar». */
interface Reading {
  kind: "reading";
  state: ReadingState;
  /** Si se puede firmar desde aquí: en el escritorio, solo un PDF. */
  signable: boolean;
  onChangeDestination?: never;
}

/** El momento del panel de firmas: el acuse o la lectura de firmas. */
export type SignaturesMoment = Acknowledgement | Reading;

interface SignaturesPanelProps {
  /** El nombre del fichero. La ruta no se enseña nunca (ADR-0011). */
  documentName: string;
  destination: Destination;
  moment: SignaturesMoment;
  /** Abre el documento con el visor del sistema. */
  onOpenDocument: () => void;
  /** Abre la carpeta donde está, con las firmas anteriores dentro. */
  onOpenFolder: () => void;
  /** Vuelve al panel de firma **con el original releído del disco**. */
  onSign: () => void;
  /** Por qué no se pudo abrir lo que se pidió, si es que se pidió algo y falló. */
  failure: NamedFailure | null;
  onOpenHelp?: () => void;
}

const FORMAT_BADGES = { pades: "PAdES", cades: "CAdES", xades: "XAdES" } as const;

/** La columna derecha con todas las firmas del documento, en el acuse o en la lectura de firmas. */
export function SignaturesPanel({
  documentName,
  destination,
  moment,
  onOpenDocument,
  onOpenFolder,
  onSign,
  failure,
  onOpenHelp,
}: SignaturesPanelProps) {
  const exits = { destination, documentName, onOpenDocument, onOpenFolder, onSign };

  return (
    <div className="panel">
      <div className="panel__scroll">
        {moment.kind === "acknowledgement" ? (
          <AcknowledgementBody moment={moment} />
        ) : (
          <ReadingBody state={moment.state} onOpenHelp={onOpenHelp} />
        )}

        {failure && (
          <ErrorNotice
            situation={failure.situation}
            technicalDetail={failure.detail}
            onOpenHelp={onOpenHelp}
          />
        )}
      </div>

      {moment.kind === "acknowledgement" ? (
        <PanelFooter
          moment="acknowledgement"
          {...exits}
          onChangeDestination={moment.onChangeDestination}
        />
      ) : (
        <PanelFooter moment="reading" {...exits} signable={moment.signable} />
      )}
    </div>
  );
}

function AcknowledgementBody({ moment }: { moment: Acknowledgement }) {
  const { t, i18n } = useTranslation();
  return (
    <>
      <Row gap="xs" className="signed-panel__signed-at">
        <CheckCircleIcon size={18} />
        <span className="rf-body">
          {t("panel.signed.signedAt", { time: formatSignedTime(moment.signedAt, i18n.language) })}
        </span>
      </Row>
      <SignaturesSummary
        signatures={moment.signatures}
        findings={moment.findings}
        format="pades"
        justSigned
      />
    </>
  );
}

function ReadingBody({ state, onOpenHelp }: { state: ReadingState; onOpenHelp?: () => void }) {
  const { t } = useTranslation();
  if (state.kind === "failed") {
    return (
      <ErrorNotice
        situation={state.failure.situation}
        technicalDetail={state.failure.detail}
        title={t("panel.signed.readFailed.title")}
        onOpenHelp={onOpenHelp}
      />
    );
  }
  if (state.kind === "reading") {
    return <SignaturesSummary signatures={[]} findings={[]} justSigned={false} />;
  }
  if (state.format === "unrecognized") {
    return (
      <div className="panel__no-certificates">
        <div className="panel__notice-title">
          <AlertIcon size={18} />
          <span className="rf-title">{t("panel.signed.unrecognized.title")}</span>
        </div>
        <p className="rf-body rf-text-muted panel__notice-body">
          {t("panel.signed.unrecognized.body")}
        </p>
      </div>
    );
  }
  if (state.signatures.length === 0) {
    return (
      <div className="panel__no-certificates">
        <div className="panel__notice-title">
          <FileIcon size={18} />
          <span className="rf-title">{t("panel.signed.none.title")}</span>
        </div>
      </div>
    );
  }
  return (
    <SignaturesSummary
      signatures={state.signatures}
      findings={state.findings}
      format={state.format}
      justSigned={false}
    />
  );
}

interface SignaturesSummaryProps {
  signatures: readonly PreviousSignature[];
  findings: readonly DocumentFinding[];
  /** Sin formato, mientras se leen, el resumen no lleva sus distintivos. */
  format?: Exclude<SignatureFormat, "unrecognized">;
  justSigned: boolean;
}

function SignaturesSummary({ signatures, findings, format, justSigned }: SignaturesSummaryProps) {
  const { t } = useTranslation();
  return (
    <section className="panel__section" aria-label={t("panel.signed.title")}>
      <p className="rf-row rf-gap-xs signed-panel__title">
        <FileIcon size={16} />
        <span>{t("panel.signed.title")}</span>
      </p>
      {format !== undefined && (
        <Row gap="xs">
          <Badge>{FORMAT_BADGES[format]}</Badge>
          <Badge>{countBadge(signatures, t)}</Badge>
        </Row>
      )}
      <SignatureCards signatures={signatures} findings={findings} justSigned={justSigned} />
    </section>
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
