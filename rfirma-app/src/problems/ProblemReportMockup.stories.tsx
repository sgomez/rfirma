//! Las historias del mockup de «Informar de un problema» desde el menú: la entrada, el diálogo con su selector y el caso vacío.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { inDialogWindow } from "../../.storybook/decorators/dialogWindow";
import { MenuWithReportEntry, type Reportable, ReportWithPicker } from "./ProblemReportMockup";

const ERRAND_REPORT = `## Trámite
- Sede: portafirmas.villaejemplo.es
- Inicio: 2026-10-27 10:41:07 · Desenlace: falló

## Diario
+0,0 s   llegada            batch
+9,3 s   postfirma          HTTP 500 · 0,4 s
+9,7 s   fallo              batchPostsignerUnreachable · SAF_26`;

const PANIC_REPORT = `## Fallo interno
- Ventana principal · 2026-10-27 09:12:44
- Pánico en src/documents/adapters/files.rs:37:32

## Traza
core::result::unwrap_failed
documents::adapters::files::RealFiles::read`;

const meta = {
  title: "Mockups/Informe de problema",
  parameters: { layout: "centered" },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

export const MenuEntry: Story = {
  name: "1 · Entrada en el menú",
  render: () => <MenuWithReportEntry />,
};

const PLACES = [
  "portafirmas.villaejemplo.es",
  "Ventana principal",
  "sede.villaejemplo.es",
  "registro.villaejemplo.es",
];
const relative = new Intl.RelativeTimeFormat("es", { numeric: "auto" });
const date = new Intl.DateTimeFormat("es", { day: "numeric", month: "short" });
const DAYS = [0, 1, 2, 3, 7, 10, 15].map((ago) =>
  ago < 2 ? relative.format(-ago, "day") : date.format(new Date(2026, 9, 27 - ago)),
);

const MANY: Reportable[] = Array.from({ length: 20 }, (_, index) => {
  const place = PLACES[index % PLACES.length] ?? "";
  return {
    id: String(index),
    day: DAYS[Math.floor(index / 3)] ?? "1 oct",
    time: `${String(18 - (index % 3) * 4).padStart(2, "0")}:${String((index * 7) % 60).padStart(2, "0")}`,
    place,
    report: place === "Ventana principal" ? PANIC_REPORT : ERRAND_REPORT,
  };
});

export const Picker: Story = {
  name: "2 · Informe, con selector",
  decorators: [inDialogWindow],
  render: () => <ReportWithPicker entries={MANY} />,
};

export const PickerOpen: Story = {
  name: "2b · Selector abierto",
  decorators: [inDialogWindow],
  render: () => <ReportWithPicker entries={MANY} defaultOpen />,
};

export const PickerOutdated: Story = {
  name: "2c · Hay una versión nueva",
  decorators: [inDialogWindow],
  render: () => <ReportWithPicker entries={MANY} newerVersion="0.16.1" />,
};

export const PickerEmpty: Story = {
  name: "2d · Nada ha fallado",
  decorators: [inDialogWindow],
  render: () => <ReportWithPicker entries={[]} />,
};
