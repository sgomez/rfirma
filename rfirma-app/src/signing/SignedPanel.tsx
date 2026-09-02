import { useTranslation } from "react-i18next";
import { FileIcon } from "../design-system/icons";
import type { NamedFailure } from "../errors/classify";
import { ErrorNotice } from "../errors/ErrorNotice";
import { formatSize } from "./SigningPanel";
import "./SigningPanel.css";
import "./SignedPanel.css";

/** El documento que quedó escrito, con lo que el panel enseña de él. */
export interface SignedSummary {
  /** El nombre del fichero firmado. La ruta no se enseña nunca (ADR-0011). */
  name: string;
  /** Cuántas páginas tiene, o `null` si no se sabe: no se inventa un número. */
  pages: number | null;
  /**
   * Cuántos bytes ocupa el fichero que ha quedado.
   *
   * Lo cuenta la postfirma al escribirlo y llega hasta aquí sin tocarse
   * (ID-77): la ventana **no lo recalcula**, entre otras cosas porque no
   * conoce la ruta del fichero para poder abrirlo otra vez (ADR-0011).
   */
  sizeBytes: number;
}

interface SignedPanelProps {
  document: SignedSummary;
  /** Abre el PDF firmado con el visor del sistema. */
  onOpenDocument: () => void;
  /** Abre la carpeta donde quedó, con las firmas anteriores dentro (ID-81). */
  onOpenFolder: () => void;
  /**
   * Vuelve al panel de firma **con el original releído del disco**: firmar otra
   * vez el mismo documento (ID-80).
   */
  onSignAgain: () => void;
  /**
   * Por qué no se pudo abrir lo que se pidió, si es que se pidió algo y falló.
   *
   * El portal puede negarse o no haber nadie que abra un PDF, y entonces el
   * usuario se queda mirando un botón que no hizo nada: sin este aviso, el
   * único camino que tiene hasta el fichero fallaría en silencio.
   */
  failure?: NamedFailure | null;
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
 * El encabezado `Resumen` sostiene hoy una sola insignia, `PAdES` —rFirma no
 * produce otro formato—, y **se queda así a propósito**: guarda el sitio de la
 * ficha 14, que traerá la insignia con el número de firmas del documento y la
 * tarjeta de cada una, con `La tuya` en la del usuario. Que parezca vacío no es
 * un descuido; contar las firmas del PDF pide volver a abrirlo, y eso es de
 * v1.0.
 *
 * Los tres botones del pie son las tres salidas del estado, y los dos primeros
 * cargan más peso del que parece: bajo el arenero la aplicación nunca conoce la
 * ruta del documento y el usuario nunca la ve (ADR-0011), así que son la única
 * forma que tiene de llegar al fichero que acaba de firmar (ID-79). **No hay
 * «Firmar otro documento»**: lo hubo y se retira, porque la bandeja siempre
 * ofrece abrir y aceptar arrastre.
 */
export function SignedPanel({
  document,
  onOpenDocument,
  onOpenFolder,
  onSignAgain,
  failure = null,
}: SignedPanelProps) {
  const { t, i18n } = useTranslation();

  return (
    <div className="panel">
      <div className="panel__scroll">
        <div className="panel__header">
          <span className="panel__header-icon">
            <FileIcon />
          </span>
          <div className="panel__header-text">
            <p className="rf-title panel__document">{document.name}</p>
            <p className="rf-body rf-text-muted">
              {[
                document.pages === null
                  ? null
                  : document.pages === 1
                    ? t("panel.document.pages.one")
                    : t("panel.document.pages.many", { pages: document.pages }),
                formatSize(document.sizeBytes, i18n.language),
              ]
                .filter((piece) => piece !== null)
                .join(" · ")}
            </p>
          </div>
        </div>

        {/*
         * El encabezado con una sola insignia debajo **guarda el sitio de la
         * ficha 14**: ahí irán el número de firmas del documento y la tarjeta
         * de cada una. No se quita por parecer vacío.
         */}
        <section className="panel__section" aria-label={t("panel.signed.summary")}>
          <p className="rf-label panel__heading">{t("panel.signed.summary")}</p>
          <div className="rf-row rf-gap-xs">
            <span className="rf-badge">{t("panel.signed.format")}</span>
          </div>
        </section>

        {failure && <ErrorNotice situation={failure.situation} technicalDetail={failure.detail} />}
      </div>

      <footer className="panel__footer">
        <button
          type="button"
          className="rf-btn rf-btn--primary signed-panel__action"
          onClick={onOpenDocument}
        >
          {t("panel.signed.openDocument")}
        </button>
        <button
          type="button"
          className="rf-btn rf-btn--secondary signed-panel__action"
          onClick={onOpenFolder}
        >
          {t("panel.signed.openFolder")}
        </button>
        <button
          type="button"
          className="rf-btn rf-btn--ghost signed-panel__action"
          onClick={onSignAgain}
        >
          {t("panel.signed.signAgain")}
        </button>
      </footer>
    </div>
  );
}
