import { useCallback, useEffect, useId, useRef, useState } from "react";
import { ChevronDownIcon } from "../design-system/icons";
import "./Select.css";

/** Una opción del desplegable: el valor que se guarda y el texto que se ve. */
export interface Option<T extends string> {
  value: T;
  label: string;
}

interface SelectProps<T extends string> {
  /** El rótulo visible, que además es el nombre accesible del control. */
  label: string;
  value: T;
  options: readonly Option<T>[];
  onChange: (value: T) => void;
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
export function Select<T extends string>({ label, value, options, onChange }: SelectProps<T>) {
  const [open, setOpen] = useState(false);
  // Dónde está el cursor del teclado mientras la lista está abierta. No es la
  // selección: moverse por la lista no elige nada hasta que se pulsa Intro.
  const [active, setActive] = useState(0);
  const container = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const labelId = useId();
  const listId = useId();
  const optionId = useId();

  const chosen = options.findIndex((option) => option.value === value);
  const shown = options[chosen === -1 ? 0 : chosen];

  const close = useCallback((giveBackFocus: boolean) => {
    setOpen(false);
    if (giveBackFocus) button.current?.focus();
  }, []);

  // Al abrir, el cursor arranca en lo que ya está elegido y no en la primera
  // opción: abrir el desplegable no es empezar de cero.
  const show = () => {
    setActive(chosen === -1 ? 0 : chosen);
    setOpen(true);
  };

  // El foco se va a la lista para que el lector de pantalla la anuncie y para
  // que las flechas no muevan la página de debajo.
  useEffect(() => {
    if (open) list.current?.focus();
  }, [open]);

  // Pulsar fuera cierra, igual que el menú de la cabecera. Sin esto la lista
  // se queda flotando sobre el diálogo mientras se toca otra cosa.
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (!container.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  const choose = (index: number) => {
    const option = options[index];
    if (option) onChange(option.value);
    close(true);
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
      case "Escape":
        event.preventDefault();
        close(true);
        return;
      case "Tab":
        // Tabular sale del control, así que la lista se va con él, pero el
        // foco sigue su camino: devolverlo al botón lo dejaría atrapado.
        close(false);
        return;
      default:
    }
  };

  return (
    <div className="rf-field select" ref={container}>
      <span className="rf-label" id={labelId}>
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
        onClick={() => (open ? close(false) : show())}
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
      {open && (
        <div
          className="select__list rf-card rf-card--elevated"
          ref={list}
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
        </div>
      )}
    </div>
  );
}
