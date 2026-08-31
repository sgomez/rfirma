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
}

/**
 * La única ventana de rFirma: una cabecera y tres regiones fijas debajo
 * —bandeja, visor y panel de firma—.
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
}: MainWindowProps) {
  const { t } = useTranslation();

  return (
    <div className="main-window">
      <Header
        status={status}
        menuAnchor={menuAnchor}
        onOpenPreferences={onOpenPreferences}
        onOpenAbout={onOpenAbout}
      />
      <div className="main-window__body">
        <section className="main-window__tray" aria-label={t("window.tray")} />
        <section className="main-window__viewer" aria-label={t("window.viewer")} />
        <section className="main-window__panel" aria-label={t("window.panel")} />
      </div>
    </div>
  );
}
