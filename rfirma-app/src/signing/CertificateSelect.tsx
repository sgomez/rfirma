import type { TFunction } from "i18next";
import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import {
  CheckIcon,
  ChevronDownIcon,
  ClockIcon,
  RevokedIcon,
  SearchIcon,
  SpinnerIcon,
} from "../design-system/icons";
import type { Certificate } from "./certificate";
import {
  certificateCompactSubtitle,
  certificateHeadline,
  certificateSubtitle,
  expiryMonthYear,
  groupCertificates,
  isUsable,
} from "./certificate";
import "./CertificateSelect.css";

type Store = Certificate["stores"][number];

interface CertificateSelectProps {
  certificates: readonly Certificate[];
  /** El elegido, o `null` mientras no hay ninguno. */
  chosen: Certificate | null;
  onChoose: (certificate: Certificate) => void;
  /** El alto máximo de la lista abierta, en px; la ventana lo recorta si no cabe. */
  listMaxHeight?: number;
  /** Mientras se listan los certificados: la caja lo dice y no se abre. */
  searching?: boolean;
  disabled?: boolean;
}

interface Anchor {
  top: number;
  left: number;
  width: number;
  maxHeight: number;
}

const WINDOW_MARGIN = 8;

/** Con qué certificado se firma: la caja de dos líneas que al abrirse es un buscador (docs/design/panel-de-firma.md). */
export function CertificateSelect({
  certificates,
  chosen,
  onChoose,
  listMaxHeight = 480,
  searching = false,
  disabled = false,
}: CertificateSelectProps) {
  const { t, i18n } = useTranslation();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [anchor, setAnchor] = useState<Anchor | null>(null);
  const frame = useRef<HTMLDivElement>(null);
  const box = useRef<HTMLButtonElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const layer = useRef<HTMLDivElement>(null);
  const focusBoxOnClose = useRef(false);
  const labelId = useId();
  const listId = useId();
  const optionId = useId();

  const groups = groupCertificates(certificates);
  const all = [...groups.available, ...groups.unusable];
  const shown = all.filter((certificate) => matches(certificate, query, t));
  const shownAvailable = shown.filter((certificate) => isUsable(certificate.status));
  const shownUnusable = shown.filter((certificate) => !isUsable(certificate.status));
  const withHeaders = certificates.length > 1;

  const close = useCallback((giveBackFocus: boolean) => {
    focusBoxOnClose.current = giveBackFocus;
    setOpen(false);
    setQuery("");
  }, []);

  const show = () => {
    const at = chosen === null ? -1 : all.findIndex((one) => one.id === chosen.id);
    setActive(Math.max(at, 0));
    setOpen(true);
  };

  const place = useCallback(() => {
    const rect = frame.current?.getBoundingClientRect();
    if (!rect) return;
    const top = rect.bottom + 4;
    setAnchor({
      top,
      left: rect.left,
      width: rect.width,
      maxHeight: Math.min(listMaxHeight, window.innerHeight - top - WINDOW_MARGIN),
    });
  }, [listMaxHeight]);

  useLayoutEffect(() => {
    if (!open) return;
    place();
    const onScroll = (event: Event) => {
      if (layer.current?.contains(event.target as Node)) return;
      place();
    };
    window.addEventListener("resize", place);
    window.addEventListener("scroll", onScroll, true);
    return () => {
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", onScroll, true);
    };
  }, [open, place]);

  useEffect(() => {
    if (!open) return;
    document.getElementById(`${optionId}-${active}`)?.scrollIntoView?.({ block: "nearest" });
  }, [open, active, optionId]);

  useEffect(() => {
    if (open) {
      search.current?.focus();
    } else if (focusBoxOnClose.current) {
      focusBoxOnClose.current = false;
      box.current?.focus();
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as Node;
      if (frame.current?.contains(target) || layer.current?.contains(target)) return;
      close(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open, close]);

  const choose = (index: number) => {
    const certificate = shown[index];
    if (certificate === undefined || !isUsable(certificate.status)) return;
    onChoose(certificate);
    close(true);
  };

  const onSearchKeyDown = (event: React.KeyboardEvent) => {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        setActive((cursor) => Math.min(cursor + 1, shown.length - 1));
        return;
      case "ArrowUp":
        event.preventDefault();
        setActive((cursor) => Math.max(cursor - 1, 0));
        return;
      case "Enter":
        event.preventDefault();
        choose(active);
        return;
      case "Escape":
        event.preventDefault();
        close(true);
        return;
      case "Tab":
        close(false);
        return;
      default:
    }
  };

  const renderOption = (certificate: Certificate, index: number) => {
    const usable = isUsable(certificate.status);
    const selected = certificate.id === chosen?.id;
    const stores = certificate.stores;
    return (
      <div
        key={certificate.id}
        id={`${optionId}-${index}`}
        role="option"
        tabIndex={-1}
        aria-selected={selected}
        aria-disabled={!usable}
        title={
          usable
            ? rowTooltip(certificate, i18n.language, t)
            : shortStatusWarning(certificate.status, i18n.language, t)
        }
        className={[
          "certificate-select__option",
          index === active ? "certificate-select__option--active" : "",
          selected ? "certificate-select__option--chosen" : "",
          usable ? "" : "certificate-select__option--unusable",
        ]
          .filter((piece) => piece !== "")
          .join(" ")}
        // `onPointerDown` y no `onClick`: el oyente que cierra al pulsar fuera también es de `pointerdown`.
        onPointerDown={(event) => {
          event.preventDefault();
          choose(index);
        }}
        onPointerEnter={() => setActive(index)}
      >
        <span className="certificate-select__text">
          <span className="certificate-select__headline">{certificateHeadline(certificate)}</span>
          <span className="rf-body certificate-select__line">
            {certificateSubtitle(certificate, t)}
          </span>
          <span className="certificate-select__meta">
            {stores.map((store) => (
              <span key={store} className="rf-badge certificate-select__store">
                {storeLabel(store, t)}
              </span>
            ))}
            {certificate.status.kind === "valid" && (
              <span className="rf-body rf-text-muted certificate-select__expiry">
                {t("panel.certificate.expiresIn", {
                  date: expiryMonthYear(certificate.status.notAfter),
                })}
              </span>
            )}
          </span>
          {!usable && (
            <span className="certificate-select__reason">
              <StatusIcon status={certificate.status} />
              <span className="rf-body">
                {shortStatusWarning(certificate.status, i18n.language, t)}
              </span>
            </span>
          )}
        </span>
        {selected && (
          <span className="certificate-select__check">
            <CheckIcon size={16} strokeWidth={2} />
          </span>
        )}
      </div>
    );
  };

  const renderGroup = (label: string, members: readonly Certificate[], offset: number) => {
    if (members.length === 0) return null;
    const options = members.map((certificate, index) => renderOption(certificate, offset + index));
    if (!withHeaders) return options;
    return (
      // biome-ignore lint/a11y/useSemanticElements: dentro de un `listbox` el grupo de opciones es `role="group"`; un `<fieldset>` no.
      <div role="group" aria-label={label} className="certificate-select__group">
        <span className="rf-label certificate-select__group-label" aria-hidden="true">
          {label}
        </span>
        {options}
      </div>
    );
  };

  return (
    <div className="rf-stack certificate-select">
      <span className="rf-label" id={labelId}>
        {t("panel.certificate.title")}
      </span>
      <div className="certificate-select__frame" ref={frame}>
        {open ? (
          <div className="certificate-select__search">
            <span className="certificate-select__search-icon">
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
              aria-activedescendant={shown.length > 0 ? `${optionId}-${active}` : undefined}
              placeholder={t("panel.certificate.search")}
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
            className="certificate-select__box"
            role="combobox"
            aria-labelledby={labelId}
            aria-expanded="false"
            aria-haspopup="listbox"
            disabled={searching || disabled}
            onClick={show}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown" || event.key === "ArrowUp") {
                event.preventDefault();
                show();
              }
            }}
          >
            {searching ? (
              <>
                <span className="certificate-select__spinner">
                  <SpinnerIcon size={16} />
                </span>
                <span className="rf-text-muted certificate-select__unchosen">
                  {t("panel.certificate.loading")}
                </span>
              </>
            ) : chosen === null ? (
              <span className="rf-text-muted certificate-select__unchosen">
                {t("panel.certificate.chooseOne")}
              </span>
            ) : (
              <span className="certificate-select__text">
                <span className="certificate-select__chosen">{certificateHeadline(chosen)}</span>
                <span className="rf-body rf-text-muted certificate-select__ellipsis">
                  {certificateCompactSubtitle(chosen, t)}
                </span>
              </span>
            )}
            <span className="certificate-select__arrow">
              <ChevronDownIcon strokeWidth={1.8} />
            </span>
          </button>
        )}
      </div>
      {open &&
        anchor &&
        createPortal(
          <div
            ref={layer}
            className="certificate-select__layer"
            style={{
              top: anchor.top,
              left: anchor.left,
              width: anchor.width,
              maxHeight: anchor.maxHeight,
            }}
          >
            {query.trim() !== "" && shown.length > 0 && (
              <span className="rf-body rf-text-muted certificate-select__count">
                {t("panel.certificate.matches", { shown: shown.length, total: all.length })}
              </span>
            )}
            {shown.length === 0 && (
              <span className="rf-body rf-text-muted certificate-select__empty">
                {t("panel.certificate.noMatch")}
              </span>
            )}
            <div
              className="certificate-select__list"
              id={listId}
              role="listbox"
              aria-labelledby={labelId}
            >
              {renderGroup(t("panel.certificate.groups.available"), shownAvailable, 0)}
              {renderGroup(
                t("panel.certificate.groups.cannotUse"),
                shownUnusable,
                shownAvailable.length,
              )}
            </div>
          </div>,
          document.body,
        )}
    </div>
  );
}

