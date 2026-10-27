//! Las historias del mockup del informe de un fallo interno: el pánico en la ventana principal, el fallo al pintar y el pánico a mitad de un trámite de sede.

import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import {
  MainWindowPanic,
  RenderFailure,
  SedeFailure,
  SedeReportSend,
  WindowFrame,
} from "./ProblemReportFailures";

const ENVIRONMENT = `## Entorno
- rFirma 0.16.0 · dmg · aarch64
- macOS 27.0.1 · es_ES
- WebKit 619 · librería nativa: cargada
- PC/SC: 1 lector · DNIe 4.0 · rdnie`;

const PANIC_REPORT = `## Fallo interno
- Ventana principal · 2026-10-27 09:12:44
- Pánico en src/documents/adapters/files.rs:37:32

## Traza
core::result::unwrap_failed
documents::adapters::files::RealFiles::read
documents::application::documents::bytes_of
documents::adapters::tauri::read_document

${ENVIRONMENT}`;

const RENDER_REPORT = `## Fallo al pintar
- Ventana principal · 2026-10-27 09:15:02
- TypeError en DocumentTabs

## Componentes
App › MainWindow › Header › DocumentTabs

## Traza
DocumentTabs.tsx:88:19
Header.tsx:31:7
MainWindow.tsx:30:5

${ENVIRONMENT}`;

const SEDE_PANIC_REPORT = `## Trámite
- Sede: portafirmas.villaejemplo.es
- Inicio: 2026-10-27 10:41:07 · Desenlace: falló
- Protocolo: v4 · WebSocket

## Diario
+0,0 s   llegada            batch
+0,4 s   operación          lote · 3 documentos · PAdES · SHA256withRSA · 1–10 MB
+2,1 s   certificado        DNIe · AC DNIE 004
+6,8 s   prefirma           HTTP 200 · 1,3 s
+7,0 s   fallo interno      site/application/errand/batch.rs:212:18

## Traza
core::option::unwrap_failed
site::application::errand::batch::presign_each
site::application::errand::batch::run
site::adapters::tauri::sign_batch

${ENVIRONMENT}`;

const SEDE_UNREACHABLE_REPORT = `## Trámite
- Sede: portafirmas.villaejemplo.es
- Inicio: 2026-10-27 10:41:07 · Desenlace: falló
- Protocolo: v4 · WebSocket

## Diario
+0,0 s   llegada            batch
+0,4 s   operación          lote · 3 documentos · PAdES · SHA256withRSA · 1–10 MB
+2,1 s   certificado        DNIe · AC DNIE 004
+6,8 s   prefirma           HTTP 200 · 1,3 s
+9,2 s   firma del hash     3 de 3 · 0,9 s
+9,3 s   postfirma          HTTP 500 · 0,4 s
+9,7 s   fallo              batchPostsignerUnreachable · SAF_26

${ENVIRONMENT}`;

const SEDE_SHA1_REPORT = `## Trámite
- Sede: sede.villaejemplo.es
- Inicio: 2026-10-27 12:03:51 · Desenlace: rechazado
- Protocolo: v3 · servlet

## Diario
+0,0 s   llegada            sign
+0,2 s   operación          firma · CAdES · SHA1withRSA · 10–100 KB
+0,2 s   rechazo            sha1 · SAF_03

${ENVIRONMENT}`;

const meta = {
  title: "Mockups/Informe de problema · fallos internos",
  parameters: { layout: "centered" },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

export const MainPanic: Story = {
  name: "5 · Ventana principal: pánico, recorrido",
  render: () => (
    <WindowFrame width={900} height={640}>
      <MainWindowPanic report={PANIC_REPORT} />
    </WindowFrame>
  ),
};

export const MainPanicSent: Story = {
  name: "5b · Ventana principal: enviado",
  render: () => (
    <WindowFrame width={900} height={640}>
      <MainWindowPanic report={PANIC_REPORT} initial="sent" />
    </WindowFrame>
  ),
};

export const MainPanicUnreachable: Story = {
  name: "5c · Ventana principal: GlitchTip no contesta",
  render: () => (
    <WindowFrame width={900} height={640}>
      <MainWindowPanic report={PANIC_REPORT} reachable={false} />
    </WindowFrame>
  ),
};

export const Render: Story = {
  name: "6 · Fallo al pintar: recorrido",
  render: () => (
    <WindowFrame width={900} height={640}>
      <RenderFailure report={RENDER_REPORT} />
    </WindowFrame>
  ),
};

const SEDE_CASES = {
  internal: {
    sentence: "Es un fallo de rFirma. Vuelve a la sede e inténtalo de nuevo.",
    report: SEDE_PANIC_REPORT,
  },
  unreachable: {
    sentence: "La sede no ha respondido. Vuelve a la sede e inténtalo de nuevo.",
    report: SEDE_UNREACHABLE_REPORT,
  },
  sha1: {
    sentence:
      "La sede ha pedido una firma con SHA-1, que ya no es segura. Contacta con la sede para terminar el trámite.",
    report: SEDE_SHA1_REPORT,
  },
};

function SedeJourney({
  which,
  reachable = true,
}: {
  which: keyof typeof SEDE_CASES;
  reachable?: boolean;
}) {
  const [reporting, setReporting] = useState(false);
  const { sentence, report } = SEDE_CASES[which];
  return reporting ? (
    <WindowFrame width={760} height={660}>
      <div className="sede-window" style={{ height: "100%" }}>
        <SedeReportSend
          site="portafirmas.villaejemplo.es"
          report={report}
          reachable={reachable}
          onBack={() => setReporting(false)}
        />
      </div>
    </WindowFrame>
  ) : (
    <WindowFrame width={520} height={420}>
      <div className="sede-window" style={{ height: "100%" }}>
        <SedeFailure
          site="portafirmas.villaejemplo.es"
          title="No se ha completado la petición"
          sentence={sentence}
          onReport={() => setReporting(true)}
        />
      </div>
    </WindowFrame>
  );
}

export const SedePanic: Story = {
  name: "7 · Ventana de sede: fallo de rFirma",
  render: () => <SedeJourney which="internal" />,
};

export const SedePanicUnreachable: Story = {
  name: "7b · Ventana de sede: GlitchTip no contesta",
  render: () => <SedeJourney which="internal" reachable={false} />,
};

export const SedeUnreachable: Story = {
  name: "8 · Ventana de sede: la sede no responde",
  render: () => <SedeJourney which="unreachable" />,
};

export const SedeSha1: Story = {
  name: "9 · Ventana de sede: rFirma rechaza la petición",
  render: () => <SedeJourney which="sha1" />,
};
