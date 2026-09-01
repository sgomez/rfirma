import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { Badge } from "../documents/document";
import { Header } from "./Header";
import "./MainWindow.css";
import type { MenuAnchor } from "./menuAnchor";

interface MainWindowProps {
  /** La insignia del documento abierto, o `null` si no hay ninguno. */
  status: Badge | null;
  /** Dónde va el menú de dos entradas. Ver [`MenuAnchor`]. */
  menuAnchor: MenuAnchor;
  onOpenPreferences: () => void;
  onOpenAbout: () => void;
  /** El contenido de la bandeja, que es quien sabe de documentos. */
  tray: ReactNode;
  /** El contenido del visor, que es quien sabe de páginas y de recuadros. */
  viewer: ReactNode;
  /**
   * El contenido del panel, que es quien sabe de certificados y de firma, o
   * `null` cuando no hay documento abierto: entonces el panel **no se monta**
   * y la ventana se ve en dos columnas (ID-51).
   */
  panel: ReactNode;
}

/**
 * La única ventana de rFirma: una cabecera y, debajo, la bandeja, el visor y
 * —en cuanto hay documento— el panel de firma.
 *
 * **Sin documento la ventana es de dos columnas.** El panel no se oculta con
 * `display: none`: no se monta (ID-51), que es lo que ya hacía la composición
 * al pasar `null` y lo que dice el estado 1 de la tabla de la ficha.
 *
 * **No hay navegación.** El recorrido entero, de abrir el documento a
 * guardarlo firmado, ocurre aquí sin cambiar de pantalla (ID-25), así que no
 * hay router y no debe aparecer uno: las diez situaciones de la ficha son
 * combinaciones del contenido de las tres regiones, no pantallas distintas.
 *
 * Este componente es **solo la disposición**: no conoce documentos ni
 * certificados. Quién llena cada región es cosa de su propio sub-issue.
 */
export function MainWindow({
  status,
  menuAnchor,
  onOpenPreferences,
  onOpenAbout,
  tray,
  viewer,
  panel,
}: MainWindowProps) {
  const { t } = useTranslation();

  // Sin documento no hay panel que montar, y sin panel la ventana es de dos
  // columnas. `null` y `undefined` son lo que la composición pasa; una cadena
  // vacía o un `false` no llegan aquí.
  const hasPanel = panel !== null && panel !== undefined;

  return (
    <div className="main-window">
      <Header
        status={status}
        menuAnchor={menuAnchor}
        onOpenPreferences={onOpenPreferences}
        onOpenAbout={onOpenAbout}
      />
      <div
        className={hasPanel ? "main-window__body" : "main-window__body main-window__body--no-panel"}
      >
        <section className="main-window__tray" aria-label={t("window.tray")}>
          {tray}
        </section>
        <section className="main-window__viewer" aria-label={t("window.viewer")}>
          {viewer}
        </section>
        {hasPanel && (
          <section className="main-window__panel" aria-label={t("window.panel")}>
            {panel}
          </section>
        )}
      </div>
    </div>
  );
}
