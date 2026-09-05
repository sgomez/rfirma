import type { TFunction } from "i18next";
import { useCallback, useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { CertificateIcon, CheckIcon, ChevronDownIcon } from "../design-system/icons";
import type { Certificate } from "./certificate";
import { groupCertificates, isUsable } from "./certificate";
import "./CertificateSelect.css";

interface CertificateSelectProps {
  certificates: readonly Certificate[];
  /**
   * El elegido, o `null` mientras no hay ninguno.
   *
   * Quién viene puesto al arrancar lo decide `chosenFrom`, no este componente:
   * el que se usó en la última firma, y sin él —o sin nada recordado— ninguno.
   */
  chosen: Certificate | null;
  onChoose: (certificate: Certificate) => void;
}

/**
 * **Con qué certificado se firma**: un desplegable, no una tarjeta
 * (docs/design/panel-de-firma.md).
 *
 * Tres cosas que parecen detalles y son el componente entero:
 *
 * - **La lista va superpuesta**, no en flujo. Abrirla no mueve la firma visible
 *   ni el botón de firmar, y con nueve certificados el panel sigue midiendo lo
 *   mismo. Un acordeón que empuja el contenido saca el botón primario de la
 *   vista justo mientras se elige.
 * - **La fila se identifica por el asa** que acuñó el backend, no por la
 *   etiqueta: dos claves con el mismo `CKA_LABEL` son dos filas distintas, y
 *   por etiqueta se firmaba siempre con la primera.
 * - **Un certificado que no sirve se lista igual**, dice por qué y no se deja
 *   elegir. Esconderlo sería más limpio y peor: quien viene a firmar justo con
 *   ese se quedaría mirando una lista donde falta, sin saber por qué.
 *
 * El teclado es el de `combobox` + `listbox` con `aria-activedescendant`, el
 * mismo que el desplegable de Preferencias: un `<div>` con un `onClick` no es
 * un desplegable, es un dibujo de uno.
 */
export function CertificateSelect({ certificates, chosen, onChoose }: CertificateSelectProps) {
  const { t, i18n } = useTranslation();
  const [open, setOpen] = useState(false);
  // Dónde está el cursor del teclado mientras la lista está abierta. No es la
  // elección: moverse por la lista no elige nada hasta que se pulsa Intro.
  const [active, setActive] = useState(0);
  // La posición del panel, calculada al abrir a partir del disparador: el
  // panel vive en un portal, fuera de la columna que recorta, así que ya no
  // puede colgar de su disparador con `position: absolute` normal.
  const [anchor, setAnchor] = useState<{ top: number; left: number; width: number } | null>(null);
  const container = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const listId = useId();
  const optionId = useId();

  // Agrupados y ordenados de una vez: el índice que mueve el teclado y el que
  // identifica cada fila son los mismos en toda la función, así que se calcula
  // una sola vez el orden en que la lista realmente se pinta.
  const groups = groupCertificates(certificates);
  const ordered = [...groups.available, ...groups.unusable];

  const at = chosen === null ? -1 : ordered.findIndex((one) => one.id === chosen.id);

  const close = useCallback((giveBackFocus: boolean) => {
    setOpen(false);
    if (giveBackFocus) button.current?.focus();
  }, []);

  // Al abrir, el cursor arranca en lo que ya está elegido; sin nada elegido, en
  // la primera fila, que **no** es elegirla.
  const show = () => {
    setActive(at === -1 ? 0 : at);
    const rect = button.current?.getBoundingClientRect();
    if (rect) setAnchor({ top: rect.bottom + 4, left: rect.left, width: rect.width });
    setOpen(true);
  };

  // El foco se va a la lista para que el lector de pantalla la anuncie y para
  // que las flechas no muevan el panel de debajo.
  useEffect(() => {
    if (open) list.current?.focus();
  }, [open]);

  // Pulsar fuera cierra, igual que el menú de la cabecera. Sin esto la lista se
  // queda flotando sobre el panel mientras se toca otra cosa. El panel vive en
  // un portal, fuera de `container`, así que también cuenta como «dentro».
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
    // Una fila inutilizable se recorre y se lee, pero no elige: el cursor puede
    // pararse en ella para que el motivo se anuncie.
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
        // Tabular sale del control, así que la lista se va con él, pero el foco
        // sigue su camino: devolverlo al botón lo dejaría atrapado.
        close(false);
        return;
      default:
    }
  };

  /** Una fila del `ordered` que corresponde al `index` global de la lista
   * aplanada, dentro del grupo que la pinta. */
  const renderOption = (certificate: Certificate, index: number) => {
    const usable = isUsable(certificate.status);
    return (
      <div
        key={certificate.id}
        id={`${optionId}-${index}`}
        role="option"
        // El foco lo guarda la lista y el cursor lo lleva
        // `aria-activedescendant`: la fila no entra en el orden de
        // tabulación.
        tabIndex={-1}
        aria-selected={certificate.id === chosen?.id}
        aria-disabled={!usable}
        className={[
          "certificate-select__option",
          index === active ? "certificate-select__option--active" : "",
          usable ? "" : "certificate-select__option--unusable",
        ]
          .filter((piece) => piece !== "")
          .join(" ")}
        // `onPointerDown` y no `onClick`: el oyente que cierra al pulsar fuera
        // también es de `pointerdown`, y con `click` la lista se desmontaría
        // antes de que llegara el clic. Una fila no utilizable no llega aquí
        // siquiera: `pointer-events: none` la saca del todo (CertificateSelect.css).
        onPointerDown={(event) => {
          event.preventDefault();
          choose(index);
        }}
        onPointerEnter={() => setActive(index)}
      >
        <span className="certificate-select__text">
          <span className="rf-title certificate-select__holder">{certificate.holderName}</span>
          <span className="rf-body rf-text-muted certificate-select__line">
            {[
              certificate.idNumber,
              t("panel.certificate.issuer", { issuer: certificate.issuer }),
              t(`panel.certificate.stores.${certificate.store}`),
            ]
              .filter((piece) => piece !== "")
              .join(" · ")}
          </span>
          {!usable && (
            <span className="rf-body certificate-select__reason">
              {statusWarning(certificate.status, i18n.language, t)}
            </span>
          )}
        </span>
        <span className="certificate-select__check">
          {certificate.id === chosen?.id && <CheckIcon size={16} />}
        </span>
      </div>
    );
  };

  return (
    <div className="certificate-select" ref={container}>
      <button
        type="button"
        ref={button}
        className="certificate-select__trigger"
        role="combobox"
        aria-expanded={open}
        aria-controls={open ? listId : undefined}
        aria-haspopup="listbox"
        aria-label={t("panel.certificate.title")}
        onClick={() => (open ? close(false) : show())}
        onKeyDown={(event) => {
          if (!open && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
            event.preventDefault();
            show();
          }
        }}
      >
        <span className="certificate-select__icon">
          <CertificateIcon />
        </span>
        {chosen === null ? (
          // Sin nada recordado no hay preselección: elegir con qué identidad se
          // firma un documento con validez jurídica no lo hace la aplicación
          // por su cuenta.
          <span className="rf-body certificate-select__unchosen">
            {t("panel.certificate.choose")}
          </span>
        ) : (
          <span className="certificate-select__text">
            <span className="rf-title certificate-select__holder">{chosen.holderName}</span>
            {/* El almacén **no** sale en el disparador: elegido ya no
                desambigua nada. */}
            <span className="rf-body rf-text-muted certificate-select__line">
              {[chosen.idNumber, t("panel.certificate.issuer", { issuer: chosen.issuer })]
                .filter((piece) => piece !== "")
                .join(" · ")}
            </span>
          </span>
        )}
        <span className={open ? "certificate-select__arrow--up" : "certificate-select__arrow"}>
          <ChevronDownIcon />
        </span>
      </button>
      {open &&
        anchor &&
        createPortal(
          <div
            className="certificate-select__layer"
            style={{ top: anchor.top, left: anchor.left, width: anchor.width }}
          >
            <div
              className="certificate-select__list"
              ref={list}
              id={listId}
              role="listbox"
              tabIndex={-1}
              aria-label={t("panel.certificate.list")}
              aria-activedescendant={`${optionId}-${active}`}
              onKeyDown={onKeyDown}
            >
              {groups.available.length > 0 && (
                <>
                  <div className="certificate-select__group-label" role="presentation">
                    {t("panel.certificate.groups.available")}
                  </div>
                  {groups.available.map((certificate, index) => renderOption(certificate, index))}
                </>
              )}
              {groups.unusable.length > 0 && (
                <>
                  <div className="certificate-select__group-label" role="presentation">
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

/** Por qué no se puede firmar con este certificado, dicho antes del PIN. */
export function statusWarning(status: Certificate["status"], locale: string, t: TFunction): string {
  switch (status.kind) {
    case "expired":
      return t("panel.certificate.expired", {
        date: new Intl.DateTimeFormat(locale, { dateStyle: "long" }).format(status.notAfter * 1000),
      });
    case "notYetValid":
      return t("panel.certificate.notYetValid");
    case "revoked":
      return t("panel.certificate.revoked", { reason: status.reason });
    default:
      return t("panel.certificate.unreadable");
  }
}
