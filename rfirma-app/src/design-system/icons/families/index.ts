//! Las familias de iconos candidatas, en el orden en que el catálogo las compara.

import type { IconFamily } from "../glyph";
import { HEROICONS_ICONS } from "./heroicons";
import { RFIRMA_ICONS } from "./rfirma";

export const ICON_FAMILIES: readonly IconFamily[] = [RFIRMA_ICONS, HEROICONS_ICONS];
