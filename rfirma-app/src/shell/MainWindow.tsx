import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { Badge } from "../documents/document";
import { Header } from "./Header";
import "./MainWindow.css";
import type { MenuAnchor } from "./menuAnchor";

interface MainWindowProps {
  /** La insignia del documento abierto, o `null` si no hay ninguno. */
  status: Badge | null;
  /** Dónde va el menú. Ver [`MenuAnchor`]. */
  menuAnchor: MenuAnchor;
  /** Si «Estado de rFirma» lleva el triángulo de aviso. Ver [`Header`]. */
  hasAttention?: boolean;
  onOpenStatus?: () => void;
  onOpenPreferences: () => void;
  onOpenHelp?: () => void;
  onOpenAbout: () => void;
  /**
   * La franja de notificación, o `null` —lo normal— cuando no hay nada que
   * notificar: entonces **no se monta** y las regiones suben. Es un
   * hueco, no un aviso concreto: quien decide qué se cuenta es la
   * composición. Ver [`NotificationStrip`].
   */
  notification?: ReactNode;
  /** La vista del cuerpo que sustituye a las regiones, o `null` si no hay ninguna. */
  view?: ReactNode;
  /** La tira de pestañas de los documentos abiertos, bajo la cabecera. */
  tabs: ReactNode;
  /** El contenido del visor, que es quien sabe de páginas y de recuadros. */
  viewer: ReactNode;
  /**
   * El contenido del panel, que es quien sabe de certificados y de firma, o
   * `null` cuando no hay documento abierto: entonces el panel **no se monta**
   * y la ventana se ve en una columna (ID-51).
   */
  panel: ReactNode;
}

/**
 * La única ventana de rFirma: una cabecera, la tira de pestañas y, debajo, el
 * visor y —en cuanto hay documento— el panel de firma.
 *
 * **Sin documento la ventana es de una columna.** El panel no se oculta con
 * `display: none`: no se monta (ID-51), que es lo que ya hacía la composición
 * al pasar `null` y lo que dice el estado 1 de la tabla de la ficha.
 *
 * **No hay navegación.** El recorrido entero, de abrir el documento a
 * guardarlo firmado, ocurre aquí sin cambiar de pantalla (ID-25), así que no
 * hay router y no debe aparecer uno: las diez situaciones de la ficha son
 * combinaciones del contenido de las regiones, no pantallas distintas.
 *
 * **Entre la cabecera y las regiones hay sitio para una franja** de
 * notificación (ID-207). No está casi nunca: cuando no hay nada que notificar
 * no se monta, y la ventana es exactamente la de antes. Lo que se cuenta ahí
 * no lo sabe la ventana, que solo le presta el hueco.
 *
 * Este componente es **solo la disposición**: no conoce documentos ni
 * certificados. Quién llena cada región es cosa de su propio sub-issue.
 */
export function MainWindow({
  status,
  menuAnchor,
  hasAttention = false,
  onOpenStatus = () => {},
  onOpenPreferences,
  onOpenHelp = () => {},
  onOpenAbout,
  notification = null,
  view = null,
  tabs,
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
        hasAttention={hasAttention}
        onOpenStatus={onOpenStatus}
        onOpenPreferences={onOpenPreferences}
        onOpenHelp={onOpenHelp}
        onOpenAbout={onOpenAbout}
      />
      {view === null || view === undefined ? tabs : null}
      {notification}
      {view !== null && view !== undefined ? (
        view
      ) : (
        <div
          className={
            hasPanel ? "main-window__body" : "main-window__body main-window__body--no-panel"
          }
        >
          <section className="main-window__viewer" aria-label={t("window.viewer")}>
            {viewer}
          </section>
          {hasPanel && (
            <section className="main-window__panel" aria-label={t("window.panel")}>
              {panel}
            </section>
          )}
        </div>
      )}
    </div>
  );
}
