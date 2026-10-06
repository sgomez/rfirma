//! El marco de las historias de los paneles de firma: la columna de 380 px a su alto de ventana, con borde y sombra, y las órdenes espía.

import type { Decorator } from "@storybook/react-vite";
import { fn } from "storybook/test";
import type { Destination } from "../../src/signing/destination";

/** 380 × 608 —la zona que se desliza de 446 px y el pie de 162— con el `transform` que hace del marco el bloque contenedor de lo que flota. */
export const inPanelColumn: Decorator = (Story) => (
  <div
    style={{
      width: 380,
      height: 608,
      position: "relative",
      transform: "translateZ(0)",
      overflow: "hidden",
      border: "1px solid var(--rf-border-subtle)",
      borderRadius: "var(--rf-radius-lg)",
      boxShadow: "var(--rf-shadow-elevated)",
      background: "var(--rf-bg)",
    }}
  >
    <Story />
  </div>
);

export const panelStoryParameters = {
  layout: "centered",
  designSync: { cardMode: "column" },
} as const;

export const WRITABLE_DESTINATION: Destination = {
  folder: "Documentos",
  name: "contrato-firmado.pdf",
  writable: true,
};

export const UNWRITABLE_DESTINATION: Destination = {
  folder: "Documentos",
  name: null,
  writable: false,
};

export const panelActions = {
  onSign: fn(),
  onOpenHelp: fn(),
};
