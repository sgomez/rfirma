//! El pie fijo del panel, también tras firmar: el destino y, según el estado, «Firmar», «Reintentar» y «Volver» o las salidas de abrir el firmado.

import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import { AlertIcon, FileIcon, FolderIcon } from "../design-system/icons";
import { Row } from "../design-system/Row";
import type { Certificate, CertificateState } from "./certificate";
import { isUsable } from "./certificate";
import type { Destination } from "./destination";
import { shortenDestination } from "./destination";
import type { SigningFailure } from "./failure";

interface PanelFooterDestinationProps {
  destination: Destination;
  documentName: string;
}

interface PanelFooterSigningProps extends PanelFooterDestinationProps {
  moment?: undefined;
  failure: SigningFailure | null;
  onChangeDestination: () => void;
  signing: boolean;
  blocked: boolean;
  /** Si la firma local lo rechaza por certificado. */
  closed: boolean;
  certificate: CertificateState;
  onSign: () => void;
  /** Cierra el error y vuelve al panel, con el ciclo a medias olvidado en el backend. */
  onBack: () => void;
}

interface PanelFooterExitsProps extends PanelFooterDestinationProps {
  /** Abre el documento con el visor del sistema. */
  onOpenDocument: () => void;
  /** Abre la carpeta donde está, con las firmas anteriores dentro. */
  onOpenFolder: () => void;
  /** Vuelve al panel de firma con el original releído del disco. */
  onSign: () => void;
}

interface PanelFooterAcknowledgementProps extends PanelFooterExitsProps {
  moment: "acknowledgement";
  /** Mueve el destino del documento firmado (ADR-0011). */
  onChangeDestination: () => void;
}

interface PanelFooterReadingProps extends PanelFooterExitsProps {
  moment: "reading";
  /** Si el documento se puede firmar en el escritorio, que solo firma PDF. */
  signable: boolean;
  onChangeDestination?: never;
}

type PanelFooterProps =
  | PanelFooterSigningProps
  | PanelFooterAcknowledgementProps
  | PanelFooterReadingProps;

/** El mensaje de destino no escribible, con la carpeta en negrita. */
function unwritableMessage(message: string, folder: string) {
  const at = message.indexOf(folder);
  if (at < 0) {
    return message;
  }
  return (
    <>
      {message.slice(0, at)}
      <strong>{folder}</strong>
      {message.slice(at + folder.length)}
    </>
  );
}

/**
 * El pie del panel: 162 px en todos los estados
 * (docs/design/panel-de-firma.md § Pie fijo). El destino arriba —«Guardar
 * en» mientras se decide, «Documento» en el resumen— y, abajo, la fila de 44 px
 * con la acción del momento: «Firmar», «Reintentar»/«Volver», o los dos
 * caminos hasta el fichero firmado y «Firmar».
 */
export function PanelFooter(props: PanelFooterProps) {
  const { t } = useTranslation();
  const { destination, documentName } = props;
  const signed = props.moment !== undefined;
  const signing = props.moment === undefined && props.signing;
  const signable = props.moment !== "reading" || props.signable;
  const fullName = destination.name ?? documentName;
  const shortened = shortenDestination({ folder: destination.folder, name: fullName });
  const writable = signed || destination.writable;

  return (
    <footer className="panel__footer">
      <div className="panel__destination">
        <Row className="panel__destination-label-row">
          <p className="rf-label panel__destination-label">
            {t(signed ? "panel.signed.document" : "panel.footer.savedIn")}
          </p>
          {props.onChangeDestination !== undefined && (
            <Button
              variant="ghost"
              className={`panel__destination-change${signing ? " panel__controls--dim" : ""}`}
              onClick={props.onChangeDestination}
            >
              {t("actions.change")}
            </Button>
          )}
        </Row>
        {writable ? (
          <div className="panel__destination-box">
            {destination.folder !== "" && (
              <Row
                as="span"
                gap="xs"
                className="rf-text-muted panel__destination-folder"
                title={destination.folder}
              >
                <FolderIcon size={15} />
                <span className="panel__destination-ellipsis">{shortened.folder}</span>
              </Row>
            )}
            <Row as="span" gap="xs" className="panel__destination-name" title={fullName}>
              <FileIcon size={15} />
              <span className="panel__destination-ellipsis">{shortened.name}</span>
            </Row>
          </div>
        ) : (
          <Row gap="xs" className="panel__destination-unwritable">
            <AlertIcon size={16} />
            <span className="panel__destination-unwritable-text" title={destination.folder}>
              {unwritableMessage(
                t("panel.footer.unwritable", { folder: shortened.folder }),
                shortened.folder,
              )}
            </span>
          </Row>
        )}
      </div>
      {props.moment !== undefined ? (
        <Row gap="xs" className="panel__signed-actions">
          <Button variant="primary" className="panel__signed-open" onClick={props.onOpenDocument}>
            {t(signable ? "panel.signed.openDocument" : "panel.signed.openFile")}
          </Button>
          <Button
            variant="secondary"
            title={t("panel.signed.openFolder")}
            className="panel__signed-folder"
            onClick={props.onOpenFolder}
          >
            <FolderIcon />
          </Button>
          <Button
            variant="ghost"
            className="panel__signed-sign"
            title={signable ? undefined : t("panel.signed.onlyPdfs")}
            disabled={!signable}
            onClick={props.onSign}
          >
            {t("actions.sign")}
          </Button>
        </Row>
      ) : (
        <>
          {props.failure && (
            <Row gap="xs" className="panel__failure-actions">
              <Button variant="primary" className="panel__failure-retry" onClick={props.onSign}>
                {t("actions.retry")}
              </Button>
              <Button variant="ghost" className="panel__failure-back" onClick={props.onBack}>
                {t("actions.back")}
              </Button>
            </Row>
          )}
          {!props.failure && (
            <SignButton
              chosen={props.certificate.kind === "chosen" ? props.certificate.certificate : null}
              signing={props.signing}
              blocked={props.blocked}
              closed={props.closed}
              onSign={props.onSign}
            />
          )}
        </>
      )}
    </footer>
  );
}

interface SignButtonProps {
  chosen: Certificate | null;
  signing: boolean;
  /** Con el interruptor encendido y sin colocar, o con el rango en error. */
  blocked: boolean;
  closed: boolean;
  onSign: () => void;
}

function SignButton({ chosen, signing, blocked, closed, onSign }: SignButtonProps) {
  const { t } = useTranslation();
  const usable = chosen !== null && isUsable(chosen.status);
  return (
    <Row className="panel__sign-row">
      <Button
        variant="primary"
        className="panel__sign"
        title={signButtonTitle(t, closed)}
        disabled={signing || blocked || closed || !usable}
        onClick={onSign}
      >
        {t(signing ? "panel.footer.signing" : "actions.sign")}
      </Button>
    </Row>
  );
}

function signButtonTitle(t: TFunction, closed: boolean) {
  return closed ? t("panel.previousSignatures.closed") : undefined;
}
