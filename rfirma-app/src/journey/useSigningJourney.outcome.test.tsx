import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import type { DocumentInHand } from "../documents/document";
import { usePlacement } from "../placement/usePlacement";
import { inMemoryDestination, type SignedDocumentOpener } from "../signing/destination";
import type { SigningBackend, SigningOrder, StageResult } from "../signing/flow";
import { NO_PREVIOUS_SIGNATURES, type PreviousSignature } from "../signing/previousSignatures";
import { unavailableStampComposer } from "../signing/stampPreview";
import type { TokenFailure } from "../signing/token";
import type { VisibleSignature } from "../signing/visibleSignature";
import { aCertificate, document } from "../testing/harness";
import { CatalogProvider } from "../testing/render";
import { recordingDocument, seated } from "../viewer/testing/fixtures";
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

const tokenAbsent: StageResult<never> = {
  ok: false,
  failure: { situation: "tokenAbsent", detail: "CKR_DEVICE_REMOVED (C_Sign)", attemptsLeft: null },
};

function aSigner(overrides: Partial<SigningBackend> = {}): SigningBackend {
  return {
    presign: async () => ({ ok: true, value: { kind: "typedOnScreen" } }),
    sign: async () => ({ ok: true, value: undefined }),
    postsign: async () => ({
      ok: true,
      value: { name: "contrato-firmado.pdf", folder: "Documentos", sizeBytes: 1200 },
    }),
    padesLowerLeft: async (placement) => [placement.rect[0], placement.rect[1]],
    previousSignatures: async () => NO_PREVIOUS_SIGNATURES,
    signedDocumentSignatures: async () => NO_PREVIOUS_SIGNATURES,
    discard: async () => {},
    ...overrides,
  };
}

function anOpener(overrides: Partial<SignedDocumentOpener> = {}): SignedDocumentOpener {
  return {
    openDocument: vi.fn(async () => {}),
    openFolder: vi.fn(async () => {}),
    ...overrides,
  };
}

function aSignature(name: string): PreviousSignature {
  return {
    name,
    idNumber: "00000000T",
    organizationIdentifier: null,
    issuer: "AC FNMT Usuarios",
    certificateSerialNumber: "1",
    signingTime: "2026-09-14T10:32:05Z",
    validity: "valid",
    validityReason: null,
    signingDate: null,
    closesDocument: false,
    countersignatures: [],
  };
}

const contract = document("contrato.pdf", { placement: seated });
const other = document("otro.pdf", { placement: seated });

function renderJourney(
  signer: SigningBackend,
  options: { opener?: SignedDocumentOpener; initialSignature?: VisibleSignature } = {},
) {
  const ports = {
    signer,
    stamps: unavailableStampComposer(),
    destinations: inMemoryDestination({
      folder: "Documentos",
      name: "contrato-firmado.pdf",
      writable: true,
    }),
    opener: options.opener ?? anOpener(),
  };
  const pdf = recordingDocument(3).document;
  const opening = { placement: seated, pageCount: 3 };
  const certificates = {
    listing: {
      kind: "listed" as const,
      certificates: [{ ...aCertificate, remembered: true }],
    },
    lookAgain: async () => {},
  };
  const reopenDocument = vi.fn();
  const rendered = renderHook(
    ({ active }: { active: DocumentInHand }) => {
      const placement = usePlacement({ document: opening, standardRectOn: null });
      return useSigningJourney({
        ports,
        document: { active, pdf, sizeBytes: 2_400, opening },
        placement,
        certificates,
        rubric: null,
        settingsFolder: "Documentos",
        initialSignature: options.initialSignature ?? visible,
        reopenDocument,
      });
    },
    { wrapper: withCatalog, initialProps: { active: contract } },
  );
  const journey = rendered.result;
  const signHere = async () => {
    await waitFor(() => expect(journey.current.certificate.state.kind).toBe("chosen"));
    await waitFor(() =>
      expect(journey.current.destination.value.name).toBe("contrato-firmado.pdf"),
    );
    await act(async () => {
      void journey.current.signing.sign();
    });
  };
  const switchTo = (active: DocumentInHand) => rendered.rerender({ active });
  return { journey, signHere, switchTo, reopenDocument };
}

