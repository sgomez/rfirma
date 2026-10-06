//! La lista desplegable con buscador sobre `Popover`: caja cerrada, buscador al abrir, filtrado sin diacríticos, grupos, opciones deshabilitadas y teclado completo; cada opción la pinta quien la usa.

import { type ReactNode, useCallback, useEffect, useId, useRef, useState } from "react";
import { classNames } from "./classNames";
import { CheckIcon, ChevronDownIcon, SearchIcon } from "./icons";
import { Popover } from "./Popover";
import { Stack } from "./Stack";
import "./Combobox.css";

/** Una opción: su clave, lo que pinta el llamador, las palabras por las que se encuentra y, si acaso, su grupo. */
export interface ComboboxOption<T> {
  id: string;
  item: T;
  keywords: readonly string[];
  group?: string;
  disabled?: boolean;
  title?: string;
}

interface ComboboxProps<T> {
  label: string;
  options: readonly ComboboxOption<T>[];
  /** La clave de la opción elegida, o `null` mientras no hay ninguna. */
  value: string | null;
  onChange: (item: T) => void;
  renderOption: (item: T) => ReactNode;
  /** Lo que enseña la caja cerrada. */
  children: ReactNode;
  searchPlaceholder: string;
  emptyMessage: string;
  countLabel: (shown: number, total: number) => string;
  /** El alto máximo de la lista abierta, en px; la ventana lo recorta si no cabe. */
  listMaxHeight?: number;
  /** Pinta las cabeceras con más de una opción aunque todas sean de un mismo grupo. */
  alwaysGroupHeaders?: boolean;
  disabled?: boolean;
  defaultOpen?: boolean;
}

interface Group<T> {
  label: string | undefined;
  options: ComboboxOption<T>[];
}

