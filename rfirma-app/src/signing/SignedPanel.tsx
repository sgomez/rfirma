import { useTranslation } from "react-i18next";
import { FileIcon } from "../design-system/icons";
import "./SigningPanel.css";
import "./SignedPanel.css";

/** El documento que quedó escrito, con lo que el panel enseña de él. */
export interface SignedSummary {
  /** El nombre del fichero firmado. La ruta no se enseña nunca (ADR-0011). */
  name: string;
  /** Cuántas páginas tiene, o `null` si no se sabe: no se inventa un número. */
  pages: number | null;
}

interface SignedPanelProps {
  document: SignedSummary;
  /** Volver al panel de firma con el documento abierto: empezar otra vez. */
  onSignAnother: () => void;
}

/**
 * La columna derecha cuando la firma ya está escrita
 * (docs/design/panel-de-firma.md, estado «Firmado»).
 *
 * Sin este panel el recorrido terminaba en silencio: la postfirma devolvía un
 * [`SignedDocument`] que nadie leía, el diálogo de progreso se cerraba y la
 * ventana volvía al panel con el nombre del fichero **original**. Quien firma
 * no recibía ninguna confirmación de que se hubiera escrito nada.
 *
 * Enseña solo lo que se sabe sin volver a abrir el PDF resultante: el fichero
 * que quedó, el formato —rFirma solo produce PAdES— y la salida para firmar
 * otro documento. La insignia con el número de firmas, las tarjetas de cada
 * firma y los dos botones de «Abrir» necesitan datos y capacidades que hoy no
 * existen, y por la regla del ID-44 —lo desconocido no ocupa sitio— **no se
 * montan** en lugar de aparecer vacíos. Están anotados como pendientes en la
 * ficha.
 */
export function SignedPanel({ document, onSignAnother }: SignedPanelProps) {
  const { t } = useTranslation();

  return (
    <div className="panel">
      <div className="panel__scroll">
        <div className="panel__header">
          <span className="panel__header-icon">
            <FileIcon />
          </span>
          <div className="panel__header-text">
            <p className="rf-title panel__document">{document.name}</p>
            {document.pages !== null && (
              <p className="rf-body rf-text-muted">
                {document.pages === 1
                  ? t("panel.document.pages.one")
                  : t("panel.document.pages.many", { pages: document.pages })}
              </p>
            )}
          </div>
        </div>

        <section className="panel__section" aria-label={t("panel.signed.summary")}>
          <p className="rf-label panel__heading">{t("panel.signed.summary")}</p>
          <div className="rf-row rf-gap-xs">
            <span className="rf-badge">{t("panel.signed.format")}</span>
          </div>
        </section>
      </div>

      <footer className="panel__footer">
        <button
          type="button"
          className="rf-btn rf-btn--ghost signed-panel__again"
          onClick={onSignAnother}
        >
          {t("panel.signed.signAnother")}
        </button>
      </footer>
    </div>
  );
}
