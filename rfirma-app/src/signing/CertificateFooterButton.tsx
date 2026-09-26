import { useCallback, useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { CheckIcon, ChevronDownIcon, SpinnerIcon } from "../design-system/icons";
import "./CertificateFooterButton.css";
import { shortStatusWarning } from "./CertificateSelect";
import type { Certificate } from "./certificate";
import { firstNameAndSurname, groupCertificates, isUsable } from "./certificate";

interface CertificateFooterButtonProps {
  certificates: readonly Certificate[];
  /** El elegido, o `null` mientras no hay ninguno. */
  chosen: Certificate | null;
  onChoose: (certificate: Certificate) => void;
  onSign: () => void;
  /** Mientras la firma corre, el chevron no abre la lista y la fila se atenúa. */
  signing: boolean;
  /** Con el interruptor encendido y sin colocar, o con el rango en error. */
  blocked: boolean;
}

/** «Caduca en 06/2027»: mes y año de caducidad, sin traducir su formato. */
function expiryMonthYear(notAfter: number): string {
  const date = new Date(notAfter * 1000);
  return `${String(date.getMonth() + 1).padStart(2, "0")}/${date.getFullYear()}`;
}

/**
 * El botón partido del pie: «Firmar como <nombre y primer apellido>» y el
 * chevron que abre la lista de certificados hacia arriba
 * (docs/design/panel-de-firma.md § Certificado).
 *
 * Sin nada elegido es **un solo botón**, «Elegir certificado ▾», que abre la
 * lista en vez de firmar: no hay nada con qué hacerlo todavía.
 */
export function CertificateFooterButton({
  certificates,
  chosen,
  onChoose,
  onSign,
  signing,
  blocked,
}: CertificateFooterButtonProps) {
  const { t, i18n } = useTranslation();
  const [open, setOpen] = useState(false);
  // Dónde está el cursor del teclado mientras la lista está abierta.
  const [active, setActive] = useState(0);
  const [anchor, setAnchor] = useState<{
    bottom: number;
    left: number;
    width: number;
    maxHeight: number;
  } | null>(null);
  const container = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const listId = useId();
  const optionId = useId();

  const groups = groupCertificates(certificates);
  const ordered = [...groups.available, ...groups.unusable];
  const at = chosen === null ? -1 : ordered.findIndex((one) => one.id === chosen.id);

  const close = useCallback((giveBackFocus: boolean) => {
    setOpen(false);
    if (giveBackFocus) trigger.current?.focus();
  }, []);

  // La lista **siempre** cuelga hacia arriba, anclada al panel y no al
  // disparador: con un certificado elegido el disparador es el chevron de
  // 44 px, y anclarse a él sacaba la lista con su mismo ancho
  // (docs/design/panel-de-firma.md § Geometría).
  const show = () => {
    setActive(at === -1 ? 0 : at);
    const panelRect = container.current?.closest(".panel")?.getBoundingClientRect();
    if (panelRect) {
      setAnchor({
        bottom: window.innerHeight - panelRect.bottom + 68,
        left: panelRect.left + 24,
        width: panelRect.width - 48,
        maxHeight: Math.max(100, panelRect.height - 76),
      });
    }
    setOpen(true);
  };

  useEffect(() => {
    if (open) list.current?.focus();
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as Node;
      if (container.current?.contains(target)) return;
      if (list.current?.contains(target)) return;
      setOpen(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  const choose = (index: number) => {
    const certificate = ordered[index];
    if (certificate === undefined || !isUsable(certificate.status)) return;
    onChoose(certificate);
    close(true);
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    const last = ordered.length - 1;
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        setActive((cursor) => Math.min(cursor + 1, last));
        return;
      case "ArrowUp":
        event.preventDefault();
        setActive((cursor) => Math.max(cursor - 1, 0));
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
        close(false);
        return;
      default:
    }
  };

  const toggle = () => (open ? close(false) : show());
  const openOnArrow = (event: React.KeyboardEvent) => {
    if (!open && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
      event.preventDefault();
      show();
    }
  };

  const renderOption = (certificate: Certificate, index: number) => {
    const usable = isUsable(certificate.status);
    return (
      <div
        key={certificate.id}
        id={`${optionId}-${index}`}
        role="option"
        tabIndex={-1}
        aria-selected={certificate.id === chosen?.id}
        aria-disabled={!usable}
        title={
          usable ? certificate.holderName : shortStatusWarning(certificate.status, i18n.language, t)
        }
        className={[
          "certificate-footer__option",
          index === active ? "certificate-footer__option--active" : "",
          certificate.id === chosen?.id ? "certificate-footer__option--chosen" : "",
          usable ? "" : "certificate-footer__option--unusable",
        ]
          .filter((piece) => piece !== "")
          .join(" ")}
        onPointerDown={(event) => {
          event.preventDefault();
          choose(index);
        }}
        onPointerEnter={() => setActive(index)}
      >
        <span className="certificate-footer__text">
          <span className="certificate-footer__holder">{certificate.holderName}</span>
          <span className="rf-body rf-text-muted certificate-footer__line">
            {[
              certificate.issuer,
              t(`panel.certificate.stores.${certificate.store}`),
              certificate.status.kind === "valid"
                ? t("panel.certificate.expiresIn", {
                    date: expiryMonthYear(certificate.status.notAfter),
                  })
                : null,
            ]
              .filter((piece) => piece !== null)
              .join(" · ")}
          </span>
          {!usable && (
            <span className="rf-body certificate-footer__reason">
              {shortStatusWarning(certificate.status, i18n.language, t)}
            </span>
          )}
        </span>
        <span className="certificate-footer__check">
          {certificate.id === chosen?.id && <CheckIcon size={16} />}
        </span>
      </div>
    );
  };

  const unusable = chosen !== null && !isUsable(chosen.status);

  return (
    <div className="certificate-footer" ref={container}>
      <div
        className={
          signing
            ? "certificate-footer__row certificate-footer__row--dim"
            : "certificate-footer__row"
        }
      >
        {chosen === null ? (
          <button
            type="button"
            ref={trigger}
            className="rf-btn rf-btn--primary certificate-footer__unchosen"
            role="combobox"
            aria-expanded={open}
            aria-controls={open ? listId : undefined}
            aria-haspopup="listbox"
            aria-label={t("panel.certificate.title")}
            onClick={toggle}
            onKeyDown={openOnArrow}
          >
            <span className="certificate-footer__unchosen-text">
              {t("panel.certificate.choose")}
            </span>
            <span className={open ? "certificate-footer__arrow--up" : "certificate-footer__arrow"}>
              <ChevronDownIcon size={14} strokeWidth={2} />
            </span>
          </button>
        ) : (
          <>
            <button
              type="button"
              className="rf-btn rf-btn--primary certificate-footer__sign"
              title={chosen.holderName}
              disabled={signing || blocked || unusable}
              onClick={onSign}
            >
              <span className="certificate-footer__verb">
                {t(signing ? "panel.footer.signingVerb" : "panel.footer.signVerb")}
              </span>{" "}
              <span className="certificate-footer__holder-name">{firstNameAndSurname(chosen)}</span>
            </button>
            <button
              type="button"
              ref={trigger}
              title={t("panel.certificate.changeTitle")}
              className="certificate-footer__chevron"
              role="combobox"
              aria-expanded={open}
              aria-controls={open ? listId : undefined}
              aria-haspopup="listbox"
              aria-label={t("panel.certificate.title")}
              disabled={signing}
              onClick={toggle}
              onKeyDown={openOnArrow}
            >
              <span
                className={open ? "certificate-footer__arrow--up" : "certificate-footer__arrow"}
              >
                <ChevronDownIcon size={14} strokeWidth={2} />
              </span>
            </button>
          </>
        )}
      </div>
      {open &&
        anchor &&
        createPortal(
          <div
            className="certificate-footer__layer"
            style={{ bottom: anchor.bottom, left: anchor.left, width: anchor.width }}
          >
            <div
              className="certificate-footer__list"
              ref={list}
              id={listId}
              role="listbox"
              tabIndex={-1}
              style={{ maxHeight: anchor.maxHeight }}
              aria-label={t("panel.certificate.list")}
              aria-activedescendant={`${optionId}-${active}`}
              onKeyDown={onKeyDown}
            >
              {groups.available.length > 0 && (
                <>
                  <div className="rf-label certificate-footer__group-label" role="presentation">
                    {t("panel.certificate.groups.available")}
                  </div>
                  {groups.available.map((certificate, index) => renderOption(certificate, index))}
                </>
              )}
              {groups.unusable.length > 0 && (
                <>
                  <div className="rf-label certificate-footer__group-label" role="presentation">
                    {t("panel.certificate.groups.unusable")}
                  </div>
                  {groups.unusable.map((certificate, index) =>
                    renderOption(certificate, groups.available.length + index),
                  )}
                </>
              )}
            </div>
          </div>,
          document.body,
        )}
    </div>
  );
}

/**
 * El mismo botón partido mientras se busca: sin certificado que nombrar, con
 * el indicador de 16 px y el ▾ inerte (docs/design/panel-de-firma.md §
 * Estados → Buscando certificados).
 */
export function LoadingCertificateFooterButton() {
  const { t } = useTranslation();
  return (
    <div className="certificate-footer">
      <div className="certificate-footer__row certificate-footer__row--dim">
        <button type="button" className="rf-btn rf-btn--primary certificate-footer__sign" disabled>
          <span className="certificate-footer__spinner">
            <SpinnerIcon size={16} />
          </span>
          <span className="certificate-footer__verb">{t("panel.certificate.loading")}</span>
        </button>
        <span
          className="certificate-footer__chevron certificate-footer__chevron--inert"
          aria-hidden="true"
        >
          <ChevronDownIcon size={14} strokeWidth={2} />
        </span>
      </div>
    </div>
  );
}
