import { useCallback, useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertIcon, ExternalLinkIcon, MenuIcon } from "../design-system/icons";
import "./Header.css";
import type { MenuAnchor } from "./menuAnchor";

interface HeaderProps {
  /** Dónde va el menú. Ver [`MenuAnchor`]. */
  menuAnchor: MenuAnchor;
  /**
   * Si «Estado de rFirma» lleva el triángulo de aviso: certificado de rFirma
   * ausente o a medias, o nadie atendiendo las sedes
   * (docs/design/cabecera.md, sección «El aviso»).
   */
  hasAttention?: boolean;
  onOpenStatus: () => void;
  onOpenPreferences: () => void;
  onOpenHelp: () => void;
  onOpenAbout: () => void;
}

/**
 * La franja superior de la ventana: identidad y el **único** menú de la
 * aplicación. Sin certificado ni insignia de documento: el certificado lo
 * dice el botón «Firmar como» del panel y el estado, la pestaña
 * (docs/design/cabecera.md).
 *
 * No hay barra de menús: el ADR-0007 la retiró, y por eso aquí no hay
 * `role="menubar"` ni entradas de *Archivo* o *Ver*. Abrir un documento tiene
 * la zona de soltar de la bandeja y guardar tiene la fila «Guardar en» del
 * panel; repetirlos en un menú sería un segundo camino para lo mismo.
 *
 * En macOS las dos entradas se registran en el menú de aplicación nativo, así
 * que el botón de menú **se oculta** en vez de quedarse vacío.
 *
 * El menú **arranca cerrado**. El artboard del estado vacío lo dibuja
 * desplegado, pero eso enseña una posibilidad y no un estado inicial: una
 * aplicación que abre con un menú encima del documento no es lo que el canvas
 * pide.
 */
export function Header({
  menuAnchor,
  hasAttention = false,
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

  return (
    <header className={open ? "header header--menuOpen" : "header"}>
      <p className="header__name rf-title">{t("app.name")}</p>
      <div className="rf-row">
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
              <MenuIcon />
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
                  <span
                    className={
                      hasAttention
                        ? "header__entryIcon header__entryIcon--attention"
                        : "header__entryIcon"
                    }
                    aria-hidden={!hasAttention}
                  >
                    {hasAttention && (
                      <span role="img" aria-label={t("header.statusAttention")}>
                        <AlertIcon size={14} />
                      </span>
                    )}
                  </span>
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
