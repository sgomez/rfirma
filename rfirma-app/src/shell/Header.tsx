import { useCallback, useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Badge } from "../documents/document";
import "./Header.css";
import type { MenuAnchor } from "./menuAnchor";

interface HeaderProps {
  /**
   * La insignia del documento abierto, o `null` cuando no hay ninguno. Dos
   * valores y solo dos: la cabecera no conoce `Unavailable`, que describe una
   * fila de la bandeja y no el documento que se está firmando.
   */
  status: Badge | null;
  /** Dónde va el menú de dos entradas. Ver [`MenuAnchor`]. */
  menuAnchor: MenuAnchor;
  onOpenPreferences: () => void;
  onOpenAbout: () => void;
}

/**
 * La franja superior de la ventana: identidad, estado del documento y el
 * **único** menú de la aplicación.
 *
 * No hay barra de menús: el ADR-0007 la retiró, y por eso aquí no hay
 * `role="menubar"` ni entradas de *Archivo* o *Ver*. Abrir un documento tiene
 * la zona de soltar de la bandeja y guardar tiene la fila «Se guardará en» del
 * panel; repetirlos en un menú sería un segundo camino para lo mismo.
 *
 * En macOS las dos entradas se registran en el menú de aplicación nativo, así
 * que el botón ☰ **se oculta** en vez de quedarse vacío.
 */
export function Header({ status, menuAnchor, onOpenPreferences, onOpenAbout }: HeaderProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const menuId = useId();
  const container = useRef<HTMLDivElement>(null);

  const close = useCallback(() => setOpen(false), []);

  // Un menú desplegado se cierra al pulsar fuera y con Escape. Sin esto queda
  // flotando sobre la ventana mientras se trabaja debajo.
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (!container.current?.contains(event.target as Node)) close();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open, close]);

  const choose = (action: () => void) => () => {
    close();
    action();
  };

  return (
    <header className="header rf-surface">
      <p className="rf-title">{t("app.name")}</p>
      <div className="rf-row">
        {status !== null && (
          <span className={status === "Signed" ? "rf-badge rf-badge--primary" : "rf-badge"}>
            {t(status === "Signed" ? "badges.signed" : "badges.unsigned")}
          </span>
        )}
        {menuAnchor === "header" && (
          <div className="header__menu" ref={container}>
            <button
              type="button"
              className={open ? "rf-btn header__button header__button--open" : "rf-btn"}
              aria-label={t("header.menu")}
              aria-haspopup="menu"
              aria-expanded={open}
              aria-controls={open ? menuId : undefined}
              onClick={() => setOpen((wasOpen) => !wasOpen)}
            >
              ☰
            </button>
            {open && (
              <div className="header__popup rf-card rf-card--elevated" id={menuId} role="menu">
                <button
                  type="button"
                  role="menuitem"
                  className="rf-btn header__entry"
                  onClick={choose(onOpenPreferences)}
                >
                  {t("header.preferences")}
                </button>
                <button
                  type="button"
                  role="menuitem"
                  className="rf-btn header__entry"
                  onClick={choose(onOpenAbout)}
                >
                  {t("header.about")}
                </button>
              </div>
            )}
          </div>
        )}
      </div>
    </header>
  );
}
