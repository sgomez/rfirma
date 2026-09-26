import { useTranslation } from "react-i18next";
import { AlertIcon, FileIcon, FolderIcon } from "../design-system/icons";
import { CertificateFooterButton, LoadingCertificateFooterButton } from "./CertificateFooterButton";
import type { Certificate } from "./certificate";
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
  certificate: CertificateState;
  onChooseCertificate: (certificate: Certificate) => void;
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
  /** Abre la carpeta donde quedó, con las firmas anteriores dentro (ID-81). */
  onOpenFolder: () => void;
  /** Vuelve al panel de firma con el original releído del disco (ID-80). */
  onSignAgain: () => void;
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
 * en» mientras se decide, «Guardado en» una vez escrito, con `Cambiar` oculto
 * sin mover nada (`visibility:hidden`)— y, abajo, la fila de 44 px con la
 * acción del momento: el certificado, «Reintentar»/«Volver», o los dos
 * caminos hasta el fichero firmado y «Volver a firmar».
 */
export function PanelFooter(props: PanelFooterProps) {
  const { t } = useTranslation();
  const { destination, documentName, signed = false } = props;
  const signing = !props.signed && props.signing;
  const fullName = destination.name ?? documentName;
  // El recorte por el medio del nombre y por la cola de la carpeta siguen
  // siendo los de `shortenDestination`; la elipsis de la hoja de estilos es
  // solo el resguardo cuando ni eso basta (design-system.md § Ruta de destino).
  const shortened = shortenDestination({ folder: destination.folder, name: fullName });
  const writable = signed || destination.writable;

  return (
    <footer className="panel__footer">
      <div className="panel__destination">
        <div className="rf-row panel__destination-label-row">
          <p className="rf-label panel__destination-label">
            {t(signed ? "panel.signed.savedIn" : "panel.footer.savedIn")}
          </p>
          <button
            type="button"
            className={
              "rf-btn rf-btn--ghost panel__destination-change" +
              (signed ? " panel__destination-change--hidden" : "") +
              (signing ? " panel__controls--dim" : "")
            }
            onClick={props.signed ? undefined : props.onChangeDestination}
          >
            {t("actions.change")}
          </button>
        </div>
        {writable ? (
          <div className="panel__destination-box">
            <span
              className="rf-row rf-gap-xs rf-text-muted panel__destination-folder"
              title={destination.folder}
            >
              <FolderIcon size={15} />
              <span className="panel__destination-ellipsis">{shortened.folder}</span>
            </span>
            <span className="rf-row rf-gap-xs panel__destination-name" title={fullName}>
              <FileIcon size={15} />
              <span className="panel__destination-ellipsis">{shortened.name}</span>
            </span>
          </div>
        ) : (
          <div className="rf-row rf-gap-xs panel__destination-unwritable">
            <AlertIcon size={16} />
            <span className="panel__destination-unwritable-text">
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
            {t("panel.signed.openDocument")}
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
            className="rf-btn rf-btn--ghost panel__signed-again"
            onClick={props.onSignAgain}
          >
            {t("panel.signed.signAgain")}
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
                {t("panel.footer.retrySigning")}
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
          {!props.failure && props.certificate.kind === "loading" && (
            <LoadingCertificateFooterButton />
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
                  {t("panel.certificate.retry")}
                </button>
              </div>
            )}
          {!props.failure &&
            (props.certificate.kind === "unchosen" || props.certificate.kind === "chosen") && (
              <CertificateFooterButton
                certificates={props.certificate.certificates}
                chosen={props.certificate.kind === "chosen" ? props.certificate.certificate : null}
                onChoose={props.onChooseCertificate}
                onSign={props.onSign}
                signing={props.signing}
                blocked={props.blocked}
              />
            )}
        </>
      )}
    </footer>
  );
}
