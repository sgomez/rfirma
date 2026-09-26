import type { RenderResult } from "@testing-library/react";
import { useState } from "react";
import { renderWithCatalog } from "../testing/render";
import {
  activating,
  type PageChoice,
  type PageSet,
  type PageSets,
  pagesOf,
  placementOf,
  storing,
} from "../viewer/signatureBox";
import type { Certificate } from "./certificate";
import type { PreviousSignature, PreviousSignaturesReport } from "./previousSignatures";
import type { Rubric } from "./rubric";
import { SigningPanel } from "./SigningPanel";
import { DEFAULT_VISIBLE_SIGNATURE } from "./visibleSignature";

/** Una firma previa válida, lista para sobreescribir con `overrides`. */
export function previousSignatureOf(overrides: Partial<PreviousSignature> = {}): PreviousSignature {
  return {
    name: "Ada Lovelace Byron",
    idNumber: "99999999R",
    organizationIdentifier: null,
    issuer: "AC FNMT Usuarios",
    certificateSerialNumber: "1",
    signingTime: "2024-01-01T10:00:00Z",
    status: "valid",
    reason: null,
    ...overrides,
  };
}

/** El informe de firmas previas de las pruebas: sin avisos salvo que se digan. */
export function reportOf(
  signatures: readonly PreviousSignature[],
  overrides: Partial<Omit<PreviousSignaturesReport, "signatures">> = {},
): PreviousSignaturesReport {
  return {
    signatures,
    warningCount: 0,
    tone: "information",
    changedAfterLastSignature: false,
    ...overrides,
  };
}

export const certificate: Certificate = {
  id: "0123456789abcdef0123456789abcdef",
  label: "Firma",
  holderName: "Ada Lovelace Byron",
  stampedSigner: "Ada Lovelace Byron",
  givenName: "Ada",
  surname: "Lovelace Byron",
  idNumber: "99999999R",
  organizationIdentifier: null,
  issuer: "AC FNMT Usuarios",
  certificateSerialNumber: "1234567890",
  store: "card",
  status: { kind: "valid", notAfter: 1_894_752_000 },
  remembered: false,
};

/** Un JPEG de un píxel: lo que devuelve `rubric::normalize`, ya opaco. */
export const rubric: Rubric = {
  dataUrl: "data:image/jpeg;base64,/9j/4AAQSkZJRg==",
  width: 240,
  height: 80,
};

const noop = () => {};

/** El recuadro, en espacio de usuario: aquí solo importa que exista. */
export const rect = { x0: 100, y0: 100, x1: 300, y1: 180 };

export type PanelProps = Partial<Parameters<typeof SigningPanel>[0]>;

function panelWith(props: PanelProps) {
  return (
    <SigningPanel
      document={{ id: "doc-1", name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000 }}
      previousSignatures={reportOf([])}
      certificate={{ kind: "chosen", certificate, certificates: [certificate] }}
      onChooseCertificate={noop}
      onRetryCertificates={noop}
      onChooseModule={noop}
      signature={{ ...DEFAULT_VISIBLE_SIGNATURE, enabled: true }}
      onChangeSignature={noop}
      placement={{ rect, pages: { only: [3] } }}
      pageSets={{ single: 3, these: null }}
      onChoosePages={noop}
      pageChoice="single"
      onChangePageChoice={noop}
      viewedPage={3}
      onSeal={noop}
      onUnseal={noop}
      rubric={null}
      rubricFailure={null}
      onChooseRubric={noop}
      destination={{ folder: "Documentos", name: "contrato-firmado.pdf", writable: true }}
      onChangeDestination={noop}
      onSign={noop}
      signing={false}
      failure={null}
      onBack={noop}
      {...props}
    />
  );
}

/** `show` vuelve a pintar con otras props: es el camino de vuelta del ID-99. */
export function renderPanel(
  props: PanelProps = {},
): RenderResult & { show: (next: PanelProps) => void } {
  const result = renderWithCatalog(panelWith(props));
  return { ...result, show: (next: PanelProps) => result.rerender(panelWith(next)) };
}

/**
 * El panel con **las tres opciones de verdad** detrás, que es como vive en
 * `App.tsx` desde el #188.
 *
 * Teclear en el campo son varias pulsaciones seguidas y cada una emite el
 * conjunto: con un espía que no lo aplica, la segunda pulsación escribiría
 * sobre un panel que sigue viendo el conjunto viejo, y lo que se probaría sería
 * el espía. El recuadro es fijo porque el panel ya no lo compone: solo nombra
 * páginas, y quién las convierte en rectángulo es cosa de `App.tsx`.
 */
export function renderLivePanel(props: PanelProps = {}) {
  const chosen: (PageSet | null)[] = [];
  function Live() {
    const choice = props.pageChoice ?? "single";
    const [sets, setSets] = useState<PageSets>({
      single: 3,
      these: choice === "these" ? { only: [3] } : null,
    });
    // La opción elegida también vive fuera del panel, como en `App.tsx`: sin
    // eso, volver a pulsar «Solo 1 página» no dispara nada —el radio sigue
    // marcado— y el viaje de ida y vuelta no se podría probar.
    const [pageChoice, setPageChoice] = useState<PageChoice>(choice);
    return panelWith({
      ...props,
      placement: placementOf(rect, sets, pageChoice),
      pageSets: sets,
      onChoosePages: (next) => {
        chosen.push(next);
        setSets(storing(sets, pageChoice, next, 27));
      },
      pageChoice,
      onChangePageChoice: (next) => {
        setSets(activating(sets, next, pagesOf(sets, pageChoice), 27, 3));
        setPageChoice(next);
      },
    });
  }
  renderWithCatalog(<Live />);
  return { chosen };
}
