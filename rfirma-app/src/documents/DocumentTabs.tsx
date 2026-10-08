//! La tira de pestañas, con el menú «+N» de las ocultas y el botón partido de abrir salvo en Linux.

import { type RefObject, useCallback, useId, useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import { Icon } from "../design-system/icons";
import { Menu, MenuItem } from "../design-system/Menu";
import { SplitButton } from "../design-system/SplitButton";
import { Tab, Tabs } from "../design-system/Tabs";
import type { DocumentInHand } from "./document";
import "./DocumentTabs.css";
import { RecentRows } from "./RecentRows";
import type { RecentDocument } from "./recents";
import { layOutTabs } from "./tabLayout";

const MENU_WIDTH = 340;
const SPLIT_BUTTON_WIDTH = 160;
const STRIP_PADDING = 16;

interface DocumentTabsProps {
  tabs: readonly DocumentInHand[];
  activeId: string | null;
  recents: readonly RecentDocument[];
  onActivate: (id: string) => void;
  onClose: (id: string) => void;
  /** Abrir un documento por el portal. */
  onOpen: () => void;
  onSelectRecent: (row: RecentDocument) => void;
  onClearRecents: () => void;
  /** Mientras una firma está en curso, las demás pestañas no se activan. */
  signingLocked?: boolean;
  /** En Linux el botón partido va en la barra de título GTK, no aquí. */
  withOpenButton?: boolean;
}

/** Las pestañas de los documentos abiertos, con el botón partido de abrir salvo en Linux. */
export function DocumentTabs({
  tabs,
  activeId,
  recents,
  onActivate,
  onClose,
  onOpen,
  onSelectRecent,
  onClearRecents,
  signingLocked = false,
  withOpenButton = true,
}: DocumentTabsProps) {
  const { t } = useTranslation();
  const strip = useRef<HTMLElement>(null);
  const available = useAvailableWidth(strip, withOpenButton ? SPLIT_BUTTON_WIDTH : 0);
  const { visible, hidden } =
    available === null ? { visible: tabs, hidden: [] } : layOutTabs(available, tabs, activeId);

  return (
    <nav className="document-tabs" aria-label={t("tabs.label")} ref={strip}>
      {withOpenButton && (
        <SplitOpenButton
          recents={recents}
          openIds={new Set(tabs.map((tab) => tab.id))}
          onOpen={onOpen}
          onSelectRecent={onSelectRecent}
          onClearRecents={onClearRecents}
        />
      )}
      <Tabs className="document-tabs__list">
        {visible.map((tab) => {
          const active = tab.id === activeId;
          const locked = signingLocked && !active;
          return (
            <div
              key={tab.id}
              role="presentation"
              className={active ? "document-tab document-tab--active" : "document-tab"}
              title={locked ? t("tabs.lockedWhileSigning") : tab.name}
            >
              <Tab
                selected={active}
                aria-disabled={locked}
                className="document-tab__select"
                disabled={locked}
                onClick={() => onActivate(tab.id)}
              >
                <span className="document-tab__icon">
                  <Icon name="document" size={14} />
                </span>
                <span className="document-tab__name">{tab.name}</span>
                {tab.badge === "Signed" && (
                  <span className="document-tab__signed" role="img" aria-label={t("badges.signed")}>
                    <Icon name="signed" />
                  </span>
                )}
              </Tab>
              <button
                type="button"
                className="document-tab__close"
                aria-label={t("tabs.close", { name: tab.name })}
                onClick={() => onClose(tab.id)}
              >
                <Icon name="close" />
              </button>
            </div>
          );
        })}
      </Tabs>
      {hidden.length > 0 && <HiddenTabsMenu hidden={hidden} onActivate={onActivate} />}
    </nav>
  );
}

interface HiddenTabsMenuProps {
  hidden: readonly DocumentInHand[];
  onActivate: (id: string) => void;
}

/** El menú de las pestañas ocultas, desde «+N ▾». */
function HiddenTabsMenu({ hidden, onActivate }: HiddenTabsMenuProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const container = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const menuId = useId();
  const close = useCallback(() => setOpen(false), []);

  return (
    <div className="document-tabs__more-menu" ref={container}>
      <Button
        variant="ghost"
        ref={trigger}
        className={open ? "document-tabs__more document-tabs__more--open" : "document-tabs__more"}
        title={t("tabs.more")}
        aria-label={t("tabs.more")}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        onClick={() => setOpen((was) => !was)}
      >
        +{hidden.length}
        <Icon name="dropdown" size={14} strokeWidth={2} />
      </Button>
      <Menu
        open={open}
        onClose={close}
        anchorRef={container}
        returnFocusRef={trigger}
        id={menuId}
        className="open-menu open-menu--right"
      >
        {hidden.map((tab) => (
          <MenuItem
            key={tab.id}
            className="hidden-tab-row"
            title={tab.name}
            onClick={() => {
              close();
              onActivate(tab.id);
            }}
          >
            <span className="hidden-tab-row__name">{tab.name}</span>
            {tab.badge === "Signed" && (
              <span className="hidden-tab-row__signed" role="img" aria-label={t("badges.signed")}>
                <Icon name="signed" />
              </span>
            )}
          </MenuItem>
        ))}
      </Menu>
    </div>
  );
}

interface OpenMenuProps {
  recents: readonly RecentDocument[];
  openIds: ReadonlySet<string>;
  onOpen: () => void;
  onSelectRecent: (row: RecentDocument) => void;
  onClearRecents: () => void;
}

/** El botón partido de abrir: «Abrir PDF…» y la flecha de «Abiertos recientemente». */
function SplitOpenButton({
  recents,
  openIds,
  onOpen,
  onSelectRecent,
  onClearRecents,
}: OpenMenuProps) {
  const { t } = useTranslation();
  const [alignRight, setAlignRight] = useState(false);
  const anchor = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    const measure = () => {
      if (!anchor.current) return;
      const { left } = anchor.current.getBoundingClientRect();
      setAlignRight(left + MENU_WIDTH > window.innerWidth);
    };
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, []);

  const items =
    recents.length > 0 ? (
      <>
        <span className="rf-label open-menu__heading">{t("recents.heading")}</span>
        <RecentRows recents={recents} openIds={openIds} inMenu onSelect={onSelectRecent} />
        <hr className="rf-divider" />
        <MenuItem className="open-menu__clear" onClick={onClearRecents}>
          {t("recents.clear")}
        </MenuItem>
      </>
    ) : undefined;

  return (
    <div className="document-tabs__split-anchor" ref={anchor}>
      <SplitButton
        variant="ghost"
        className="document-tabs__split"
        menuClassName={alignRight ? "open-menu open-menu--right" : "open-menu"}
        title={t("tabs.openPdfShortcut")}
        onAction={onOpen}
        menuLabel={t("recents.heading")}
        items={items}
      >
        <Icon name="folder" size={15} />
        {t("tabs.openPdf")}
      </SplitButton>
    </div>
  );
}

function useAvailableWidth(
  strip: RefObject<HTMLElement | null>,
  openButtonWidth: number,
): number | null {
  const [width, setWidth] = useState<number | null>(null);
  useLayoutEffect(() => {
    const element = strip.current;
    if (!element || typeof ResizeObserver === "undefined") return;
    const measure = () => {
      const inner = element.clientWidth - STRIP_PADDING - openButtonWidth;
      setWidth(inner > 0 ? inner : null);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [strip, openButtonWidth]);
  return width;
}
