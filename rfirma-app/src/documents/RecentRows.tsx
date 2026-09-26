import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { SignedMarkIcon } from "../design-system/icons";
import "./DocumentTabs.css";
import type { RecentDocument } from "./recents";

interface RecentRowsProps {
  recents: readonly RecentDocument[];
  /** Los identificadores de los documentos que ya tienen pestaña. */
  openIds: ReadonlySet<string>;
  onSelect: (row: RecentDocument) => void;
  /** El rol de cada fila: `menuitem` dentro del menú «+», botón suelto fuera de él. */
  role?: "menuitem";
}

/** Las filas de los recientes, que comparten el menú «+» y el estado vacío del visor. */
export function RecentRows({ recents, openIds, onSelect, role }: RecentRowsProps) {
  const { t, i18n } = useTranslation();
  const now = new Date();

  return recents.map((row) => {
    const open = openIds.has(row.id);
    const missing = !row.available;
    return (
      <button
        key={row.id}
        type="button"
        role={role}
        className="recent-row"
        disabled={missing}
        title={missing ? t("recents.missing") : open ? t("recents.goToTab") : undefined}
        onClick={() => onSelect(row)}
      >
        <span className="recent-row__text">
          <span className="recent-row__title">
            <span className="recent-row__name">{row.name}</span>
            {row.badge === "Signed" && (
              <span className="recent-row__signed" role="img" aria-label={t("badges.signed")}>
                <SignedMarkIcon />
              </span>
            )}
          </span>
          {missing && <span className="recent-row__folder">{t("recents.missing")}</span>}
          {!missing && row.folder && <span className="recent-row__folder">{row.folder}</span>}
        </span>
        <span className={open ? "recent-row__when recent-row__when--open" : "recent-row__when"}>
          {open ? t("recents.open") : whenUsed(row.lastUsed, now, i18n.language, t)}
        </span>
      </button>
    );
  });
}

interface RecentsSectionProps {
  recents: readonly RecentDocument[];
  onSelect: (row: RecentDocument) => void;
  onClear: () => void;
}

/** Los recientes del estado vacío del visor, con su rótulo y «Vaciar la lista». */
export function RecentsSection({ recents, onSelect, onClear }: RecentsSectionProps) {
  const { t } = useTranslation();
  if (recents.length === 0) return null;
  return (
    <section className="recents-section" aria-label={t("recents.heading")}>
      <div className="recents-heading">
        <span className="rf-label recents-heading__label">{t("recents.heading")}</span>
        <button
          type="button"
          className="rf-btn rf-btn--ghost recents-heading__clear"
          onClick={onClear}
        >
          {t("recents.clear")}
        </button>
      </div>
      <RecentRows recents={recents} openIds={NOTHING_OPEN} onSelect={onSelect} />
    </section>
  );
}

const NOTHING_OPEN: ReadonlySet<string> = new Set();

function whenUsed(lastUsed: number, now: Date, language: string, t: TFunction): string {
  const used = new Date(lastUsed * 1000);
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const day = new Date(used.getFullYear(), used.getMonth(), used.getDate());
  const daysAgo = Math.round((today.getTime() - day.getTime()) / 86_400_000);
  if (daysAgo === 0) return t("recents.today");
  if (daysAgo === 1) return t("recents.yesterday");
  return new Intl.DateTimeFormat(language, { day: "numeric", month: "short" }).format(used);
}
