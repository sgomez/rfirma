//! El marco de las historias de sede: la ventana de sede a su tamaño real, para que su `position: fixed` no se coma la historia.

import type { Decorator } from "@storybook/react-vite";

/** Encierra la ventana en 520 × 420; el `transform` hace del marco el bloque contenedor de lo fijo. */
export const inSedeWindow: Decorator = (Story) => (
  <div style={{ width: 520, height: 420, position: "relative", transform: "translateZ(0)" }}>
    <Story />
  </div>
);
