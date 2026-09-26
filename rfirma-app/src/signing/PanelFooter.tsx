import { useTranslation } from "react-i18next";
import { FolderIcon } from "../design-system/icons";
import { CertificateFooterButton } from "./CertificateFooterButton";
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
  /** Con el interruptor encendido y sin colocar no se firma (ID-93). */
  unplaced: boolean;
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
  // El destino recortado. Sin nombre compuesto —la carpeta no se deja
  // comprobar— se enseña el del documento, que es lo único que se sabe.
  const shortened = shortenDestination({
    folder: destination.folder,
    name: destination.name ?? documentName,
  });
  // Una vez firmado, el rótulo ya no es una promesa que pueda incumplirse: es
  // lo que ha quedado escrito, y se enseña siempre con su caja (ID-63).
  const showBox = signed || destination.writable;

  return (
    <footer className="panel__footer">
      <div className="panel__destination">
        {showBox && (
          <p className="rf-label">{t(signed ? "panel.signed.savedIn" : "panel.footer.savedIn")}</p>
        )}
        <div className="rf-row rf-gap-xs panel__destination-row">
          <span className="panel__destination-icon">
            <FolderIcon />
          </span>
          {/* El destino son **dos cosas**: la carpeta, atenuada y precedida
              de `…/` —hay carpetas por encima y no se afirma cuáles—, y el
              nombre sin atenuar, que es el dato (ID-63). El aviso de que no
              se puede escribir **no se recorta**: perderlo por elipsis sería
              perderlo cuando más falta hace. */}
          {showBox ? (
            <p className="rf-prose panel__destination-path">
              <span className="rf-text-muted">{`…/${shortened.folder}/`}</span>
              {shortened.name}
            </p>
          ) : (
            <p className="rf-prose panel__destination-unwritable">
              {t("panel.footer.unwritable", { folder: shortened.folder })}
            </p>
          )}
          <button
            type="button"
            className={
              signed
                ? "rf-btn rf-btn--ghost panel__destination-change panel__destination-change--hidden"
                : "rf-btn rf-btn--ghost panel__destination-change"
            }
            onClick={props.signed ? undefined : props.onChangeDestination}
          >
            {t("actions.change")}
          </button>
        </div>
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
          {props.unplaced && !props.failure && (
            <p className="rf-hint panel__place-first">{t("panel.footer.placeFirst")}</p>
          )}
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
            <button type="button" className="rf-btn rf-btn--primary panel__sign" disabled>
              {t("panel.certificate.loading")}
            </button>
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
