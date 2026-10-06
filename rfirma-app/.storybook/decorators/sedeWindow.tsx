//! Los marcos de las historias de sede: la ventana completa, solo para la historia de página, y la superficie de un momento desnudo.

import type { Decorator } from "@storybook/react-vite";

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

/** Lo que comparte la historia de página: el marco a 520 × 420 y el centrado. */
export const sedeWindowMeta = {
  decorators: [inSedeWindow(520, 420)],
  parameters: { layout: "centered" },
};

/** El mismo marco a 1080 × 660, el tamaño al que crece la ventana mientras se marca el área. */
export const sedeAreaWindowMeta = { ...sedeWindowMeta, decorators: [inSedeWindow(1080, 660)] };

/** Un momento desnudo: el fondo de superficie de la ventana, sin marco, borde ni sombra. */
const onSedeSurface =
  (width: number, height: number): Decorator =>
  (Story) => (
    <div className="sede-window" style={{ width, height }}>
      <Story />
    </div>
  );

/** Lo que toda historia de un momento de sede comparte: el fondo de superficie y el centrado. */
export const sedeMomentMeta = {
  decorators: [onSedeSurface(520, 420)],
  parameters: { layout: "centered", designSync: { cardMode: "column" } },
};

/** El momento de marcar el área, que ocupa la ventana ampliada. */
export const sedeAreaMomentMeta = { ...sedeMomentMeta, decorators: [onSedeSurface(1080, 660)] };
