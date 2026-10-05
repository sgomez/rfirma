import { act, renderHook, waitFor } from "@testing-library/react";
import { type ReactNode, useMemo, useRef, useState } from "react";
import { describe, expect, it, vi } from "vitest";
import {
  aCertificate,
  aDestination,
  destinationOfferingSingleChoice,
  document,
  failingCertificateStore,
} from "../App.testSupport";
import type { Placement } from "../placement/pageSets";
import { usePlacement } from "../placement/usePlacement";
import {
  type Certificate,
  type CertificateStore,
  emptyCertificateStore,
} from "../signing/certificate";
import type { DestinationSource } from "../signing/destination";
import type { SigningBackend, SigningOrder } from "../signing/flow";
import { NO_PREVIOUS_SIGNATURES } from "../signing/previousSignatures";
import { unavailableStampComposer } from "../signing/stampPreview";
import { useCertificateListing } from "../signing/useCertificateListing";
import { DEFAULT_VISIBLE_SIGNATURE, type VisibleSignature } from "../signing/visibleSignature";
import { CatalogProvider } from "../testing/render";
import { standardRectOnPageOf } from "../viewer/signatureBox";
import { recordingDocument } from "../viewer/testing/documentViewerFixtures";
import { useSigningJourney } from "./useSigningJourney";

const withCatalog = ({ children }: { children: ReactNode }) => (
  <CatalogProvider language="es">{children}</CatalogProvider>
);

const PAGE_COUNT = 5;

const grace: Certificate = {
  ...aCertificate,
  id: "otra",
  holderName: "Grace Hopper Murray",
  givenName: "Grace",
  surname: "Hopper Murray",
};
const expired = { kind: "expired", notAfter: 0 } as const;
const remembered = (certificate: Certificate): Certificate => ({
  ...certificate,
  remembered: true,
});
const storeOf = (...found: Certificate[]): CertificateStore => ({
  ...emptyCertificateStore(),
  list: async () => found,
});
function recordingSigner(presigned: SigningOrder[] = []) {
  const signer: SigningBackend = {
    presign: async (order) => {
      presigned.push(order);
      return { ok: true, value: { kind: "typedOnScreen" } };
    },
    sign: async () => ({ ok: true, value: undefined }),
    postsign: vi.fn(async () => ({
      ok: true as const,
      value: { name: "contrato-firmado.pdf", folder: "Documentos", sizeBytes: 1200 },
    })),
    padesLowerLeft: async (placement) => [placement.rect[0], placement.rect[1]],
    previousSignatures: async () => NO_PREVIOUS_SIGNATURES,
    signedDocumentSignatures: async () => NO_PREVIOUS_SIGNATURES,
    discard: async () => {},
  };
  return signer;
}

interface Setup {
  store?: CertificateStore;
  destinations?: DestinationSource;
  signer?: SigningBackend;
  settingsFolder?: string | null;
  initialSignature?: VisibleSignature;
}

function renderJourney({
  store = storeOf(remembered(aCertificate)),
  destinations = aDestination(),
  signer = recordingSigner(),
  settingsFolder = "Documentos",
  initialSignature = DEFAULT_VISIBLE_SIGNATURE,
}: Setup = {}) {
  const pdf = recordingDocument(PAGE_COUNT).document;
  const standardRectOn = standardRectOnPageOf(pdf);
  const ports = { signer, stamps: unavailableStampComposer(), destinations, opener: openerDouble };
  const reopenDocument = vi.fn();

  return renderHook(
    () => {
      const [name, show] = useState("primero.pdf");
      const saved = useRef(new Map<string, Placement | null>());
      const opening = useMemo(
        () => ({ placement: saved.current.get(name) ?? null, pageCount: PAGE_COUNT }),
        [name],
      );
      const placement = usePlacement({
        document: opening,
        standardRectOn,
        onChange: (next) => saved.current.set(name, next),
      });
      const certificates = useCertificateListing(store);
      const journey = useSigningJourney({
        ports,
        document: { active: document(name), pdf, sizeBytes: 2_400, opening },
        placement,
        certificates,
        rubric: null,
        settingsFolder,
        initialSignature,
        reopenDocument,
      });
      return { journey, placement, show };
    },
    { wrapper: withCatalog },
  );
}

const openerDouble = {
  openDocument: async () => {},
  openFolder: async () => {},
};

type Rendered = ReturnType<typeof renderJourney>;

async function listed(rendered: Rendered) {
  await waitFor(() =>
    expect(rendered.result.current.journey.certificate.state.kind).not.toBe("loading"),
  );
}

async function turnOnVisibleSignature(rendered: Rendered) {
  await act(async () => {
    const { value, change } = rendered.result.current.journey.signature;
    change({ ...value, enabled: true });
  });
}

