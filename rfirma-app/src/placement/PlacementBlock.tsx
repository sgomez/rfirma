//! El bloque «Colocación»: el segmentado «Una página | Varias | Todas», la línea de «Una página», el campo de «Varias» con su error y el botón de la página, pintados de lo que entrega el estado.

import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../design-system/Button";
import { AlertIcon } from "../design-system/icons";
import { Stack } from "../design-system/Stack";
import { messageFor } from "./placementField";
import type { PlacementState } from "./usePlacement";

/** Lo que el bloque lee del estado de la colocación. */
export type PlacementBlockState = Pick<
  PlacementState,
  | "placing"
  | "pageMode"
  | "pagesText"
  | "rangeError"
  | "pageAction"
  | "changePageMode"
  | "typePages"
  | "sealViewedPage"
  | "unsealViewedPage"
>;

const MODES = [
  { mode: "single", label: "panel.placement.single" },
  { mode: "these", label: "panel.placement.these" },
  { mode: "all", label: "panel.placement.all" },
] as const;

/** El segmentado de los modos de páginas y la línea o el campo que va debajo. */
export function PlacementBlock({ state }: { state: PlacementBlockState }) {
  const { t } = useTranslation();
  const group = useId();
  const { placing, pageMode, pagesText, rangeError, pageAction } = state;

  const button = pageAction && (
    <Button
      variant="secondary"
      className="panel__page-button"
      onClick={pageAction === "seal" ? state.sealViewedPage : state.unsealViewedPage}
    >
      {pageAction === "seal" ? t("panel.placement.seal") : t("panel.placement.unseal")}
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
              onChange={() => state.changePageMode(mode)}
            />
            {t(label)}
          </label>
        ))}
      </div>

      {pageMode === "single" && placing.sets.single !== null && (
        <div className="panel__page-line">
          <span className="panel__page-text">
            {t("panel.placement.singlePage", { page: placing.sets.single })}
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
              onChange={(event) => state.typePages(event.target.value)}
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