describe("useSigningJourney, el resultado de la firma", () => {
  it("goes through the three stages in order and locks the window while it signs", async () => {
    const presignGate = gate<StageResult<{ kind: "typedOnScreen" }>>();
    const signGate = gate<StageResult<void>>();
    const postsignGate = gate<Awaited<ReturnType<SigningBackend["postsign"]>>>();
    const { journey, signHere } = renderJourney(
      aSigner({
        presign: () => presignGate.promise,
        sign: () => signGate.promise,
        postsign: () => postsignGate.promise,
      }),
    );

    await signHere();
    await waitFor(() => expect(journey.current.dialogs.stage).toBe("presign"));
    expect(journey.current.signals).toEqual({ signing: true, dialogOpen: true });

    await act(async () => presignGate.open({ ok: true, value: { kind: "typedOnScreen" } }));
    await waitFor(() => expect(journey.current.dialogs.stage).toBe("sign"));

    await act(async () => signGate.open({ ok: true, value: undefined }));
    await waitFor(() => expect(journey.current.dialogs.stage).toBe("postsign"));
    expect(journey.current.acknowledgement).toBeNull();

    await act(async () =>
      postsignGate.open({
        ok: true,
        value: { name: "contrato-firmado.pdf", folder: "Documentos", sizeBytes: 1200 },
      }),
    );
    await waitFor(() => expect(journey.current.acknowledgement).not.toBeNull());
    expect(journey.current.dialogs.stage).toBeNull();
    expect(journey.current.signals).toEqual({ signing: false, dialogOpen: false });
  });

  it("hands over every signature of the signed document in the acknowledgement", async () => {
    const { journey, signHere } = renderJourney(
      aSigner({
        signedDocumentSignatures: async () => ({
          ...NO_PREVIOUS_SIGNATURES,
          signatures: [aSignature("GRACE HOPPER"), aSignature("ADA LOVELACE")],
        }),
      }),
    );

    await signHere();

    await waitFor(() => expect(journey.current.acknowledgement?.signatures).toHaveLength(2));
    expect(journey.current.acknowledgement?.signatures.map((signature) => signature.name)).toEqual([
      "GRACE HOPPER",
      "ADA LOVELACE",
    ]);
    expect(journey.current.acknowledgement?.documentName).toBe("contrato-firmado.pdf");
    expect(journey.current.acknowledgement?.signedAt).not.toBeNull();
    expect(journey.current.acknowledgement?.destination).toEqual(journey.current.destination.value);
  });

  it("names the failure, leaves the document as it was, and lets the person retry", async () => {
    const sign = vi
      .fn<SigningBackend["sign"]>()
      .mockResolvedValueOnce(tokenAbsent)
      .mockResolvedValueOnce({ ok: true, value: undefined });
    const { journey, signHere } = renderJourney(aSigner({ sign }));

    await signHere();

    await waitFor(() => expect(journey.current.failure).not.toBeNull());
    expect(journey.current.failure).toEqual({
      situation: "tokenAbsent",
      detail: "CKR_DEVICE_REMOVED (C_Sign)",
      attemptsLeft: null,
    });
    expect(journey.current.acknowledgement).toBeNull();
    expect(journey.current.signals).toEqual({ signing: false, dialogOpen: false });

    await act(async () => {
      void journey.current.signing.sign();
    });

    await waitFor(() => expect(journey.current.acknowledgement).not.toBeNull());
    expect(journey.current.failure).toBeNull();
    expect(sign).toHaveBeenCalledTimes(2);
  });

  it("closes the failure and forgets the half-done cycle when going back", async () => {
    const discard = vi.fn(async () => {});
    const { journey, signHere } = renderJourney(
      aSigner({ sign: async () => tokenAbsent, discard }),
    );
    await signHere();
    await waitFor(() => expect(journey.current.failure).not.toBeNull());

    act(() => journey.current.signing.back());

    expect(journey.current.failure).toBeNull();
    expect(discard).toHaveBeenCalledOnce();
  });

  it("hands over a lost keyring pin as its own situation, so the store can be emptied", async () => {
    const lostPin: StageResult<never> = {
      ok: false,
      failure: {
        situation: "keyringPinMissing" as TokenFailure["situation"],
        detail: "PK11_CheckUserPassword: el pin del llavero no abre el almacen ya existente",
        attemptsLeft: null,
      },
    };
    const { journey, signHere } = renderJourney(aSigner({ sign: async () => lostPin }));

    await signHere();

    await waitFor(() => expect(journey.current.failure?.situation).toBe("keyringPinMissing"));
  });

  it("abandons a failure left on another tab, as pressing back would", async () => {
    const discard = vi.fn(async () => {});
    const { journey, signHere, switchTo } = renderJourney(
      aSigner({ sign: async () => tokenAbsent, discard }),
    );
    await signHere();
    await waitFor(() => expect(journey.current.failure).not.toBeNull());

    switchTo(other);

    await waitFor(() => expect(discard).toHaveBeenCalled());
    expect(journey.current.failure).toBeNull();
    switchTo(contract);
    expect(journey.current.failure).toBeNull();
  });

  it("closes the acknowledgement of a signature when another tab comes to the front", async () => {
    const { journey, signHere, switchTo } = renderJourney(aSigner());
    await signHere();
    await waitFor(() => expect(journey.current.acknowledgement).not.toBeNull());

    switchTo(other);

    await waitFor(() => expect(journey.current.acknowledgement).toBeNull());
    switchTo(contract);
    expect(journey.current.acknowledgement).toBeNull();
  });

  it("counts a failure to open the signed document or its folder in the acknowledgement", async () => {
    const opener = anOpener({
      openDocument: vi.fn(async () => {
        throw { situation: "unknown", detail: "xdg-open: no hay aplicación" };
      }),
      openFolder: vi.fn(async () => {
        throw { situation: "unknown", detail: "xdg-open: sin gestor de ficheros" };
      }),
    });
    const { journey, signHere, switchTo } = renderJourney(aSigner(), { opener });
    await signHere();
    await waitFor(() => expect(journey.current.acknowledgement).not.toBeNull());
    expect(journey.current.acknowledgement?.openFailure).toBeNull();

    act(() => journey.current.acknowledgement?.openDocument());
    await waitFor(() =>
      expect(journey.current.acknowledgement?.openFailure?.detail).toBe(
        "xdg-open: no hay aplicación",
      ),
    );

    act(() => journey.current.acknowledgement?.openFolder());
    await waitFor(() =>
      expect(journey.current.acknowledgement?.openFailure?.detail).toBe(
        "xdg-open: sin gestor de ficheros",
      ),
    );

    switchTo(other);
    await waitFor(() => expect(journey.current.acknowledgement).toBeNull());
    switchTo(contract);
    expect(journey.current.acknowledgement).toBeNull();
  });

  it("sends the phrase of Personalizada structured to the signer, without placeholders", async () => {
    const presigned: SigningOrder[] = [];
    const { signHere } = renderJourney(
      aSigner({
        presign: async (order) => {
          presigned.push(order);
          return { ok: true, value: { kind: "typedOnScreen" } };
        },
      }),
      {
        initialSignature: {
          ...visible,
          content: {
            model: "custom",
            phrase: [
              { text: "Visto bueno de " },
              { datum: "signer" },
              { text: ", " },
              { datum: "signedAt" },
              { text: " en Sevilla" },
            ],
          },
        },
      },
    );

    await signHere();

    await waitFor(() => expect(presigned).toHaveLength(1));
    expect(presigned[0]?.content).toEqual({
      model: "custom",
      phrase: [
        { text: "Visto bueno de " },
        { datum: "signer" },
        { text: ", " },
        { datum: "signedAt" },
        { text: " en Sevilla" },
      ],
    });
    expect(JSON.stringify(presigned[0])).not.toContain("$$");
  });
});
