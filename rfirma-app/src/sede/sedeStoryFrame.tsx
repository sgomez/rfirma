//! El marco de las historias de sede: la ventana de sede a su tamaño real, delimitada como una ventana.

import type { Decorator } from "@storybook/react-vite";

/** 520 × 420 con borde y sombra; el `transform` hace del marco el bloque contenedor de su `position: fixed`. */
export const inSedeWindow: Decorator = (Story) => (
  <div
    style={{
      width: 520,
      height: 420,
      position: "relative",
      transform: "translateZ(0)",
      overflow: "hidden",
      border: "1px solid var(--rf-border-subtle)",
      borderRadius: "var(--rf-radius-lg)",
      boxShadow: "var(--rf-shadow-elevated)",
    }}
  >
    <Story />
  </div>
);
