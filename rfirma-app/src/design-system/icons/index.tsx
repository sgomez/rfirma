//! El icono que la interfaz pide por su papel; la familia que lo dibuja se elige aquí y en ningún otro sitio.

import { RFIRMA_ICONS } from "./families/rfirma";
import { ICON_SIZES, type IconName } from "./names";

const ACTIVE_FAMILY = RFIRMA_ICONS;

interface IconProps {
  name: IconName;
  /** Lado del cuadro, en px; cada papel trae el suyo. */
  size?: number | string;
  strokeWidth?: number;
}

/** El icono del papel `name`, dibujado por la familia activa. */
export function Icon({ name, size = ICON_SIZES[name], strokeWidth }: IconProps) {
  const { Glyph } = ACTIVE_FAMILY.glyphs[name];
  return <Glyph size={size} strokeWidth={strokeWidth} />;
}