/** Una lista desplegable con buscador: la caja cerrada que al abrirse es un buscador sobre un `listbox`. */
export function Combobox<T>({
  label,
  options,
  value,
  onChange,
  renderOption,
  children,
  searchPlaceholder,
  emptyMessage,
  countLabel,
  listMaxHeight = 480,
  alwaysGroupHeaders = false,
  disabled = false,
  defaultOpen = false,
}: ComboboxProps<T>) {
  const [open, setOpen] = useState(defaultOpen);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(() => Math.max(indexOf(options, value), 0));
  const frame = useRef<HTMLDivElement>(null);
  const box = useRef<HTMLButtonElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const labelId = useId();
  const listId = useId();
  const optionId = useId();

  const wanted = fold(query.trim());
  const shown = options.filter((option) => matches(option, wanted));
  const groups = groupsOf(shown);
  const withHeaders =
    groupsOf(options).length > 1 ||
    (alwaysGroupHeaders && options.length > 1 && options.some((option) => option.group));
  const ordered = groups.flatMap((group) => group.options);
  const last = ordered.length - 1;

  const close = useCallback(() => {
    setOpen(false);
    setQuery("");
  }, []);

  const show = () => {
    setActive(Math.max(indexOf(options, value), 0));
    setOpen(true);
  };

  useEffect(() => {
    if (!open) return;
    document.getElementById(`${optionId}-${active}`)?.scrollIntoView?.({ block: "nearest" });
  }, [open, active, optionId]);

  const choose = (index: number) => {
    const option = ordered[index];
    if (option === undefined || option.disabled) return;
    onChange(option.item);
    close();
  };

  const onSearchKeyDown = (event: React.KeyboardEvent) => {
    const next = cursorAfter(event.key, active, last);
    if (next !== null) {
      event.preventDefault();
      setActive(next);
    } else if (event.key === "Enter") {
      event.preventDefault();
      choose(active);
    }
  };

  const renderOne = (option: ComboboxOption<T>, index: number) => {
    const selected = option.id === value;
    return (
      <div
        key={option.id}
        id={`${optionId}-${index}`}
        role="option"
        tabIndex={-1}
        aria-selected={selected}
        aria-disabled={option.disabled === true}
        title={option.title}
        className={classNames(
          "combobox__option",
          index === active && "combobox__option--active",
          selected && "combobox__option--chosen",
          option.disabled === true && "combobox__option--disabled",
        )}
        // `onPointerDown` y no `onClick`: el oyente que cierra al pulsar fuera también es de `pointerdown`.
        onPointerDown={(event) => {
          event.preventDefault();
          choose(index);
        }}
        onPointerEnter={() => setActive(index)}
      >
        {renderOption(option.item)}
        {selected && (
          <span className="combobox__check">
            <CheckIcon size={16} strokeWidth={2} />
          </span>
        )}
      </div>
    );
  };

  const renderGroups = () => {
    let offset = 0;
    return groups.map((group) => {
      const start = offset;
      offset += group.options.length;
      const rendered = group.options.map((option, index) => renderOne(option, start + index));
      if (!withHeaders) return rendered;
      return (
        // biome-ignore lint/a11y/useSemanticElements: dentro de un `listbox` el grupo de opciones es `role="group"`; un `<fieldset>` no.
        <div role="group" aria-label={group.label} className="combobox__group" key={group.label}>
          <span className="rf-label combobox__group-label" aria-hidden="true">
            {group.label}
          </span>
          {rendered}
        </div>
      );
    });
  };

  return (
    <Stack className="combobox">
      <span className="rf-label" id={labelId}>
        {label}
      </span>
      <div className="combobox__frame" ref={frame}>
        {open ? (
          <div className="combobox__search">
            <span className="combobox__icon">
              <SearchIcon />
            </span>
            <input
              ref={search}
              type="text"
              role="combobox"
              aria-labelledby={labelId}
              aria-expanded="true"
              aria-controls={listId}
              aria-autocomplete="list"
              aria-activedescendant={ordered.length > 0 ? `${optionId}-${active}` : undefined}
              placeholder={searchPlaceholder}
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setActive(0);
              }}
              onKeyDown={onSearchKeyDown}
            />
          </div>
        ) : (
          <button
            ref={box}
            type="button"
            className="combobox__box"
            role="combobox"
            aria-labelledby={labelId}
            aria-expanded="false"
            aria-haspopup="listbox"
            disabled={disabled}
            onClick={show}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown" || event.key === "ArrowUp") {
                event.preventDefault();
                show();
              }
            }}
          >
            <span className="combobox__value">{children}</span>
            <span className="combobox__icon">
              <ChevronDownIcon strokeWidth={1.8} />
            </span>
          </button>
        )}
      </div>
      <Popover
        open={open}
        onClose={close}
        anchorRef={frame}
        initialFocus={search}
        returnFocusRef={box}
        restoreFocus="always"
        portal={{ maxHeight: listMaxHeight }}
        className="combobox__layer"
      >
        {wanted !== "" && ordered.length > 0 && (
          <span className="rf-body rf-text-muted combobox__count">
            {countLabel(ordered.length, options.length)}
          </span>
        )}
        {ordered.length === 0 && (
          <span className="rf-body rf-text-muted combobox__empty">{emptyMessage}</span>
        )}
        <div className="combobox__list" id={listId} role="listbox" aria-labelledby={labelId}>
          {renderGroups()}
        </div>
      </Popover>
    </Stack>
  );
}

function indexOf<T>(options: readonly ComboboxOption<T>[], value: string | null): number {
  return groupsOf(options)
    .flatMap((group) => group.options)
    .findIndex((option) => option.id === value);
}

function groupsOf<T>(options: readonly ComboboxOption<T>[]): Group<T>[] {
  const groups: Group<T>[] = [];
  for (const option of options) {
    const group = groups.find((one) => one.label === option.group);
    if (group) group.options.push(option);
    else groups.push({ label: option.group, options: [option] });
  }
  return groups;
}

function cursorAfter(key: string, at: number, last: number): number | null {
  switch (key) {
    case "ArrowDown":
      return Math.max(Math.min(at + 1, last), 0);
    case "ArrowUp":
      return Math.max(at - 1, 0);
    case "Home":
      return 0;
    case "End":
      return Math.max(last, 0);
    default:
      return null;
  }
}

function matches<T>(option: ComboboxOption<T>, wanted: string): boolean {
  return wanted === "" || option.keywords.some((keyword) => fold(keyword).includes(wanted));
}

function fold(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}
