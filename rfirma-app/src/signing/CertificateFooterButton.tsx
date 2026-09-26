import { useCallback, useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { CheckIcon, ChevronDownIcon } from "../design-system/icons";
import "./CertificateFooterButton.css";
import { statusWarning } from "./CertificateSelect";
import type { Certificate } from "./certificate";
import { firstNameAndSurname, groupCertificates, isUsable } from "./certificate";

interface CertificateFooterButtonProps {
  certificates: readonly Certificate[];
  /** El elegido, o `null` mientras no hay ninguno. */
  chosen: Certificate | null;
  onChoose: (certificate: Certificate) => void;
  onSign: () => void;
  /** Mientras la firma corre, el chevron no abre la lista (ID-93). */
  signing: boolean;
  /** Con el interruptor encendido y sin colocar, o con el rango en error. */
  blocked: boolean;
  /** Sustituye «Firmar como…» cuando el pie está enseñando un fallo previo. */
  signLabel?: string;
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
  signLabel,
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

  // La lista **siempre** cuelga hacia arriba: vive en el pie, y en Tauri la
  // ventana corta por abajo (docs/design/panel-de-firma.md § Certificado).
  const show = () => {
    setActive(at === -1 ? 0 : at);
    const rect = trigger.current?.getBoundingClientRect();
    if (rect) {
      setAnchor({
        bottom: window.innerHeight - rect.top + 4,
        left: rect.left,
        width: rect.width,
        maxHeight: Math.max(100, Math.min(232, rect.top - 8)),
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
        className={[
          "certificate-footer__option",
          index === active ? "certificate-footer__option--active" : "",
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
          <span className="rf-title certificate-footer__holder">{certificate.holderName}</span>
          <span className="rf-body rf-text-muted certificate-footer__line">
            {[
              t("panel.certificate.issuer", { issuer: certificate.issuer }),
              t(`panel.certificate.stores.${certificate.store}`),
              certificate.status.kind === "valid"
                ? t("panel.certificate.expiresOn", {
                    date: new Intl.DateTimeFormat(i18n.language, { dateStyle: "long" }).format(
                      certificate.status.notAfter * 1000,
                    ),
                  })
                : null,
            ]
              .filter((piece) => piece !== null)
              .join(" · ")}
          </span>
          {!usable && (
            <span className="rf-body certificate-footer__reason">
              {statusWarning(certificate.status, i18n.language, t)}
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
      <div className="certificate-footer__row">
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
            <span>{t("panel.certificate.choose")}</span>
            <span className={open ? "certificate-footer__arrow--up" : "certificate-footer__arrow"}>
              <ChevronDownIcon />
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
              {signLabel ??
                t(signing ? "panel.footer.signingAs" : "panel.footer.signAs", {
                  name: firstNameAndSurname(chosen.holderName),
                })}
            </button>
            <button
              type="button"
              ref={trigger}
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
              <ChevronDownIcon />
            </button>
          </>
        )}
      </div>
      {/* Un recordado que caducó desde la última firma no debería llegar
          «chosen» (App.signingOrder.ts lo filtra), pero si ocurriera el motivo
          se dice aquí y no solo en la fila de la lista. */}
      {unusable && (
        <p className="rf-prose certificate-footer__warning" role="alert">
          {statusWarning(chosen.status, i18n.language, t)}
        </p>
      )}
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
                  <div className="certificate-footer__group-label" role="presentation">
                    {t("panel.certificate.groups.available")}
                  </div>
                  {groups.available.map((certificate, index) => renderOption(certificate, index))}
                </>
              )}
              {groups.unusable.length > 0 && (
                <>
                  <div className="certificate-footer__group-label" role="presentation">
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
