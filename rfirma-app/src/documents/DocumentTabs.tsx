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
  ChevronLeftIcon,
  ChevronRightIcon,
  CloseIcon,
  FileIcon,
  FolderIcon,
  SignedMarkIcon,
} from "../design-system/icons";
import type { DocumentInHand } from "./document";
import "./DocumentTabs.css";
import { RecentRows } from "./RecentRows";
import type { RecentDocument } from "./recents";

const TAB_WIDTH = 200;
const NARROW_TAB_WIDTH = 160;
const GAP = 2;
const MENU_WIDTH = 340;
const SLOT_WIDTH = 34;
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

/** Las pestañas de los documentos abiertos, con su menú «+», dentro de la cabecera. */
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
  const list = useRef<HTMLDivElement>(null);
  const available = useAvailableWidth(strip);
  const narrow = available !== null && tabs.length * (TAB_WIDTH + GAP) > available;
  const overflows = narrow && tabs.length * (NARROW_TAB_WIDTH + GAP) > available;
  const step = (narrow ? NARROW_TAB_WIDTH : TAB_WIDTH) + GAP;

  const activeIndex = tabs.findIndex((tab) => tab.id === activeId);

  useEffect(() => {
    const scroller = list.current;
    const active = scroller?.children.item(activeIndex);
    if (!scroller || !(active instanceof HTMLElement)) return;
    if (active.offsetLeft < scroller.scrollLeft) scroller.scrollLeft = active.offsetLeft;
    const right = active.offsetLeft + active.offsetWidth;
    if (right > scroller.scrollLeft + scroller.clientWidth) {
      scroller.scrollLeft = right - scroller.clientWidth;
    }
  }, [activeIndex]);

  const scrollBy = (delta: number) => {
    if (list.current) list.current.scrollLeft += delta;
  };

  return (
    <nav className="document-tabs" aria-label={t("tabs.label")} ref={strip}>
      <SplitOpenButton
        recents={recents}
        openIds={new Set(tabs.map((tab) => tab.id))}
        onOpen={onOpen}
        onSelectRecent={onSelectRecent}
        onClearRecents={onClearRecents}
      />
      {overflows && (
        <button
          type="button"
          className="document-tabs__slot"
          title={t("tabs.previous")}
          aria-label={t("tabs.previous")}
          onClick={() => scrollBy(-step)}
        >
          <span className="document-tabs__slot-button">
            <ChevronLeftIcon />
          </span>
        </button>
      )}
      <div className="document-tabs__list" ref={list} role="tablist">
        {tabs.map((tab) => {
          const active = tab.id === activeId;
          const locked = signingLocked && !active;
          return (
            <div
              key={tab.id}
              role="presentation"
              className={active ? "document-tab document-tab--active" : "document-tab"}
              style={{ width: narrow ? NARROW_TAB_WIDTH : TAB_WIDTH }}
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
      {overflows && (
        <button
          type="button"
          className="document-tabs__slot"
          title={t("tabs.next")}
          aria-label={t("tabs.next")}
          onClick={() => scrollBy(step)}
        >
          <span className="document-tabs__slot-button">
            <ChevronRightIcon />
          </span>
        </button>
      )}
    </nav>
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
            <ChevronDownIcon />
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
      const inner = element.clientWidth - STRIP_PADDING - 2 * SLOT_WIDTH - SPLIT_BUTTON_WIDTH;
      setWidth(inner > 0 ? inner : null);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [strip]);
  return width;
}
