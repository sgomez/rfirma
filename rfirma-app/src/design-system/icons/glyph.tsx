//! Lo que es una familia de iconos: un dibujo con su origen por cada papel, y el lápiz de contorno con que se dibujan casi todos.

import type { ReactElement, ReactNode } from "react";
import type { IconName } from "./names";
import type { IconOrigin } from "./origins";

interface GlyphProps {
  size: number | string;
  strokeWidth?: number;
}

export interface IconGlyph {
  origin: IconOrigin;
  Glyph: (props: GlyphProps) => ReactElement;
}

export interface IconFamily {
  name: string;
  glyphs: Record<IconName, IconGlyph>;
}

/** Un contorno sin relleno, redondeado, en `currentColor` y sobre lienzo de 24. */
export function stroked(
  origin: IconOrigin,
  drawing: ReactNode,
  defaultStrokeWidth = 1.5,
): IconGlyph {
  return {
    origin,
    Glyph: ({ size, strokeWidth = defaultStrokeWidth }) => (
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth={strokeWidth}
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        focusable="false"
      >
        {drawing}
      </svg>
    ),
  };
}