function storeLabel(store: Store, t: TFunction): string {
  switch (store) {
    case "card":
      return t("panel.certificate.stores.card");
    case "firefox":
      return t("panel.certificate.stores.firefox");
    case "chrome":
      return t("panel.certificate.stores.chrome");
    case "nssdb":
      return t("panel.certificate.stores.nssdb");
    case "installed":
      return t("panel.certificate.stores.installed");
    case "windows":
      return t("panel.certificate.stores.windows");
  }
}

function fold(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}

function matches(certificate: Certificate, query: string, t: TFunction): boolean {
  const wanted = fold(query.trim());
  if (wanted === "") return true;
  const haystack = [
    certificate.holderName,
    certificate.entityName ?? "",
    certificate.organizationIdentifier ?? "",
    certificate.idNumber,
    certificate.issuer,
    ...certificate.stores.map((store) => storeLabel(store, t)),
    certificate.entityName === null ? t("panel.certificate.personalKeyword") : "",
  ];
  return haystack.some((piece) => fold(piece).includes(wanted));
}

function rowTooltip(certificate: Certificate, locale: string, t: TFunction): string {
  const issuer = t("panel.certificate.issuer", { issuer: certificate.issuer });
  const stores = certificate.stores;
  if (stores.length < 2) return issuer;
  const names = new Intl.ListFormat(locale, { type: "conjunction" }).format(
    stores.map((store) => storeLabel(store, t)),
  );
  return `${issuer} · ${t("panel.certificate.sameCertificateIn", { stores: names })}`;
}

function StatusIcon({ status }: { status: Certificate["status"] }) {
  switch (status.kind) {
    case "expired":
    case "notYetValid":
      return <ClockIcon />;
    case "revoked":
      return <RevokedIcon />;
    default:
      return null;
  }
}

/** Por qué no se puede firmar con este certificado, en la frase corta de su fila. */
function shortStatusWarning(status: Certificate["status"], locale: string, t: TFunction): string {
  switch (status.kind) {
    case "expired":
      return t("panel.certificate.expiredShort", {
        date: new Intl.DateTimeFormat(locale, { dateStyle: "long" }).format(status.notAfter * 1000),
      });
    case "notYetValid":
      return t("panel.certificate.notYetValidShort");
    case "revoked":
      return t("panel.certificate.revokedShort", { reason: status.reason });
    default:
      return t("panel.certificate.unreadableShort");
  }
}
