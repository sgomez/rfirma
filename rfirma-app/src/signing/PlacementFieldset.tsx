import { useId } from "react";
import { useTranslation } from "react-i18next";
import { AlertIcon } from "../design-system/icons";
import type { PageChoice, PageSets } from "../viewer/signatureBox";
import { type FieldTrouble, messageFor, type PageButton } from "./placementField";

interface PlacementFieldsetProps {
  pageSets: PageSets;
  pageChoice: PageChoice;
  onChangePageChoice: (choice: PageChoice) => void;
  pagesText: string;
  onTypePages: (value: string) => void;
  rangeError: FieldTrouble | null;
  pageButton: PageButton | null;
}

const CHOICES = [
  { choice: "single", label: "panel.placement.single" },
  { choice: "these", label: "panel.placement.these" },
  { choice: "all", label: "panel.placement.all" },
] as const;

/** El segmentado «Una página | Varias | Todas» y la línea o el campo que va debajo. */
export function PlacementFieldset({
  pageSets,
  pageChoice,
  onChangePageChoice,
  pagesText,
  onTypePages,
  rangeError,
  pageButton,
}: PlacementFieldsetProps) {
  const { t } = useTranslation();
  const group = useId();

  const button = pageButton && (
    <button
      type="button"
      className="rf-btn rf-btn--secondary panel__page-button"
      onClick={pageButton.act}
    >
      {pageButton.label}
    </button>
  );

  return (
    <>
      <div className="panel__segmented" role="radiogroup" aria-label={t("panel.placement.title")}>
        {CHOICES.map(({ choice, label }) => (
          <label
            key={choice}
            className={
              pageChoice === choice ? "panel__segment panel__segment--chosen" : "panel__segment"
            }
          >
            <input
              type="radio"
              className="panel__segment-input"
              name={group}
              checked={pageChoice === choice}
              onChange={() => onChangePageChoice(choice)}
            />
            {t(label)}
          </label>
        ))}
      </div>

      {pageChoice === "single" && pageSets.single !== null && (
        <div className="panel__page-line">
          <span className="panel__page-text">
            {t("panel.placement.singlePage", { page: pageSets.single })}
          </span>
          {button}
        </div>
      )}

      {pageChoice === "these" && (
        <div className="rf-stack panel__range">
          <div className="panel__range-row">
            <input
              className={
                rangeError === null
                  ? "rf-input panel__range-input"
                  : "rf-input panel__range-input panel__range-input--error"
              }
              type="text"
              inputMode="numeric"
              value={pagesText}
              aria-label={t("panel.placement.field")}
              aria-invalid={rangeError !== null}
              onChange={(event) => onTypePages(event.target.value)}
            />
            {button}
          </div>
          {rangeError !== null && (
            <p className="panel__range-error">
              <AlertIcon size={15} />
              <span className="rf-body">{messageFor(rangeError, t)}</span>
            </p>
          )}
        </div>
      )}
    </>
  );
}
