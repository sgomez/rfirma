//! El segmentado de páginas de la firma visible —«Una página», «Varias», «Todas»— y la línea o el campo de debajo.

import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import { AlertIcon } from "../design-system/icons";
import { Stack } from "../design-system/Stack";
import type { PageMode, PageSets } from "../placement/pageSets";
import { type FieldTrouble, messageFor, type PageButton } from "../placement/placementField";

interface PlacementFieldsetProps {
  pageSets: PageSets;
  pageMode: PageMode;
  onChangePageMode: (mode: PageMode) => void;
  pagesText: string;
  onTypePages: (value: string) => void;
  rangeError: FieldTrouble | null;
  pageButton: PageButton | null;
}

const MODES = [
  { mode: "single", label: "panel.placement.single" },
  { mode: "these", label: "panel.placement.these" },
  { mode: "all", label: "panel.placement.all" },
] as const;

/** El segmentado «Una página | Varias | Todas» y la línea o el campo que va debajo. */
export function PlacementFieldset({
  pageSets,
  pageMode,
  onChangePageMode,
  pagesText,
  onTypePages,
  rangeError,
  pageButton,
}: PlacementFieldsetProps) {
  const { t } = useTranslation();
  const group = useId();

  const button = pageButton && (
    <Button variant="secondary" className="panel__page-button" onClick={pageButton.act}>
      {pageButton.label}
    </Button>
  );

  return (
    <>
      <div className="panel__segmented" role="radiogroup" aria-label={t("panel.placement.title")}>
        {MODES.map(({ mode, label }) => (
          <label
            key={mode}
            className={
              pageMode === mode ? "panel__segment panel__segment--chosen" : "panel__segment"
            }
          >
            <input
              type="radio"
              className="panel__segment-input"
              name={group}
              checked={pageMode === mode}
              onChange={() => onChangePageMode(mode)}
            />
            {t(label)}
          </label>
        ))}
      </div>

      {pageMode === "single" && pageSets.single !== null && (
        <div className="panel__page-line">
          <span className="panel__page-text">
            {t("panel.placement.singlePage", { page: pageSets.single })}
          </span>
          {button}
        </div>
      )}

      {pageMode === "these" && (
        <Stack className="panel__range">
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
        </Stack>
      )}
    </>
  );
}
