//! El marco de las historias de diálogos: la ventana principal a su tamaño, para que el velo `position: fixed` tenga dónde pintarse.

import type { Decorator } from "@storybook/react-vite";

/** 1280 × 720, el tamaño inicial de la ventana principal, con borde y sombra; el `transform` hace del marco el bloque contenedor del velo. */
export const inDialogWindow: Decorator = (Story) => (
  <div
    style={{
      width: 1280,
      height: 720,
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
