//! Un desplegable de la aplicación, no el `<select>` del sistema, con su teclado y su accesibilidad repuestos a mano.

import { useCallback, useId, useRef, useState } from "react";
import { ChevronDownIcon } from "../design-system/icons";
import { Popover } from "../design-system/Popover";
import "./Select.css";

/** Una opción del desplegable: el valor que se guarda y el texto que se ve. */
interface Option<T extends string> {
  value: T;
  label: string;
}

interface SelectProps<T extends string> {
  /** El rótulo, siempre el nombre accesible del control; visible salvo con `hideLabel`. */
  label: string;
  /** Oculta el rótulo a la vista, para un control que ya vive junto a su etiqueta (una celda de tabla). */
  hideLabel?: boolean;
  value: T;
  options: readonly Option<T>[];
  onChange: (value: T) => void;
  /** Hacia dónde se despliega la lista: arriba cuando debajo no queda sitio. */
  opens?: "down" | "up";
}

/**
 * Un desplegable **de la aplicación**, no el del sistema.
 *
 * Existe porque un `<select>` nativo no se puede vestir: el cierre se estila
 * con CSS, pero la lista que se despliega la pinta el sistema de ventanas
 * —GTK, bajo WebKitGTK— y no la hoja de estilos, así que las opciones salían
 * con los colores del escritorio en medio de un diálogo que va con los tokens
 * del sistema de diseño. No es un gusto ni una limitación que se pueda
 * rodear con más CSS: es que ese trozo de interfaz no es nuestro.
 *
 * A cambio hay que reponer a mano lo que el elemento nativo daba gratis, y es
 * la parte que importa: `combobox` + `listbox` con `aria-activedescendant`,
 * teclado completo (flechas, Inicio, Fin, Intro, Escape), cierre al pulsar
 * fuera y foco de vuelta al cierre. Un `<div>` con un `onClick` no es un
 * desplegable, es un dibujo de uno.
 */
export function Select<T extends string>({
  label,
  hideLabel = false,
  value,
  options,
  onChange,
  opens = "down",
}: SelectProps<T>) {
  const [open, setOpen] = useState(false);
  // Dónde está el cursor del teclado mientras la lista está abierta. No es la
  // selección: moverse por la lista no elige nada hasta que se pulsa Intro.
  const [active, setActive] = useState(0);
  const container = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const labelId = useId();
  const listId = useId();
  const optionId = useId();

  const chosen = options.findIndex((option) => option.value === value);
  const shown = options[chosen === -1 ? 0 : chosen];

  const close = useCallback(() => setOpen(false), []);

  // Al abrir, el cursor arranca en lo que ya está elegido y no en la primera
  // opción: abrir el desplegable no es empezar de cero.
  const show = () => {
    setActive(chosen === -1 ? 0 : chosen);
    setOpen(true);
  };

  const choose = (index: number) => {
    const option = options[index];
    if (option) onChange(option.value);
    close();
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    const last = options.length - 1;
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        setActive((at) => Math.min(at + 1, last));
        return;
      case "ArrowUp":
        event.preventDefault();
        setActive((at) => Math.max(at - 1, 0));
        return;
      case "Home":
        event.preventDefault();
        setActive(0);
        return;
      case "End":
        event.preventDefault();
        setActive(last);
        return;
      case "Enter":
      case " ":
        event.preventDefault();
        choose(active);
        return;
      default:
    }
  };

  return (
    <div className="rf-field select" ref={container}>
      <span className={hideLabel ? "rf-label rf-visually-hidden" : "rf-label"} id={labelId}>
        {label}
      </span>
      <button
        type="button"
        ref={button}
        className="rf-input select__control"
        role="combobox"
        aria-labelledby={labelId}
        aria-expanded={open}
        aria-controls={open ? listId : undefined}
        aria-haspopup="listbox"
        onClick={() => (open ? close() : show())}
        onKeyDown={(event) => {
          if (!open && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
            event.preventDefault();
            show();
          }
        }}
      >
        <span className="select__value">{shown?.label ?? ""}</span>
        <ChevronDownIcon />
      </button>
      <Popover
        open={open}
        onClose={close}
        anchorRef={container}
        initialFocus="panel"
        returnFocusRef={button}
        restoreFocus="always"
        className={`select__list rf-card rf-card--elevated${opens === "up" ? " select__list--up" : ""}`}
        id={listId}
        role="listbox"
        tabIndex={-1}
        aria-labelledby={labelId}
        aria-activedescendant={`${optionId}-${active}`}
        onKeyDown={onKeyDown}
      >
        {options.map((option, index) => (
          <div
            key={option.value}
            id={`${optionId}-${index}`}
            role="option"
            // El foco lo guarda la lista y el cursor lo lleva
            // `aria-activedescendant`, que es el patrón de `combobox` con
            // `listbox`: la opción **no** entra en el orden de tabulación.
            // El `-1` está para que sea enfocable por programa y para que el
            // analizador no la lea como un adorno con un `onClick` encima.
            tabIndex={-1}
            aria-selected={option.value === value}
            className={
              index === active ? "select__option select__option--active" : "select__option"
            }
            // `onPointerDown` y no `onClick`: el oyente que cierra al pulsar
            // fuera también es de `pointerdown`, y con `click` la lista se
            // desmontaría antes de que llegara el clic.
            onPointerDown={(event) => {
              event.preventDefault();
              choose(index);
            }}
            onPointerEnter={() => setActive(index)}
          >
            {option.label}
          </div>
        ))}
      </Popover>
    </div>
  );
}
