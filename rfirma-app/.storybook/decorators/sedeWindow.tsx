//! El marco de las historias de sede: la ventana a su tamaño real, delimitada como una ventana, y la configuración común de sus historias.

import type { Decorator } from "@storybook/react-vite";
import { SedeView } from "../../src/sede/SedeView";
import { sedeViewActions } from "../../src/sede/testing/fixtures/sedeView";

/** La ventana con borde y sombra; el `transform` hace del marco el bloque contenedor de su `position: fixed`. */
const inSedeWindow =
  (width: number, height: number): Decorator =>
  (Story) => (
    <div
      style={{
        width,
        height,
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
  decorators: [inSedeWindow(520, 420)],
  parameters: { layout: "centered" },
  args: { ...sedeViewActions, consentCountdown: false },
};

/** El mismo marco a 1080 × 660, el tamaño al que crece la ventana mientras se marca el área. */
export const sedeAreaStoryMeta = { ...sedeStoryMeta, decorators: [inSedeWindow(1080, 660)] };

/** Un momento desnudo: el fondo de superficie de la ventana, sin marco, borde ni sombra. */
const onSedeSurface: Decorator = (Story) => (
  <div className="sede-window" style={{ width: 520, height: 420 }}>
    <Story />
  </div>
);

/** Lo que toda historia de un momento de sede comparte: el fondo de superficie y el centrado. */
export const sedeMomentMeta = {
  decorators: [onSedeSurface],
  parameters: { layout: "centered" },
};
