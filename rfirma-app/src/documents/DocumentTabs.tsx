import {
  type RefObject,
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  ChevronDownIcon,
  CloseIcon,
  FileIcon,
  FolderIcon,
  SignedMarkIcon,
} from "../design-system/icons";
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
}

/** Las pestañas de los documentos abiertos, con el botón partido de abrir, dentro de la cabecera. */
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
}: DocumentTabsProps) {
  const { t } = useTranslation();
  const strip = useRef<HTMLElement>(null);
  const available = useAvailableWidth(strip);
  const { visible, hidden } =
    available === null ? { visible: tabs, hidden: [] } : layOutTabs(available, tabs, activeId);

  return (
    <nav className="document-tabs" aria-label={t("tabs.label")} ref={strip}>
      <SplitOpenButton
        recents={recents}
        openIds={new Set(tabs.map((tab) => tab.id))}
        onOpen={onOpen}
        onSelectRecent={onSelectRecent}
        onClearRecents={onClearRecents}
      />
      <div className="document-tabs__list" role="tablist">
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
              <button
                type="button"
                role="tab"
                aria-selected={active}
                aria-disabled={locked}
                className="document-tab__select"
                disabled={locked}
                onClick={() => onActivate(tab.id)}
              >
                <span className="document-tab__icon">
                  <FileIcon size={14} />
                </span>
                <span className="document-tab__name">{tab.name}</span>
                {tab.badge === "Signed" && (
                  <span className="document-tab__signed" role="img" aria-label={t("badges.signed")}>
                    <SignedMarkIcon />
                  </span>
                )}
              </button>
              <button
                type="button"
                className="document-tab__close"
                aria-label={t("tabs.close", { name: tab.name })}
                onClick={() => onClose(tab.id)}
              >
                <CloseIcon />
              </button>
            </div>
          );
        })}
      </div>
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
  const menuId = useId();
  const close = useCallback(() => setOpen(false), []);

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

  return (
    <div className="document-tabs__more-menu" ref={container}>
      <button
        type="button"
        className={
          open
            ? "rf-btn rf-btn--ghost document-tabs__more document-tabs__more--open"
            : "rf-btn rf-btn--ghost document-tabs__more"
        }
        title={t("tabs.more")}
        aria-label={t("tabs.more")}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        onClick={() => setOpen((was) => !was)}
      >
        +{hidden.length}
        <ChevronDownIcon size={14} strokeWidth={2} />
      </button>
      {open && (
        <div id={menuId} role="menu" className="open-menu open-menu--right">
          {hidden.map((tab) => (
            <button
              key={tab.id}
              type="button"
              role="menuitem"
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
                  <SignedMarkIcon />
                </span>
              )}
            </button>
          ))}
        </div>
      )}
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
  const [open, setOpen] = useState(false);
  const [alignRight, setAlignRight] = useState(false);
  const container = useRef<HTMLDivElement>(null);
  const menuId = useId();
  const hasRecents = recents.length > 0;
  const close = useCallback(() => setOpen(false), []);

  useLayoutEffect(() => {
    if (!open || !container.current) return;
    const { left } = container.current.getBoundingClientRect();
    setAlignRight(left + MENU_WIDTH > window.innerWidth);
  }, [open]);

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

  const choose = (action: () => void) => {
    close();
    action();
  };

  return (
    <div className="document-tabs__split" ref={container}>
      <button
        type="button"
        className="document-tabs__split-open"
        title={t("tabs.openPdfShortcut")}
        onClick={onOpen}
      >
        <FolderIcon size={15} />
        {t("tabs.openPdf")}
      </button>
      {hasRecents && (
        <>
          <span className="document-tabs__split-divider" aria-hidden="true" />
          <button
            type="button"
            className={
              open
                ? "document-tabs__split-arrow document-tabs__split-arrow--open"
                : "document-tabs__split-arrow"
            }
            title={t("tabs.recentlyOpened")}
            aria-label={t("tabs.recentlyOpened")}
            aria-haspopup="menu"
            aria-expanded={open}
            aria-controls={open ? menuId : undefined}
            onClick={() => setOpen((was) => !was)}
          >
            <ChevronDownIcon size={14} strokeWidth={2} />
          </button>
          {open && (
            <div
              id={menuId}
              role="menu"
              className={alignRight ? "open-menu open-menu--right" : "open-menu"}
            >
              <span className="rf-label open-menu__heading">{t("recents.heading")}</span>
              <RecentRows
                recents={recents}
                openIds={openIds}
                role="menuitem"
                onSelect={(row) => choose(() => onSelectRecent(row))}
              />
              <hr className="rf-divider open-menu__divider" />
              <button
                type="button"
                role="menuitem"
                className="open-menu__clear"
                onClick={() => choose(onClearRecents)}
              >
                {t("recents.clear")}
              </button>
            </div>
          )}
        </>
      )}
    </div>
  );
}

function useAvailableWidth(strip: RefObject<HTMLElement | null>): number | null {
  const [width, setWidth] = useState<number | null>(null);
  useLayoutEffect(() => {
    const element = strip.current;
    if (!element || typeof ResizeObserver === "undefined") return;
    const measure = () => {
      const inner = element.clientWidth - STRIP_PADDING - SPLIT_BUTTON_WIDTH;
      setWidth(inner > 0 ? inner : null);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [strip]);
  return width;
}