const turnOffVisibleSignature = (rendered: Rendered) =>
  act(async () => {
    const { value, change } = rendered.result.current.journey.signature;
    change({ ...value, enabled: false });
  });

const placedPages = (rendered: Rendered) => rendered.result.current.placement.placement?.pages;

async function sign(rendered: Rendered) {
  await act(async () => {
    void rendered.result.current.journey.signing.sign();
  });
  await waitFor(() => expect(rendered.result.current.journey.acknowledgement).not.toBeNull());
}

describe("useSigningJourney · el certificado", () => {
  it("names the failure of the search and loads the list when looking again", async () => {
    const rendered = renderJourney({ store: failingCertificateStore(1, [aCertificate]) });
    await listed(rendered);

    const { state } = rendered.result.current.journey.certificate;
    expect(state).toMatchObject({ kind: "failed", failure: { situation: "moduleNotFound" } });

    await act(async () => rendered.result.current.journey.certificate.lookAgain());

    expect(rendered.result.current.journey.certificate.state).toMatchObject({
      kind: "unchosen",
      certificates: [aCertificate],
    });
  });

  it("chooses no certificate by itself, and takes the one that is picked", async () => {
    const rendered = renderJourney({ store: storeOf(aCertificate, grace) });
    await listed(rendered);
    expect(rendered.result.current.journey.certificate.state.kind).toBe("unchosen");

    act(() => rendered.result.current.journey.certificate.choose(grace));

    expect(rendered.result.current.journey.certificate.state).toMatchObject({
      kind: "chosen",
      certificate: { id: grace.id },
    });
  });

  it("forgets what was picked when the list is searched again", async () => {
    const rendered = renderJourney({ store: storeOf(aCertificate, grace) });
    await listed(rendered);
    act(() => rendered.result.current.journey.certificate.choose(grace));

    await act(async () => rendered.result.current.journey.certificate.lookAgain());

    await waitFor(() =>
      expect(rendered.result.current.journey.certificate.state.kind).toBe("unchosen"),
    );
  });

  it("starts with the certificate used last time already chosen", async () => {
    const rendered = renderJourney({ store: storeOf(aCertificate, remembered(grace)) });
    await listed(rendered);

    expect(rendered.result.current.journey.certificate.state).toMatchObject({
      kind: "chosen",
      certificate: { id: grace.id },
    });
  });

  it("falls back to no certificate when the remembered one is gone, without an error", async () => {
    const rendered = renderJourney({ store: storeOf(aCertificate, grace) });
    await listed(rendered);

    expect(rendered.result.current.journey.certificate.state.kind).toBe("unchosen");
    expect(rendered.result.current.journey.failure).toBeNull();
  });

  it.each([
    ["the remembered one has expired", [aCertificate, remembered({ ...grace, status: expired })]],
    ["the sole one can be used", [aCertificate]],
    ["the sole one cannot be used", [{ ...aCertificate, status: expired }]],
  ])("does not preselect a certificate when %s", async (_case, found) => {
    const rendered = renderJourney({ store: storeOf(...found) });
    await listed(rendered);

    expect(rendered.result.current.journey.certificate.state.kind).toBe("unchosen");
  });

  it("keeps the chosen certificate and the visible signature when the document changes", async () => {
    const rendered = renderJourney({ store: storeOf(aCertificate, grace) });
    await listed(rendered);
    act(() => rendered.result.current.journey.certificate.choose(grace));
    await turnOnVisibleSignature(rendered);

    act(() => rendered.result.current.show("segundo.pdf"));

    expect(rendered.result.current.journey.certificate.state).toMatchObject({
      kind: "chosen",
      certificate: { id: grace.id },
    });
    expect(rendered.result.current.journey.signature.value.enabled).toBe(true);
  });
});

