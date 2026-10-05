import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { aCertificate, document } from "../App.testSupport";
import { usePlacement } from "../placement/usePlacement";
import { inMemoryDestination, type SignedDocumentOpener } from "../signing/destination";
import type { SigningBackend, SigningOrder, StageResult } from "../signing/flow";
import { NO_PREVIOUS_SIGNATURES } from "../signing/previousSignatures";
import { unavailableStampComposer } from "../signing/stampPreview";
import type { CertificateListing } from "../signing/useCertificateListing";
import type { VisibleSignature } from "../signing/visibleSignature";
import { CatalogProvider } from "../testing/render";
import { recordingDocument, seated } from "../viewer/testing/documentViewerFixtures";
import { useSigningJourney } from "./useSigningJourney";

const withCatalog = ({ children }: { children: ReactNode }) => (
  <CatalogProvider language="es">{children}</CatalogProvider>
);

const visible: VisibleSignature = {
  enabled: true,
  withRubric: false,
  content: { model: "complete" },
};

function gate<T>() {
  let open: (value: T) => void = () => {};
  const promise = new Promise<T>((resolve) => {
    open = resolve;
  });
  return { promise, open };
}

describe("useSigningJourney", () => {
  it("signs the open document once, from the order to the acknowledgement", async () => {
    const calls: string[] = [];
    const signGate = gate<StageResult<void>>();
    const presigned: SigningOrder[] = [];
    const signer: SigningBackend = {
      presign: async (order) => {
        calls.push("presign");
        presigned.push(order);
        return { ok: true, value: { kind: "typedOnScreen" } };
      },
      sign: () => {
        calls.push("sign");
        return signGate.promise;
      },
      postsign: async () => {
        calls.push("postsign");
        return {
          ok: true,
          value: { name: "contrato-firmado.pdf", folder: "Documentos", sizeBytes: 1200 },
        };
      },
      padesLowerLeft: async (placement) => [placement.rect[0], placement.rect[1]],
      previousSignatures: async () => NO_PREVIOUS_SIGNATURES,
      signedDocumentSignatures: async () => NO_PREVIOUS_SIGNATURES,
      discard: async () => {},
    };
    const opener: SignedDocumentOpener = {
      openDocument: vi.fn(async () => {}),
      openFolder: vi.fn(async () => {}),
    };
    const ports = {
      signer,
      stamps: unavailableStampComposer(),
      destinations: inMemoryDestination({
        folder: "Documentos",
        name: "contrato-firmado.pdf",
        writable: true,
      }),
      opener,
    };
    const active = document("contrato.pdf", { placement: seated });
    const pdf = recordingDocument(3).document;
    const opening = { placement: seated, pageCount: 3 };
    const listing: CertificateListing = {
      kind: "listed",
      certificates: [{ ...aCertificate, remembered: true }],
    };
    const certificates = { listing, lookAgain: async () => {} };
    const reopenDocument = vi.fn();

    const { result } = renderHook(
      () => {
        const placement = usePlacement({ document: opening, standardRectOn: null });
        return useSigningJourney({
          ports,
          document: { active, pdf, sizeBytes: 2_400, opening },
          placement,
          certificates,
          rubric: null,
          settingsFolder: "Documentos",
          initialSignature: visible,
          reopenDocument,
        });
      },
      { wrapper: withCatalog },
    );

    expect(result.current.certificate.state.kind).toBe("chosen");
    await waitFor(() => expect(result.current.destination.value.name).toBe("contrato-firmado.pdf"));
    expect(result.current.acknowledgement).toBeNull();

    await act(async () => {
      void result.current.signing.sign();
    });
    await waitFor(() => expect(calls).toEqual(["presign", "sign"]));
    expect(result.current.signals).toEqual({ signing: true, dialogOpen: true });
    expect(result.current.dialogs.stage).toBe("sign");

    await act(async () => {
      signGate.open({ ok: true, value: undefined });
    });

    await waitFor(() => expect(result.current.acknowledgement).not.toBeNull());
    expect(calls).toEqual(["presign", "sign", "postsign"]);
    expect(presigned[0]).toMatchObject({
      document: active.id,
      certificate: aCertificate.id,
      placement: { page: 1, pageCount: 3, rect: [50, 60, 250, 140] },
      language: "es",
      allowUnregisteredSignatures: false,
    });
    expect(result.current.acknowledgement?.documentName).toBe("contrato-firmado.pdf");
    expect(result.current.signals).toEqual({ signing: false, dialogOpen: false });
    expect(result.current.failure).toBeNull();

    act(() => result.current.acknowledgement?.openFolder());
    expect(opener.openFolder).toHaveBeenCalledOnce();

    act(() => result.current.acknowledgement?.signAgain());
    expect(result.current.acknowledgement).toBeNull();
    expect(reopenDocument).toHaveBeenCalledOnce();
  });
});
