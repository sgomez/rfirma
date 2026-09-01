import { useId } from "react";
import "./Switch.css";

interface SwitchProps {
  checked: boolean;
  label: string;
  hint?: string;
  /**
   * `true` para la separación de Preferencias (16 px entre la pastilla y el
   * texto); por omisión, la del panel de firma (8 px).
   *
   * Es una propiedad y no un valor fijo porque los dos artboards que llevan
   * este mismo interruptor lo separan distinto —`rf-gap-xs` en el panel,
   * `rf-gap-sm` en el diálogo— y un solo número no puede ser los dos.
   */
  wide?: boolean;
  onChange: (checked: boolean) => void;
}

/**
 * Un interruptor.
 *
 * Se maqueta con tokens y no sale del sistema de diseño, que a propósito no lo
 * tiene: el vocabulario de `rf-*` está cerrado, y lo que cada pantalla necesita
 * de más se escribe con `var(--rf-*)` en su propio CSS.
 *
 * Es un `role="switch"` de verdad y no una casilla disfrazada, para que el
 * lector de pantalla diga «activado» y no «marcado».
 *
 * El interruptor va **delante** del texto, que es como lo dibujan los dos
 * artboards que lo llevan —el panel de firma y preferencias—: la pastilla es lo
 * que se busca con la vista, y a la izquierda cae siempre en la misma columna
 * aunque el texto de al lado ocupe una línea o tres. La ayuda queda fuera del
 * botón, sangrada hasta el texto: dentro se sumaría al nombre accesible y el
 * lector de pantalla leería el párrafo entero al llegar al interruptor.
 */
export function Switch({ checked, label, hint, wide = false, onChange }: SwitchProps) {
  const hintId = useId();

  return (
    <div className={wide ? "switch switch--wide" : "switch"}>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-describedby={hint ? hintId : undefined}
        className="switch__control"
        onClick={() => onChange(!checked)}
      >
        <span className="switch__track" aria-hidden="true">
          <span className="switch__knob" />
        </span>
        <span className="rf-prose">{label}</span>
      </button>
      {hint && (
        <p className="rf-hint switch__hint" id={hintId}>
          {hint}
        </p>
      )}
    </div>
  );
}
