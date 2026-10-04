//! El marco de las historias del panel de estado y de la retirada: la ventana de 1180 × 700 con su borde.

import type { Decorator } from "@storybook/react-vite";

/** El `transform` hace del marco el bloque contenedor del `position: fixed` del velo de la retirada. */
export const inStatusWindow: Decorator = (Story) => (
  <div
    style={{
      width: 1180,
      height: 700,
      position: "relative",
      transform: "translateZ(0)",
      overflow: "hidden",
      display: "flex",
      flexDirection: "column",
      border: "1px solid var(--rf-border-subtle)",
      borderRadius: "var(--rf-radius-lg)",
      boxShadow: "var(--rf-shadow-elevated)",
    }}
  >
    <Story />
  </div>
);
