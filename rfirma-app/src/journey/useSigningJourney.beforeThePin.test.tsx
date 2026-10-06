import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it } from "vitest";
import type { DocumentInHand } from "../documents/document";
import type { Placement } from "../placement/pageSets";
import { usePlacement } from "../placement/usePlacement";
import { inMemoryDestination } from "../signing/destination";
import type { SigningBackend, SigningOrder } from "../signing/flow";
import {
  NO_PREVIOUS_SIGNATURES,
  type PreviousSignature,
  type PreviousSignaturesReport,
} from "../signing/previousSignatures";
import { unavailableStampComposer } from "../signing/stampPreview";
import type { CertificateListing } from "../signing/useCertificateListing";
import { aCertificate, aPdfWithViews, document } from "../testing/harness";
import { CatalogProvider } from "../testing/render";
import { useSigningJourney } from "./useSigningJourney";

const A4: readonly [number, number, number, number] = [0, 0, 595, 842];
const SMALL: readonly [number, number, number, number] = [0, 0, 200, 150];
const rect = { x0: 250, y0: 50, x1: 450, y1: 100 };

const withCatalog = ({ children }: { children: ReactNode }) => (
  <CatalogProvider language="es">{children}</CatalogProvider>
);

function aSignature(overrides: Partial<PreviousSignature>): PreviousSignature {
  return {
    name: "Bruce Wayne",
    idNumber: "00000000T",
    organizationIdentifier: null,
    issuer: "AC FNMT Usuarios",
    certificateSerialNumber: "1",
    signingTime: "2024-01-01T10:00:00.000Z",
    validity: "valid",
    validityReason: null,
    signingDate: null,
    closesDocument: false,
    countersignatures: [],
    ...overrides,
  };
}

const validSignature = aSignature({ name: "Alfred Pennyworth" });
const expiredSignature = aSignature({
  validity: "expired",
  validityReason: { kind: "certificateExpired", date: "2020-03-05T12:00:00Z", holder: null },
});
const unknownTypeSignature = aSignature({
  name: "Notaría XYZ",
  validity: "invalid",
  validityReason: { kind: "unknownSignatureType" },
});

interface Scenario {
  report?: Partial<PreviousSignaturesReport>;
  views?: readonly (readonly [number, number, number, number])[];
  pages?: Placement["pages"];
  active?: DocumentInHand;
}

async function journeyAbout({
  report,
  views = [A4, A4, A4],
  pages = { only: [1] },
  active,
}: Scenario) {
  const presigned: SigningOrder[] = [];
  const signer: SigningBackend = {
    presign: async (order) => {
      presigned.push(order);
      return { ok: true, value: { kind: "typedOnScreen" } };
    },
    sign: async () => ({ ok: true, value: undefined }),
    postsign: async () => ({
      ok: true,
      value: { name: "factura-firmado.pdf", folder: "Documentos", sizeBytes: 1 },
    }),
    padesLowerLeft: async (placement) => [placement.rect[0], placement.rect[1]],
    previousSignatures: async () => ({ ...NO_PREVIOUS_SIGNATURES, ...report }),
    signedDocumentSignatures: async () => NO_PREVIOUS_SIGNATURES,
    discard: async () => {},
  };
  const placement: Placement = { rect, pages };
  const openDocument = active ?? document("factura.pdf", { placement });
  const pdf = aPdfWithViews(views);
  const opening = { placement, pageCount: views.length };
  const listing: CertificateListing = {
    kind: "listed",
    certificates: [{ ...aCertificate, remembered: true }],
  };

  const ports = {
    signer,
    stamps: unavailableStampComposer(),
    destinations: inMemoryDestination({
      folder: "Documentos",
      name: "factura-firmado.pdf",
      writable: true,
    }),
    opener: { openDocument: async () => {}, openFolder: async () => {} },
  };
  const certificates = { listing, lookAgain: async () => {}, install: async () => false };

  const { result } = renderHook(
    () => {
      const placementState = usePlacement({ document: opening, standardRectOn: null });
      return useSigningJourney({
        ports,
        document: { active: openDocument, pdf, sizeBytes: 2_400, opening },
        placement: placementState,
        certificates,
        rubric: null,
        settingsFolder: "Documentos",
        initialSignature: { enabled: true, withRubric: false, content: { model: "complete" } },
        reopenDocument: () => {},
      });
    },
    { wrapper: withCatalog },
  );

  await waitFor(() => expect(result.current.certificate.state.kind).toBe("chosen"));
  if (report !== undefined) {
    await waitFor(() => expect(result.current.previousSignatures).not.toBe(NO_PREVIOUS_SIGNATURES));
  }
  const sign = () => act(async () => void (await result.current.signing.sign()));
  return { result, presigned, sign };
}

