//! El interruptor `role="switch"` del sistema de diseño, con su texto al lado o desnudo y nombrado por etiqueta o por referencia.

import { useId } from "react";
import { classNames } from "./classNames";
import "./Switch.css";

type SwitchName =
  | {
      /** El texto del interruptor, a su lado y su nombre accesible. */
      label: string;
      labelledBy?: never;
    }
  | {
      /** El `id` de otro elemento que lo nombra: el interruptor va desnudo, sin texto propio. */
      labelledBy: string;
      label?: never;
    };

type SwitchProps = SwitchName & {
  checked: boolean;
  hint?: string;
  className?: string;
  /**
   * `true` para la separación de Preferencias (16 px entre la pastilla y el
   * texto); por omisión, la del panel de firma (8 px).
   *
   * Es una propiedad y no un valor fijo porque los dos artboards que llevan
   * este mismo interruptor lo separan distinto —`rf-gap-xs` en el panel,
   * `rf-gap-sm` en el diálogo— y un solo número no puede ser los dos.
   */
  wide?: boolean;
  /** El rótulo delante, en `.rf-label`, y el interruptor a su derecha: la fila de «Firma visible». */
  trailing?: boolean;
  /** Bloqueado en su valor actual; `title` dice el motivo. */
  disabled?: boolean;
  title?: string;
  onChange: (checked: boolean) => void;
};

/**
 * Un interruptor.
 *
 * Se maqueta con tokens, en `Switch.css`, junto al componente. El estado
 * bloqueado no lo decide él: `checked` y `disabled` llegan de fuera, y un
 * interruptor puede estar encendido y bloqueado a la vez.
 *
 * Con `labelledBy` va desnudo: solo la pastilla, nombrada por otro elemento
 * (una fila con su propio texto y su miniatura).
 *
 * Es un `role="switch"` de verdad y no una casilla disfrazada, para que el
 * lector de pantalla diga «activado» y no «marcado».
 *
 * El interruptor va **delante** del texto, como lo dibujan «Con rúbrica» y
 * preferencias; con `trailing` va detrás del rótulo. La pastilla es lo que se
 * busca con la vista, y a la izquierda cae siempre en la misma columna
 * aunque el texto de al lado ocupe una línea o tres. La ayuda queda fuera del
 * botón, sangrada hasta el texto: dentro se sumaría al nombre accesible y el
 * lector de pantalla leería el párrafo entero al llegar al interruptor.
 */
export function Switch({
  checked,
  label,
  labelledBy,
  hint,
  className,
  wide = false,
  trailing = false,
  disabled = false,
  title,
  onChange,
}: SwitchProps) {
  const hintId = useId();

  return (
    <div
      className={classNames(
        "switch",
        wide && "switch--wide",
        trailing && "switch--trailing",
        label === undefined && "switch--bare",
        className,
      )}
    >
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-labelledby={labelledBy}
        aria-describedby={hint ? hintId : undefined}
        disabled={disabled}
        title={title}
        className="switch__control"
        onClick={() => onChange(!checked)}
      >
        {trailing && label !== undefined && <span className="rf-label switch__label">{label}</span>}
        <span className="switch__track" aria-hidden="true">
          <span className="switch__knob" />
        </span>
        {!trailing && label !== undefined && <span className="rf-prose">{label}</span>}
      </button>
      {hint && (
        <p className="rf-hint switch__hint" id={hintId}>
          {hint}
        </p>
      )}
    </div>
  );
}
