import { useTranslation } from "react-i18next";
import { UploadIcon } from "../design-system/icons";
import type { ShownBadge } from "./document";
import "./DocumentTray.css";
import { type RecentDocument, shownBadge } from "./recents";

const BADGE_KEY = {
  Signed: "badges.signed",
  Unsigned: "badges.unsigned",
  Unavailable: "badges.unavailable",
} as const satisfies Record<ShownBadge, string>;

interface DocumentTrayProps {
  recents: readonly RecentDocument[];
  /** El identificador del documento activo, o `null`. */
  activeId: string | null;
  /** Abrir un documento, que va por el portal. Ver [`DocumentPicker`]. */
  onOpen: () => void;
  onSelect: (document: RecentDocument) => void;
  onForget: (id: string) => void;
}

/**
 * La columna izquierda: **qué documento se firma**.
 *
 * Es el único punto de entrada de documentos de la aplicación, y por eso la
 * zona de soltar es un botón que llama al portal y no un `<input type="file">`:
 * el segundo sería un camino paralelo, y además no existe bajo el sandbox.
 *
 * Las filas se pintan con los metadatos cacheados —nombre, insignia y fecha—
 * **sin abrir ningún fichero** (ADR-0010). Un documento que ya no responde sale con
 * la insignia `No disponible`, se atenúa y ofrece quitarla, pero **sigue en la
 * lista**: un PDF en un USB desmontado no está borrado.
 */
export function DocumentTray({ recents, activeId, onOpen, onSelect, onForget }: DocumentTrayProps) {
  const { t, i18n } = useTranslation();
  const dates = new Intl.DateTimeFormat(i18n.language, { dateStyle: "short" });

  return (
    <div className="tray">
      <button type="button" className="tray__drop-zone" onClick={onOpen}>
        <span className="tray__drop-icon">
          <UploadIcon />
        </span>
        <span className="rf-prose tray__drop-text">{t("tray.dropZone")}</span>
      </button>
      {recents.length === 0 ? (
        <p className="rf-prose rf-text-muted">{t("tray.empty")}</p>
      ) : (
        <>
          <p className="rf-label tray__heading">{t("tray.recents")}</p>
          <ul className="tray__list" aria-label={t("tray.recents")}>
            {recents.map((document) => {
              const badge = shownBadge(document);
              const unavailable = badge === "Unavailable";
              return (
                <li key={document.id} className="tray__row">
                  <button
                    type="button"
                    className={rowClass(document.id === activeId, unavailable)}
                    aria-current={document.id === activeId}
                    disabled={unavailable}
                    onClick={() => onSelect(document)}
                  >
                    <span className="rf-prose tray__name">{document.name}</span>
                    <span className="rf-row rf-gap-xs">
                      <span className="rf-badge">{t(BADGE_KEY[badge])}</span>
                      <span className="rf-body rf-text-muted">
                        {dates.format(document.lastUsed * 1000)}
                      </span>
                    </span>
                  </button>
                  {unavailable && (
                    <button
                      type="button"
                      className="rf-btn rf-btn--ghost tray__forget"
                      onClick={() => onForget(document.id)}
                    >
                      {t("tray.remove")}
                    </button>
                  )}
                </li>
              );
            })}
          </ul>
        </>
      )}
    </div>
  );
}

function rowClass(selected: boolean, unavailable: boolean): string {
  const classes = ["tray__card", "rf-card", "rf-card--interactive"];
  if (selected) classes.push("tray__card--selected");
  if (unavailable) classes.push("tray__card--unavailable");
  return classes.join(" ");
}
