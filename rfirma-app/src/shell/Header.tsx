//! La cabecera única, sin barra de menús (ADR-0007): el hueco de las pestañas, el aviso de estado y el menú cuando se ancla en la cabecera.

import { type ReactNode, useCallback, useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertIcon, ExternalLinkIcon, MenuIcon } from "../design-system/icons";
import "./Header.css";
import type { MenuAnchor } from "./menuAnchor";

interface HeaderProps {
  /** Dónde va el menú. Ver [`MenuAnchor`]. */
  menuAnchor: MenuAnchor;
  /**
   * Si la barra lleva el botón de aviso: certificado de rFirma ausente o a
   * medias, o nadie atendiendo las sedes (docs/design/cabecera.md, sección
   * «El aviso»).
   */
  hasAttention?: boolean;
  /** Lo que se pinta antes del menú, o nada en las vistas sin documentos. */
  documents?: ReactNode;
  onOpenStatus: () => void;
  onOpenPreferences: () => void;
  onOpenHelp: () => void;
  onOpenAbout: () => void;
}

/**
 * La barra única de la ventana: el hueco de los documentos y el **único**
 * menú de la aplicación. Sin certificado ni insignia de documento: el certificado lo
 * dice el selector del panel y el estado, la pestaña
 * (docs/design/cabecera.md).
 *
 * No hay barra de menús: el ADR-0007 la retiró, y por eso aquí no hay
 * `role="menubar"` ni entradas de *Archivo* o *Ver*. Abrir un documento tiene
 * la zona de soltar de la bandeja y guardar tiene la fila «Guardar en» del
 * panel; repetirlos en un menú sería un segundo camino para lo mismo.
 *
 * En macOS las dos entradas se registran en el menú de aplicación nativo, así
 * que el botón de menú **se oculta** en vez de quedarse vacío. En Linux todo
 * menos las pestañas va en la barra de título GTK, y sin pestañas no hay cabecera.
 *
 * El menú **arranca cerrado**. El artboard del estado vacío lo dibuja
 * desplegado, pero eso enseña una posibilidad y no un estado inicial: una
 * aplicación que abre con un menú encima del documento no es lo que el canvas
 * pide.
 */
export function Header({
  menuAnchor,
  hasAttention = false,
  documents = null,
  onOpenStatus,
  onOpenPreferences,
  onOpenHelp,
  onOpenAbout,
}: HeaderProps) {
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
      if (event.key === "Escape") {
        event.preventDefault();
        close();
      }
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

  if (menuAnchor === "titlebar") {
    return documents === null ? null : (
      <header className="header header--tabsOnly">{documents}</header>
    );
  }

  return (
    <header className={open ? "header header--menuOpen" : "header"}>
      {documents}
      <span className="header__gap" />
      <div className="header__end">
        {hasAttention && (
          <button
            type="button"
            className="rf-btn header__button header__attention"
            aria-label={t("header.attention")}
            title={t("header.attention")}
            onClick={onOpenStatus}
          >
            <AlertIcon size={16} />
          </button>
        )}
        {menuAnchor === "header" && (
          <div className="header__menu" ref={container}>
            <button
              type="button"
              className={
                open ? "rf-btn header__button header__button--open" : "rf-btn header__button"
              }
              aria-label={t("header.menu")}
              aria-haspopup="menu"
              aria-expanded={open}
              aria-controls={open ? menuId : undefined}
              onClick={() => setOpen((wasOpen) => !wasOpen)}
            >
              <MenuIcon size={18} />
            </button>
            {open && (
              <div className="header__popup rf-card rf-card--elevated" id={menuId} role="menu">
                <button
                  type="button"
                  role="menuitem"
                  className="rf-btn header__entry"
                  onClick={choose(onOpenStatus)}
                >
                  <span className="header__entryLabel">{t("header.status")}</span>
                  <span className="header__entryIcon" aria-hidden="true" />
                </button>
                <hr className="rf-divider header__divider" />
                <button
                  type="button"
                  role="menuitem"
                  className="rf-btn header__entry"
                  onClick={choose(onOpenPreferences)}
                >
                  <span className="header__entryLabel">{t("header.preferences")}</span>
                  <span className="header__entryIcon" aria-hidden="true" />
                </button>
                <button
                  type="button"
                  role="menuitem"
                  className="rf-btn header__entry"
                  onClick={choose(onOpenHelp)}
                >
                  <span className="header__entryLabel">{t("header.help")}</span>
                  <span className="header__entryIcon" aria-hidden="true">
                    <ExternalLinkIcon />
                  </span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  className="rf-btn header__entry"
                  onClick={choose(onOpenAbout)}
                >
                  <span className="header__entryLabel">{t("header.about")}</span>
                  <span className="header__entryIcon" aria-hidden="true" />
                </button>
              </div>
            )}
          </div>
        )}
      </div>
    </header>
  );
}
