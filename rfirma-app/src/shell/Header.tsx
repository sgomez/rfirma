//! La cabecera única, sin barra de menús (ADR-0007): el hueco de las pestañas, el aviso de estado y el menú cuando se ancla en la cabecera.

import { type ReactNode, useCallback, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import { Icon } from "../design-system/icons";
import { Menu, MenuItem } from "../design-system/Menu";
import "./Header.css";
import type { ReaderStatus } from "../signing/certificate";
import type { MenuAnchor } from "./menuAnchor";
import { ReaderIndicator } from "./ReaderIndicator";

interface HeaderProps {
  /** Dónde va el menú. Ver [`MenuAnchor`]. */
  menuAnchor: MenuAnchor;
  /**
   * Si la barra lleva el botón de aviso: certificado de rFirma ausente o a
   * medias, o nadie atendiendo las sedes (docs/design/cabecera.md, sección
   * «El aviso»).
   */
  hasAttention?: boolean;
  /** Si hay lector de tarjetas; sin él, la cabecera no lo dice. */
  reader?: ReaderStatus;
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
 * En Windows y macOS el menú es el botón de esta cabecera. En Linux todo
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
  reader,
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
  const trigger = useRef<HTMLButtonElement>(null);

  const close = useCallback(() => setOpen(false), []);

  const choose = (action: () => void) => () => {
    close();
    action();
  };

  if (menuAnchor === "titlebar") {
    return documents === null && reader === undefined ? null : (
      <header className="header header--tabsOnly">
        {documents}
        {reader && (
          <div className="header__end">
            <ReaderIndicator reader={reader} />
          </div>
        )}
      </header>
    );
  }

  return (
    <header className={open ? "header header--menuOpen" : "header"}>
      {documents}
      <span className="header__gap" />
      <div className="header__end">
        {reader && <ReaderIndicator reader={reader} />}
        {hasAttention && (
          <Button
            className="header__button header__attention"
            aria-label={t("header.attention")}
            title={t("header.attention")}
            onClick={onOpenStatus}
          >
            <Icon name="warning" size={16} />
          </Button>
        )}
        {menuAnchor === "header" && (
          <div className="header__menu" ref={container}>
            <Button
              ref={trigger}
              className={open ? "header__button header__button--open" : "header__button"}
              aria-label={t("header.menu")}
              aria-haspopup="menu"
              aria-expanded={open}
              aria-controls={open ? menuId : undefined}
              onClick={() => setOpen((wasOpen) => !wasOpen)}
            >
              <Icon name="menu" size={18} />
            </Button>
            <Menu
              open={open}
              onClose={close}
              anchorRef={container}
              returnFocusRef={trigger}
              className="header__popup"
              id={menuId}
            >
              <MenuItem className="header__entry" onClick={choose(onOpenStatus)}>
                <span className="header__entryLabel">{t("status.title")}</span>
                <span className="header__entryIcon" aria-hidden="true" />
              </MenuItem>
              <hr className="rf-divider" />
              <MenuItem className="header__entry" onClick={choose(onOpenPreferences)}>
                <span className="header__entryLabel">{t("header.preferences")}</span>
                <span className="header__entryIcon" aria-hidden="true" />
              </MenuItem>
              <MenuItem className="header__entry" onClick={choose(onOpenHelp)}>
                <span className="header__entryLabel">{t("header.help")}</span>
                <span className="header__entryIcon" aria-hidden="true">
                  <Icon name="externalLink" />
                </span>
              </MenuItem>
              <MenuItem className="header__entry" onClick={choose(onOpenAbout)}>
                <span className="header__entryLabel">{t("header.about")}</span>
                <span className="header__entryIcon" aria-hidden="true" />
              </MenuItem>
            </Menu>
          </div>
        )}
      </div>
    </header>
  );
}
