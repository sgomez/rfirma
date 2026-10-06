//! Los dobles y ayudas que comparten las pruebas de `SigningPanel`.

import type { RenderResult } from "@testing-library/react";
import { screen } from "@testing-library/react";
import { expect } from "vitest";
import { placementStateOf } from "../placement/placementFixtures";
import { renderWithCatalog } from "../testing/render";
import type { Certificate } from "./certificate";
import type { PreviousSignature, PreviousSignaturesReport } from "./previousSignatures";
import type { Rubric } from "./rubric";
import { SigningPanel } from "./SigningPanel";
import {
  aCertificateSection,
  aDestinationSection,
  aRubricSection,
  aSigningSection,
  aVisibleSignatureSection,
} from "./signingSectionFixtures";
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
    validity: "valid",
    validityReason: null,
    signingDate: null,
    closesDocument: false,
    countersignatures: [],
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
    findings: [],
    ...overrides,
  };
}

export { aCertificateSection, aRubricSection, aSigningSection, aVisibleSignatureSection };

export const certificate: Certificate = {
  id: "0123456789abcdef0123456789abcdef",
  label: "Firma",
  holderName: "Ada Lovelace Byron",
  stampedSigner: "Ada Lovelace Byron",
  givenName: "Ada",
  surname: "Lovelace Byron",
  idNumber: "99999999R",
  organizationIdentifier: null,
  entityName: null,
  issuer: "AC FNMT Usuarios",
  certificateSerialNumber: "1234567890",
  stores: ["card"],
  status: { kind: "valid", notAfter: 1_894_752_000 },
  remembered: false,
};

/** Un JPEG de un píxel: lo que devuelve `rubric::normalize`, ya opaco. */
export const rubric: Rubric = {
  dataUrl: "data:image/jpeg;base64,/9j/4AAQSkZJRg==",
  width: 240,
  height: 80,
};

const rect = { x0: 100, y0: 100, x1: 300, y1: 180 };

export type PanelProps = Partial<Parameters<typeof SigningPanel>[0]>;

function panelWith(props: PanelProps) {
  return (
    <SigningPanel
      document={{ id: "doc-1", name: "contrato.pdf", sizeBytes: 2_400_000 }}
      previousSignatures={reportOf([])}
      certificate={aCertificateSection({
        kind: "chosen",
        certificate,
        certificates: [certificate],
      })}
      signature={aVisibleSignatureSection({ ...DEFAULT_VISIBLE_SIGNATURE, enabled: true })}
      placementState={placementStateOf({
        rect,
        sets: { single: 3, these: null },
        viewedPage: 3,
        pageCount: 27,
      })}
      rubric={aRubricSection()}
      destination={aDestinationSection({
        folder: "Documentos",
        name: "contrato-firmado.pdf",
        writable: true,
      })}
      signing={aSigningSection()}
      failure={null}
      {...props}
    />
  );
}

/** `show` vuelve a pintar con otras props. */
export function renderPanel(
  props: PanelProps = {},
): RenderResult & { show: (next: PanelProps) => void } {
  const result = renderWithCatalog(panelWith(props));
  return { ...result, show: (next: PanelProps) => result.rerender(panelWith(next)) };
}

/** La línea del aviso de firmas previas: el texto entero y la coletilla en negrita aparte. */
export function expectNoticeLine(lead: string, problems: string) {
  expect(document.querySelector(".panel__co-signature-text")).toHaveTextContent(
    `${lead} · ${problems}`,
  );
  expect(screen.getByText(problems).tagName).toBe("STRONG");
}