describe("useSigningJourney, antes del PIN: páginas sin sello", () => {
  const threePages: Scenario = { views: [A4, SMALL, A4], pages: { only: [1, 2, 3] } };

  it("opens the notice when a chosen page will fall, and cancelling it signs nothing", async () => {
    const { result, presigned, sign } = await journeyAbout(threePages);

    await sign();

    expect(result.current.dialogs.sealLoss?.fallen).toBe(1);
    expect(result.current.signals.dialogOpen).toBe(true);
    expect(presigned).toHaveLength(0);

    act(() => result.current.dialogs.sealLoss?.cancel());

    expect(result.current.dialogs.sealLoss).toBeNull();
    expect(result.current.signals.dialogOpen).toBe(false);
    expect(presigned).toHaveLength(0);
  });

  it("signs anyway with the exact order already built, when confirmed", async () => {
    const { result, presigned, sign } = await journeyAbout(threePages);
    await sign();

    await act(async () => void (await result.current.dialogs.sealLoss?.confirm()));

    await waitFor(() => expect(presigned).toHaveLength(1));
    expect(presigned[0]?.placement?.pages).toEqual({ only: [1, 2, 3] });
    expect(result.current.dialogs.sealLoss).toBeNull();
  });

  it("does not open when every chosen page keeps its visible signature", async () => {
    const { result, presigned, sign } = await journeyAbout({
      views: [A4, SMALL, A4],
      pages: { only: [1, 3] },
    });

    await sign();

    await waitFor(() => expect(presigned).toHaveLength(1));
    expect(result.current.dialogs.sealLoss).toBeNull();
  });
});

describe("useSigningJourney, antes del PIN: ¿Firmar de todos modos?", () => {
  it("opens for an expired signature, with its row, and does not sign", async () => {
    const { result, presigned, sign } = await journeyAbout({
      report: { tone: "attention", signatures: [expiredSignature] },
    });

    await sign();

    expect(result.current.dialogs.signAnyway?.problems).toEqual([
      { kind: "signature", number: 1, signature: expiredSignature },
    ]);
    expect(presigned).toHaveLength(0);
  });

  it("opens for a finding alone, as a row without a signature", async () => {
    const { result, sign } = await journeyAbout({
      report: {
        tone: "attention",
        signatures: [validSignature],
        findings: ["modifiedAfterLastSignature"],
      },
    });

    await sign();

    expect(result.current.dialogs.signAnyway?.problems).toEqual([
      { kind: "finding", finding: "modifiedAfterLastSignature" },
    ]);
  });

  it("lists one row per problem, findings first, and leaves the valid signatures out", async () => {
    const { result, sign } = await journeyAbout({
      report: {
        tone: "attention",
        signatures: [validSignature, expiredSignature, unknownTypeSignature],
        findings: ["contentAddedOnTop"],
      },
    });

    await sign();

    expect(result.current.dialogs.signAnyway?.problems).toEqual([
      { kind: "finding", finding: "contentAddedOnTop" },
      { kind: "signature", number: 2, signature: expiredSignature },
      { kind: "signature", number: 3, signature: unknownTypeSignature },
    ]);
  });

  it("shows a signature of an unknown type as one more row, with no notice of its own", async () => {
    const { result, sign } = await journeyAbout({
      report: { tone: "attention", signatures: [unknownTypeSignature] },
    });

    await sign();

    expect(result.current.dialogs.signAnyway?.problems).toEqual([
      { kind: "signature", number: 1, signature: unknownTypeSignature },
    ]);
  });

  it("lets the bridge cosign once the unknown-type row is accepted", async () => {
    const { result, presigned, sign } = await journeyAbout({
      report: { tone: "attention", signatures: [unknownTypeSignature] },
    });
    await sign();

    await act(async () => void (await result.current.dialogs.signAnyway?.confirm()));

    await waitFor(() => expect(presigned).toHaveLength(1));
    expect(presigned[0]?.allowUnregisteredSignatures).toBe(true);
    expect(result.current.dialogs.signAnyway).toBeNull();
  });

  it("signs nothing when it is cancelled", async () => {
    const { result, presigned, sign } = await journeyAbout({
      report: { tone: "attention", signatures: [expiredSignature] },
    });
    await sign();

    act(() => result.current.dialogs.signAnyway?.cancel());

    expect(result.current.dialogs.signAnyway).toBeNull();
    expect(result.current.signals.dialogOpen).toBe(false);
    expect(presigned).toHaveLength(0);
  });

  it("signs with the exact order already built, when confirmed", async () => {
    const { result, presigned, sign } = await journeyAbout({
      report: { tone: "attention", signatures: [expiredSignature] },
    });
    await sign();

    await act(async () => void (await result.current.dialogs.signAnyway?.confirm()));

    await waitFor(() => expect(presigned).toHaveLength(1));
    expect(presigned[0]).toMatchObject({
      document: "id-factura.pdf",
      certificate: aCertificate.id,
      placement: { page: 1, pageCount: 3 },
    });
    expect(presigned[0]?.allowUnregisteredSignatures).toBeFalsy();
    expect(result.current.dialogs.signAnyway).toBeNull();
  });

  it("signs directly when every signature is valid", async () => {
    const { result, presigned, sign } = await journeyAbout({
      report: { signatures: [validSignature, aSignature({ name: "Selene" })] },
    });

    await sign();

    await waitFor(() => expect(presigned).toHaveLength(1));
    expect(result.current.dialogs.signAnyway).toBeNull();
  });
});

describe("useSigningJourney, con un documento que no se recuerda", () => {
  it("signs it like any other, with its own id in the order", async () => {
    const { presigned, sign } = await journeyAbout({
      active: document("de-la-sede.pdf", {
        remembered: false,
        placement: { rect, pages: { only: [1] } },
      }),
    });

    await sign();

    await waitFor(() => expect(presigned).toHaveLength(1));
    expect(presigned[0]?.document).toBe("id-de-la-sede.pdf");
  });
});
