import { useId } from "react";
import { useTranslation } from "react-i18next";
import { AlertIcon } from "../design-system/icons";
import type { PageChoice, PageSets } from "../viewer/signatureBox";
import { type FieldTrouble, messageFor, type SealButton } from "./placementField";

interface PlacementFieldsetProps {
  documentPages: number;
  pageSets: PageSets;
  pageChoice: PageChoice;
  onChangePageChoice: (choice: PageChoice) => void;
  pagesText: string;
  onTypePages: (value: string) => void;
  rangeError: FieldTrouble | null;
  echo: string | null;
  sealedCount: number;
  sealButton: SealButton;
}

/**
 * El bloque «Colocación»: los tres modos de página y el botón de sellar, a
 * todo el ancho y bajo los radios (#194).
 */
export function PlacementFieldset({
  documentPages,
  pageSets,
  pageChoice,
  onChangePageChoice,
  pagesText,
  onTypePages,
  rangeError,
  echo,
  sealedCount,
  sealButton,
}: PlacementFieldsetProps) {
  const { t } = useTranslation();
  const placementName = useId();

  return (
    <fieldset className="panel__placement">
      <legend className="rf-label">{t("panel.placement.title")}</legend>

      <label className="panel__placement-option">
        <input
          type="radio"
          name={placementName}
          checked={pageChoice === "single"}
          onChange={() => onChangePageChoice("single")}
        />
        <span className="rf-body">{t("panel.placement.single")}</span>
        {/* La etiqueta es fija y el número va en el pie: «esta página» no dice
            cuál y deja de ser cierto en cuanto pasas de página (ID-97). Su
            página, no la del conjunto activo: con «Todas» delante este pie
            sigue diciendo la suya, que es la que volverá si se elige (#188). */}
        <span className="rf-hint panel__placement-foot">
          {pageSets.single === null
            ? t("panel.placement.singleUnplaced")
            : t("panel.placement.singlePage", { page: pageSets.single })}
        </span>
      </label>

      <label className="panel__placement-option">
        <input
          type="radio"
          name={placementName}
          checked={pageChoice === "these"}
          onChange={() => onChangePageChoice("these")}
        />
        <span className="rf-body">{t("panel.placement.these")}</span>
      </label>

      {pageChoice === "these" && (
        <div
          className={
            rangeError === null
              ? "rf-field panel__placement-field"
              : "rf-field rf-field--error panel__placement-field"
          }
        >
          <input
            className="rf-input"
            type="text"
            inputMode="numeric"
            value={pagesText}
            aria-label={t("panel.placement.field")}
            aria-invalid={rangeError !== null}
            placeholder="1,2-3,10-20"
            onChange={(event) => onTypePages(event.target.value)}
          />
          {rangeError === null ? (
            echo !== null && <p className="rf-hint">{echo}</p>
          ) : (
            <p className="rf-hint panel__placement-error">
              <span className="panel__notice-icon">
                <AlertIcon />
              </span>
              <span>{messageFor(rangeError, t)}</span>
            </p>
          )}
        </div>
      )}

      <label className="panel__placement-option">
        <input
          type="radio"
          name={placementName}
          checked={pageChoice === "all"}
          onChange={() => onChangePageChoice("all")}
        />
        <span className="rf-body">{t("panel.placement.all", { pages: documentPages })}</span>
      </label>

      <button
        type="button"
        className={`rf-btn ${sealButton.variant} panel__placement-seal`}
        onClick={sealButton.act}
      >
        {sealButton.label}
      </button>

      {/* Un solo campo de firma con el widget replicado, no una firma por
          página: es lo que se estampa, y decirlo aquí evita prometer trece
          firmas. */}
      {sealedCount > 1 && (
        <p className="rf-hint">{t("panel.placement.replicated", { count: sealedCount })}</p>
      )}
    </fieldset>
  );
}
