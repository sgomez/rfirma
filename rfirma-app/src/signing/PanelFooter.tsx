import { useTranslation } from "react-i18next";
import { FolderIcon } from "../design-system/icons";
import { ErrorNotice } from "../errors/ErrorNotice";
import type { Destination } from "./destination";
import { shortenDestination } from "./destination";
import type { SigningFailure } from "./failure";

interface PanelFooterProps {
  failure: SigningFailure | null;
  destination: Destination;
  documentName: string;
  onChangeDestination: () => void;
  /** Con el interruptor encendido y sin colocar no se firma (ID-93). */
  unplaced: boolean;
  usable: boolean;
  signing: boolean;
  blocked: boolean;
  onSign: () => void;
  onOpenHelp?: () => void;
}

/** El pie del panel: destino o fallo, y el botón que firma. */
export function PanelFooter({
  failure,
  destination,
  documentName,
  onChangeDestination,
  unplaced,
  usable,
  signing,
  blocked,
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
      <button
        type="button"
        className="rf-btn rf-btn--primary panel__sign"
        disabled={!usable || signing || blocked}
        onClick={onSign}
      >
        {failure ? t("panel.footer.retry") : t("actions.sign")}
      </button>
    </footer>
  );
}
