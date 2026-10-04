//! El marco de las historias de sede: la ventana a su tamaño real, delimitada como una ventana, y la configuración común de sus historias.

import type { Decorator } from "@storybook/react-vite";
import { SedeView } from "./SedeView";
import { sedeViewActions } from "./sedeStoryPort";

/** 520 × 420 con borde y sombra; el `transform` hace del marco el bloque contenedor de su `position: fixed`. */
const inSedeWindow: Decorator = (Story) => (
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

/** Lo que toda historia de sede comparte: el componente, el marco, el centrado y las órdenes espía. */
export const sedeStoryMeta = {
  component: SedeView,
  decorators: [inSedeWindow],
  parameters: { layout: "centered" },
  args: { ...sedeViewActions, consentCountdown: false },
};
