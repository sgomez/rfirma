//! El pie fijo del panel, también tras firmar: el destino y, según el estado, «Firmar», «Reintentar» y «Volver», las salidas de sin certificados o las de abrir el firmado.

import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { AlertIcon, FileIcon, FolderIcon } from "../design-system/icons";
import type { Certificate } from "./certificate";
import { isUsable } from "./certificate";
import type { Destination } from "./destination";
import { shortenDestination } from "./destination";
import type { SigningFailure } from "./failure";
import type { CertificateState } from "./SigningPanel";

interface PanelFooterDestinationProps {
  destination: Destination;
  documentName: string;
}

interface PanelFooterSigningProps extends PanelFooterDestinationProps {
  signed?: false;
  failure: SigningFailure | null;
  onChangeDestination: () => void;
  signing: boolean;
  blocked: boolean;
  /** Si la firma local lo rechaza por certificado. */
  closed: boolean;
  certificate: CertificateState;
  onRetryCertificates: () => void;
  onChooseModule: () => void;
  onSign: () => void;
  /** Cierra el error y vuelve al panel, con el ciclo a medias olvidado en el backend. */
  onBack: () => void;
}

interface PanelFooterSignedProps extends PanelFooterDestinationProps {
  signed: true;
  /** Abre el PDF firmado con el visor del sistema. */
  onOpenDocument: () => void;
  /** Abre la carpeta donde quedó, con las firmas anteriores dentro. */
  onOpenFolder: () => void;
  /** Vuelve al panel de firma con el original releído del disco. */
  onSign: () => void;
  /** Mueve el destino del documento (ADR-0011); ausente, el pie no ofrece «Cambiar». */
  onChangeDestination?: () => void;
  /** Si el documento se puede firmar en el escritorio, que solo firma PDF. */
  signable?: boolean;
}

type PanelFooterProps = PanelFooterSigningProps | PanelFooterSignedProps;

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
  const { destination, documentName, signed = false } = props;
  const signing = !props.signed && props.signing;
  const fullName = destination.name ?? documentName;
  const shortened = shortenDestination({ folder: destination.folder, name: fullName });
  const writable = signed || destination.writable;

  return (
    <footer className="panel__footer">
      <div className="panel__destination">
        <div className="rf-row panel__destination-label-row">
          <p className="rf-label panel__destination-label">
            {t(signed ? "panel.signed.document" : "panel.footer.savedIn")}
          </p>
          {props.onChangeDestination !== undefined && (
            <button
              type="button"
              className={
                "rf-btn rf-btn--ghost panel__destination-change" +
                (signing ? " panel__controls--dim" : "")
              }
              onClick={props.onChangeDestination}
            >
              {t("actions.change")}
            </button>
          )}
        </div>
        {writable ? (
          <div className="panel__destination-box">
            {destination.folder !== "" && (
              <span
                className="rf-row rf-gap-xs rf-text-muted panel__destination-folder"
                title={destination.folder}
              >
                <FolderIcon size={15} />
                <span className="panel__destination-ellipsis">{shortened.folder}</span>
              </span>
            )}
            <span className="rf-row rf-gap-xs panel__destination-name" title={fullName}>
              <FileIcon size={15} />
              <span className="panel__destination-ellipsis">{shortened.name}</span>
            </span>
          </div>
        ) : (
          <div className="rf-row rf-gap-xs panel__destination-unwritable">
            <AlertIcon size={16} />
            <span className="panel__destination-unwritable-text" title={destination.folder}>
              {unwritableMessage(
                t("panel.footer.unwritable", { folder: shortened.folder }),
                shortened.folder,
              )}
            </span>
          </div>
        )}
      </div>
      {props.signed ? (
        <div className="rf-row rf-gap-xs panel__signed-actions">
          <button
            type="button"
            className="rf-btn rf-btn--primary panel__signed-open"
            onClick={props.onOpenDocument}
          >
            {t(props.signable === false ? "panel.signed.openFile" : "panel.signed.openDocument")}
          </button>
          <button
            type="button"
            title={t("panel.signed.openFolder")}
            className="rf-btn rf-btn--secondary panel__signed-folder"
            onClick={props.onOpenFolder}
          >
            <FolderIcon />
          </button>
          <button
            type="button"
            className="rf-btn rf-btn--ghost panel__signed-sign"
            title={props.signable === false ? t("panel.signed.onlyPdfs") : undefined}
            disabled={props.signable === false}
            onClick={props.onSign}
          >
            {t("actions.sign")}
          </button>
        </div>
      ) : (
        <>
          {props.failure && (
            <div className="rf-row rf-gap-xs panel__failure-actions">
              <button
                type="button"
                className="rf-btn rf-btn--primary panel__failure-retry"
                onClick={props.onSign}
              >
                {t("actions.retry")}
              </button>
              <button
                type="button"
                className="rf-btn rf-btn--ghost panel__failure-back"
                onClick={props.onBack}
              >
                {t("actions.back")}
              </button>
            </div>
          )}
          {!props.failure &&
            (props.certificate.kind === "empty" || props.certificate.kind === "failed") && (
              <div className="rf-row rf-gap-xs panel__certificate-actions">
                <button
                  type="button"
                  className="rf-btn rf-btn--primary panel__add-certificate"
                  onClick={props.onChooseModule}
                >
                  {t("panel.footer.addCertificate")}
                </button>
                <button
                  type="button"
                  className="rf-btn rf-btn--secondary panel__retry"
                  onClick={props.onRetryCertificates}
                >
                  {t("actions.lookAgain")}
                </button>
              </div>
            )}
          {!props.failure &&
            props.certificate.kind !== "empty" &&
            props.certificate.kind !== "failed" && (
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
    <div className="rf-row panel__sign-row">
      <button
        type="button"
        className="rf-btn rf-btn--primary panel__sign"
        title={signButtonTitle(t, closed)}
        disabled={signing || blocked || closed || !usable}
        onClick={onSign}
      >
        {t(signing ? "panel.footer.signing" : "actions.sign")}
      </button>
    </div>
  );
}

function signButtonTitle(t: TFunction, closed: boolean) {
  return closed ? t("panel.previousSignatures.closed") : undefined;
}
