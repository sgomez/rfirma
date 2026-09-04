import { InfoIcon } from "../design-system/icons";
import "./NotificationStrip.css";

interface NotificationStripProps {
  /**
   * La frase. **Una sola**: la franja mide 41 px y admite una frase, no un
   * párrafo (ID-207).
   */
  message: string;
  /**
   * La **única** acción secundaria de la franja, o nada si la notificación no
   * lleva ninguna. Una y no dos: el botón primario de la ventana vive al pie
   * del panel de firma y aquí no se compite con él.
   */
  action?: { label: string; onSelect: () => void };
  /** Rótulo accesible de la `×` que la descarta. */
  dismissLabel: string;
  onDismiss: () => void;
}

/**
 * La franja bajo la cabecera: **el patrón de notificación de la ventana**
 * (ID-207), no el widget del aviso de versión.
 *
 * Nada de esto es modal (ID-181). Un modal al arrancar interrumpe el recorrido
 * para decir algo que no lo bloquea; la franja se ve sin abrir nada, se
 * descarta y desaparece del todo. Se descartaron una insignia en el botón de
 * menú —no se ve hasta abrir el menú, así que no notifica— y una línea en el
 * pie —rFirma no tiene barra de estado—.
 *
 * Cuesta 41 px de una ventana cuyo mínimo son 560 y es la única cosa que
 * empuja el documento: por eso lleva icono, **una** frase, **una** acción y la
 * `×`, y nada más.
 *
 * Este componente no conoce a su inquilino: recibe textos ya traducidos, así
 * que el segundo aviso que haga falta no tiene que tocarlo. Quien decide si
 * hay algo que notificar es la composición, y cuando no lo hay la franja **no
 * se monta** y las regiones suben.
 *
 * Lo que la franja **no** es: un sitio para errores del recorrido. El error de
 * firma se queda en el pie del panel y los de Preferencias, en su sección.
 */
export function NotificationStrip({
  message,
  action,
  dismissLabel,
  onDismiss,
}: NotificationStripProps) {
  return (
    <div className="notification-strip" role="status">
      <span className="notification-strip__icon" aria-hidden="true">
        <InfoIcon size={18} />
      </span>
      <p className="rf-body notification-strip__message">{message}</p>
      {action !== undefined && (
        <button
          type="button"
          className="rf-btn rf-btn--ghost notification-strip__action"
          onClick={action.onSelect}
        >
          {action.label}
        </button>
      )}
      <button
        type="button"
        className="rf-btn rf-btn--ghost notification-strip__dismiss"
        aria-label={dismissLabel}
        onClick={onDismiss}
      >
        <span aria-hidden="true">×</span>
      </button>
    </div>
  );
}
