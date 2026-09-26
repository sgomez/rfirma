import { useTranslation } from "react-i18next";
import { FolderIcon } from "../design-system/icons";
import { ErrorNotice } from "../errors/ErrorNotice";
import { CertificateFooterButton } from "./CertificateFooterButton";
import type { Certificate } from "./certificate";
import type { Destination } from "./destination";
import { shortenDestination } from "./destination";
import type { SigningFailure } from "./failure";
import type { CertificateState } from "./SigningPanel";

interface PanelFooterProps {
  failure: SigningFailure | null;
  destination: Destination;
  documentName: string;
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
  onOpenHelp?: () => void;
}

/**
 * El pie del panel: 162 px en todos los estados
 * (docs/design/panel-de-firma.md § Pie fijo). «Guardar en» arriba y, abajo,
 * la fila de 44 px con el certificado y la acción del momento —buscar,
 * añadir uno, o el botón partido que firma—.
 */
export function PanelFooter({
  failure,
  destination,
  documentName,
  onChangeDestination,
  unplaced,
  signing,
  blocked,
  certificate,
  onChooseCertificate,
  onRetryCertificates,
  onChooseModule,
  onSign,
  onOpenHelp,
}: PanelFooterProps) {
  const { t } = useTranslation();
  // El destino recortado. Sin nombre compuesto —la carpeta no se deja
  // comprobar— se enseña el del documento, que es lo único que se sabe.
  const shortened = shortenDestination({
    folder: destination.folder,
    name: destination.name ?? documentName,
  });

  return (
    <footer className="panel__footer">
      {failure ? (
        <ErrorNotice
          situation={failure.situation}
          technicalDetail={failure.detail}
          onOpenHelp={onOpenHelp}
        />
      ) : (
        <div className="panel__destination">
          {/* El rótulo es una promesa, así que **desaparece** cuando no se
              puede cumplir: con la carpeta no escribible el pie dice solo que
              no se puede escribir en ella, y no las dos cosas a la vez. */}
          {destination.writable && <p className="rf-label">{t("panel.footer.savedIn")}</p>}
          <div className="rf-row rf-gap-xs panel__destination-row">
            <span className="panel__destination-icon">
              <FolderIcon />
            </span>
            {/* El destino son **dos cosas**: la carpeta, atenuada y precedida
                de `…/` —hay carpetas por encima y no se afirma cuáles—, y el
                nombre sin atenuar, que es el dato (ID-63). El aviso de que no
                se puede escribir **no se recorta**: perderlo por elipsis sería
                perderlo cuando más falta hace. */}
            {destination.writable ? (
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
              className="rf-btn rf-btn--ghost panel__destination-change"
              onClick={onChangeDestination}
            >
              {t("actions.change")}
            </button>
          </div>
        </div>
      )}
      {unplaced && <p className="rf-hint panel__place-first">{t("panel.footer.placeFirst")}</p>}
      {certificate.kind === "loading" && (
        <button type="button" className="rf-btn rf-btn--primary panel__sign" disabled>
          {t("panel.certificate.loading")}
        </button>
      )}
      {(certificate.kind === "empty" || certificate.kind === "failed") && (
        <div className="rf-row rf-gap-xs panel__certificate-actions">
          <button
            type="button"
            className="rf-btn rf-btn--primary panel__add-certificate"
            onClick={onChooseModule}
          >
            {t("panel.footer.addCertificate")}
          </button>
          <button
            type="button"
            className="rf-btn rf-btn--secondary panel__retry"
            onClick={onRetryCertificates}
          >
            {t("panel.certificate.retry")}
          </button>
        </div>
      )}
      {(certificate.kind === "unchosen" || certificate.kind === "chosen") && (
        <CertificateFooterButton
          certificates={certificate.certificates}
          chosen={certificate.kind === "chosen" ? certificate.certificate : null}
          onChoose={onChooseCertificate}
          onSign={onSign}
          signing={signing}
          blocked={blocked}
          signLabel={failure ? t("panel.footer.retry") : undefined}
        />
      )}
    </footer>
  );
}
