//! Las historias del mockup del informe de problema: el menú, el informe con su selector y el informe en la ventana de sede.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { type ReactNode, useState } from "react";
import { fn } from "storybook/test";
import { inDialogWindow } from "../../.storybook/decorators/dialogWindow";
import { SedeEnding } from "../sede/SedeEnding";
import {
  type DatedErrand,
  type JournaledErrand,
  MenuWithReportEntry,
  ReportWithPicker,
  SedeReport,
} from "./ProblemReportMockup";

const ERRANDS: JournaledErrand[] = [
  {
    id: "a",
    when: "Hoy, 10:41",
    site: "portafirmas.villaejemplo.es",
    operation: "Firma de lote · 3 documentos",
    outcome: "failed",
  },
  {
    id: "b",
    when: "Hoy, 09:58",
    site: "portafirmas.villaejemplo.es",
    operation: "Firma de lote · 3 documentos",
    outcome: "abandoned",
  },
  {
    id: "c",
    when: "Ayer, 18:02",
    site: "sede.villaejemplo.es",
    operation: "Firma · PAdES",
    outcome: "interrupted",
  },
  {
    id: "d",
    when: "22 oct, 12:15",
    site: "sede.villaejemplo.es",
    operation: "Selección de certificado",
    outcome: "cancelled",
  },
];

const REPORT = `## Trámite
- Sede: portafirmas.villaejemplo.es
- Inicio: 2026-10-24 10:41:07 · Desenlace: falló
- Protocolo: v4 · WebSocket

## Diario
+0,0 s   llegada            batch
+0,4 s   operación          lote · 3 documentos · PAdES · SHA256withRSA · 1–10 MB
+2,1 s   certificado        DNIe · AC DNIE 004
+6,8 s   prefirma           HTTP 200 · 1,3 s
+9,2 s   firma del hash     3 de 3 · 0,9 s
+9,3 s   postfirma          HTTP 500 · 0,4 s
+9,7 s   fallo              batchPostsignerUnreachable · SAF_26

## Puente
AOException
  es.gob.afirma.signers.batch.client.BatchSigner.sign:312
  es.gob.afirma.nativebridge.NativeBridge.batchPostsign:540

## Entorno
- rFirma 0.9.3 · flatpak (stable, flathub) · x86_64
- Ubuntu 24.04 · GNOME · Wayland · es_ES
- WebKitGTK 2.48.3 · librería nativa: cargada
- PC/SC: 1 lector · DNIe 4.0 · rdnie
- Canal local: confiado por Firefox`;

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

const previewActions = { onBack: fn(), onSave: fn(), onOpenIssue: fn() };

function SedeFrame({
  width,
  height,
  children,
}: {
  width: number;
  height: number;
  children: ReactNode;
}) {
  return (
    <div
      className="sede-window"
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
      {children}
    </div>
  );
}

const sedePreview = {
  errand: ERRANDS[0] as JournaledErrand,
  report: REPORT,
  fitsInLink: true,
  newerVersion: null,
  ...previewActions,
};

export const SedeGrown: Story = {
  name: "3 · Ventana de sede: el informe",
  render: () => (
    <SedeFrame width={760} height={660}>
      <SedeReport {...sedePreview} />
    </SedeFrame>
  ),
};

function SedeJourney() {
  const [reporting, setReporting] = useState(false);
  return reporting ? (
    <SedeFrame width={760} height={660}>
      <SedeReport {...sedePreview} onBack={() => setReporting(false)} />
    </SedeFrame>
  ) : (
    <SedeFrame width={520} height={420}>
      <SedeEnding
        origin="portafirmas.villaejemplo.es"
        outcome={{
          kind: "refused",
          situation: "batchPostsignerUnreachable",
          detail: "SAF_26: no se pudo contactar con el servicio de lote",
        }}
        onClose={fn()}
        onReport={() => setReporting(true)}
      />
    </SedeFrame>
  );
}

export const SedeWholeJourney: Story = {
  name: "4 · Ventana de sede: recorrido entero",
  render: () => <SedeJourney />,
};

const SITES = ["portafirmas.villaejemplo.es", "sede.villaejemplo.es", "registro.villaejemplo.es"];
const OPERATIONS = ["Firma de lote · 3 documentos", "Firma · PAdES", "Selección de certificado"];
const OUTCOMES: JournaledErrand["outcome"][] = ["failed", "abandoned", "interrupted", "cancelled"];
const relative = new Intl.RelativeTimeFormat("es", { numeric: "auto" });
const date = new Intl.DateTimeFormat("es", { day: "numeric", month: "short" });
const DAYS = [0, 1, 2, 3, 7, 10, 15].map((ago) =>
  ago < 2 ? relative.format(-ago, "day") : date.format(new Date(2026, 9, 24 - ago)),
);

const MANY: DatedErrand[] = Array.from({ length: 20 }, (_, index) => ({
  id: String(index),
  day: DAYS[Math.floor(index / 3)] ?? "1 oct",
  time: `${String(18 - (index % 3) * 4).padStart(2, "0")}:${String((index * 7) % 60).padStart(2, "0")}`,
  site: SITES[index % 3] ?? "",
  operation: OPERATIONS[index % 3] ?? "",
  outcome: OUTCOMES[index % 4] ?? "failed",
  when: "",
}));

export const Picker: Story = {
  name: "2 · Informe, con selector de trámite",
  decorators: [inDialogWindow],
  render: () => <ReportWithPicker errands={MANY} report={REPORT} />,
};

export const PickerOpen: Story = {
  name: "2b · Selector abierto, 20 trámites",
  decorators: [inDialogWindow],
  render: () => <ReportWithPicker errands={MANY} report={REPORT} defaultOpen />,
};

export const PickerTooLongAndOutdated: Story = {
  name: "2c · No cabe y hay versión nueva",
  decorators: [inDialogWindow],
  render: () => (
    <ReportWithPicker errands={MANY} report={REPORT} fitsInLink={false} newerVersion="0.9.4" />
  ),
};

export const PickerEmpty: Story = {
  name: "2d · Ningún trámite fallido",
  decorators: [inDialogWindow],
  render: () => <ReportWithPicker errands={[]} report={REPORT} />,
};
