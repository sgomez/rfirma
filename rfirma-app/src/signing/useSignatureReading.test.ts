import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { SignedDocumentOpener } from "./destination";
import { type SigningBackend, unavailableSigningBackend } from "./flow";
import type { PreviousSignaturesReport } from "./previousSignatures";
import { useSignatureReading } from "./useSignatureReading";

const report: PreviousSignaturesReport = {
  signatures: [],
  warningCount: 0,
  tone: "information",
  changedAfterLastSignature: false,
  findings: [],
};

function aSigner(overrides: Partial<SigningBackend> = {}): SigningBackend {
  return {
    ...unavailableSigningBackend(),
    previousSignatures: async () => report,
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

const contract = { id: "doc-1", name: "contrato.pdf" };
const row = { folder: "Documentos" };

function mount(
  options: {
    signer?: SigningBackend;
    opener?: SignedDocumentOpener;
    active?: { id: string; name: string } | null;
  } = {},
) {
  const signer = options.signer ?? aSigner();
  const opener = options.opener ?? anOpener();
  return renderHook(({ active }) => useSignatureReading(signer, opener, active, row), {
    initialProps: { active: options.active === undefined ? contract : options.active },
  });
}

describe("useSignatureReading", () => {
  it("delivers no reading until a document is viewed", () => {
    const { result } = mount();

    expect(result.current.reading).toBeNull();
  });

  it("is reading while the signatures are being read", () => {
    const { result } = mount({
      signer: aSigner({ previousSignatures: () => new Promise(() => {}) }),
    });

    act(() => result.current.view("doc-1"));

    expect(result.current.reading?.kind).toBe("reading");
    expect(result.current.reading?.documentName).toBe("contrato.pdf");
  });

  it("delivers what was read, with its format", async () => {
    const { result } = mount({
      signer: aSigner({ previousSignatures: async () => ({ ...report, format: "xades" }) }),
    });

    act(() => result.current.view("doc-1"));

    await waitFor(() => expect(result.current.reading?.kind).toBe("read"));
    expect(result.current.reading).toMatchObject({ kind: "read", format: "xades", signatures: [] });
  });

  it("reads the document as PAdES when the report names no format", async () => {
    const { result } = mount();

    act(() => result.current.view("doc-1"));

    await waitFor(() => expect(result.current.reading).toMatchObject({ format: "pades" }));
  });

  it("delivers the failure when reading is refused", async () => {
    const { result } = mount({
      signer: aSigner({ previousSignatures: () => Promise.reject(new Error("ilegible")) }),
    });

    act(() => result.current.view("doc-1"));

    await waitFor(() => expect(result.current.reading?.kind).toBe("failed"));
  });

  it("takes the destination folder from the recents row of the document", () => {
    const { result } = mount();

    act(() => result.current.view("doc-1"));

    expect(result.current.reading?.destination).toEqual({
      folder: "Documentos",
      name: "contrato.pdf",
      writable: true,
    });
  });

  it("is signable only for a PDF", () => {
    const { result } = mount({ active: { id: "doc-2", name: "contrato.xsig" } });

    act(() => result.current.view("doc-2"));

    expect(result.current.reading?.signable).toBe(false);
  });

  it("opens the document and its folder through the opener", () => {
    const opener = anOpener();
    const { result } = mount({ opener });
    act(() => result.current.view("doc-1"));

    act(() => result.current.reading?.openDocument());
    act(() => result.current.reading?.openFolder());

    expect(opener.openDocument).toHaveBeenCalledWith("doc-1");
    expect(opener.openFolder).toHaveBeenCalledWith("doc-1");
  });

  it("keeps the failure to open the document", async () => {
    const opener = anOpener({ openDocument: () => Promise.reject(new Error("sin portal")) });
    const { result } = mount({ opener });
    act(() => result.current.view("doc-1"));

    act(() => result.current.reading?.openDocument());

    await waitFor(() => expect(result.current.reading?.openFailure).not.toBeNull());
  });

  it("stops reading when signing again", () => {
    const { result } = mount();
    act(() => result.current.view("doc-1"));

    act(() => result.current.reading?.signAgain());

    expect(result.current.reading).toBeNull();
  });

  it("closes when another tab comes to the front", () => {
    const { result, rerender } = mount();
    act(() => result.current.view("doc-1"));

    rerender({ active: { id: "doc-2", name: "otro.pdf" } });
    expect(result.current.reading).toBeNull();

    rerender({ active: contract });
    expect(result.current.reading).toBeNull();
  });
});
