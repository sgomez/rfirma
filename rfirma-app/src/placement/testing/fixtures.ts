//! El estado del bloque «Colocación» ya resuelto y sin gancho detrás, para las historias y las pruebas de las ventanas que lo montan.

import type { PlacementBlockState } from "../PlacementBlock";
import {
  NO_PAGE_SETS,
  type PageMode,
  type PageSets,
  placementOf,
  type UserSpaceRect,
} from "../pageSets";
import { fieldTroubleOf, pageActionOf, typedTextOf } from "../placementField";

interface Situation {
  rect?: UserSpaceRect | null;
  sets?: PageSets;
  mode?: PageMode;
  viewedPage?: number;
  pageCount: number;
}

const noop = () => {};

/** El bloque tal y como lo deja el estado para esa colocación, con órdenes que no hacen nada. */
export function placementStateOf({
  rect = null,
  sets = NO_PAGE_SETS,
  mode = "single",
  viewedPage = 1,
  pageCount,
}: Situation): PlacementBlockState {
  const pagesText = typedTextOf(sets.these, pageCount);
  const rangeError = fieldTroubleOf(pagesText, mode, pageCount);
  return {
    placing: { rect, sets, mode },
    pageMode: mode,
    pagesText,
    rangeError,
    pageAction: pageActionOf(placementOf(rect, sets, mode), mode, viewedPage, rangeError),
    changePageMode: noop,
    typePages: noop,
    sealViewedPage: noop,
    unsealViewedPage: noop,
  };
}