describe("useSigningJourney · la firma visible", () => {
  it("starts off, with nothing placed, and signs with an order without placement", async () => {
    const presigned: SigningOrder[] = [];
    const rendered = renderJourney({ signer: recordingSigner(presigned) });
    await listed(rendered);

    expect(rendered.result.current.journey.signature.value.enabled).toBe(false);
    expect(placedPages(rendered)).toBeUndefined();

    await sign(rendered);

    expect(presigned).toHaveLength(1);
    expect(presigned[0]?.placement).toBeNull();
  });

  it("signs with an order without placement when it was turned on and off again", async () => {
    const presigned: SigningOrder[] = [];
    const rendered = renderJourney({ signer: recordingSigner(presigned) });
    await listed(rendered);
    await turnOnVisibleSignature(rendered);
    await waitFor(() => expect(placedPages(rendered)).toEqual({ only: [1] }));
    await turnOffVisibleSignature(rendered);

    await sign(rendered);

    expect(presigned).toHaveLength(1);
    expect(presigned[0]?.placement).toBeNull();
  });

  it("is kept off while no certificate is chosen, with nothing placed", async () => {
    const rendered = renderJourney({
      store: storeOf(aCertificate, grace),
      initialSignature: { ...DEFAULT_VISIBLE_SIGNATURE, enabled: true },
    });
    await listed(rendered);

    expect(rendered.result.current.journey.signature.value.enabled).toBe(false);
    expect(placedPages(rendered)).toBeUndefined();
  });

  it("places the box on the page in view when it is turned on", async () => {
    const rendered = renderJourney();
    await listed(rendered);
    act(() => rendered.result.current.placement.viewPage(3));

    await turnOnVisibleSignature(rendered);

    await waitFor(() => expect(placedPages(rendered)).toEqual({ only: [3] }));
    expect(rendered.result.current.journey.signature.value.enabled).toBe(true);
  });

  it("keeps the box where it was when it goes off and on again", async () => {
    const rendered = renderJourney();
    await listed(rendered);
    await turnOnVisibleSignature(rendered);
    await waitFor(() => expect(placedPages(rendered)).toEqual({ only: [1] }));
    act(() => rendered.result.current.placement.viewPage(3));

    await turnOffVisibleSignature(rendered);
    await turnOnVisibleSignature(rendered);

    expect(placedPages(rendered)).toEqual({ only: [1] });
  });

  it("brings back the placement of a tab, on its page, when the tab is chosen again", async () => {
    const rendered = renderJourney();
    await listed(rendered);
    await turnOnVisibleSignature(rendered);
    await waitFor(() => expect(placedPages(rendered)).toEqual({ only: [1] }));
    act(() => rendered.result.current.placement.viewPage(3));
    await act(async () => rendered.result.current.placement.sealViewedPage());
    await waitFor(() => expect(placedPages(rendered)).toEqual({ only: [3] }));

    act(() => rendered.result.current.show("segundo.pdf"));
    await waitFor(() => expect(placedPages(rendered)).toEqual({ only: [1] }));
    act(() => rendered.result.current.show("primero.pdf"));

    expect(placedPages(rendered)).toEqual({ only: [3] });
  });
});

describe("useSigningJourney · el destino", () => {
  const single = {
    id: "single-42",
    folder: "Escritorio",
    name: "contrato-firmado-2.pdf",
    writable: true,
  };
  const initial = { folder: "Documentos", name: "contrato-firmado.pdf", writable: true };

  it("shows the settings folder until the backend answers, and when it cannot", async () => {
    const rendered = renderJourney({
      destinations: {
        previewFor: () => Promise.reject(new Error("sin respuesta")),
        chooseSingle: async () => null,
      },
    });

    expect(rendered.result.current.journey.destination.value).toEqual({
      folder: "Documentos",
      name: null,
      writable: true,
    });
    await listed(rendered);
    expect(rendered.result.current.journey.destination.value.folder).toBe("Documentos");
  });

  it("asks the backend for the destination of the document in front", async () => {
    const rendered = renderJourney({
      destinations: destinationOfferingSingleChoice(initial, single),
    });

    await waitFor(() => expect(rendered.result.current.journey.destination.value).toEqual(initial));
  });

  it("sends the one-signature destination to the backend, and changes the footer", async () => {
    const signer = recordingSigner();
    const rendered = renderJourney({
      signer,
      destinations: destinationOfferingSingleChoice(initial, single),
    });
    await waitFor(() => expect(rendered.result.current.journey.destination.value).toEqual(initial));

    await act(async () => rendered.result.current.journey.destination.chooseSingle());
    await waitFor(() => expect(rendered.result.current.journey.destination.value).toEqual(single));

    await sign(rendered);

    expect(signer.postsign).toHaveBeenCalledWith("single-42");
  });

  it("forgets the one-signature destination when the document changes", async () => {
    const rendered = renderJourney({
      destinations: destinationOfferingSingleChoice(initial, single),
    });
    await act(async () => rendered.result.current.journey.destination.chooseSingle());
    await waitFor(() => expect(rendered.result.current.journey.destination.value).toEqual(single));

    act(() => rendered.result.current.show("segundo.pdf"));

    await waitFor(() => expect(rendered.result.current.journey.destination.value).toEqual(initial));
  });

  it("forgets the one-signature destination once the signature is done", async () => {
    const rendered = renderJourney({
      destinations: destinationOfferingSingleChoice(initial, single),
    });
    await act(async () => rendered.result.current.journey.destination.chooseSingle());
    await waitFor(() => expect(rendered.result.current.journey.destination.value).toEqual(single));

    await sign(rendered);

    await waitFor(() => expect(rendered.result.current.journey.destination.value).toEqual(initial));
  });
});
