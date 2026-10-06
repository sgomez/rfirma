import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { PdfSource } from "../viewer/source";
import { recordingDocument } from "../viewer/testing/fixtures";
import type { DocumentInHand } from "./document";
import type { Documents } from "./useDocuments";
import { useOpenPdf } from "./useOpenPdf";

function document(name: string): DocumentInHand {
  return {
    id: `id-${name}`,
    name,
    badge: "Unsigned",
    modified: 1_700_000_000,
    placement: null,
    remembered: true,
  };
}

function documentsWith(active: DocumentInHand | null, overrides: Partial<Documents> = {}) {
  return {
    active,
    open: async () => {},
    clearRecents: async () => {},
    ...overrides,
  } as Documents;
}

const opensEverything: PdfSource = {
  open: async () => ({ ok: true, pdf: recordingDocument(4).document, sizeBytes: 1234 }),
};

describe("useOpenPdf", () => {
  it("delivers the pdf, its size and the page count of the active document", async () => {
    const documents = documentsWith(document("a.pdf"));
    const { result } = renderHook(() => useOpenPdf(documents, opensEverything));

    await waitFor(() => expect(result.current.pdf).not.toBeNull());
    expect(result.current.sizeBytes).toBe(1234);
    expect(result.current.failure).toBeNull();
    expect(result.current.opening?.pageCount).toBe(4);
  });

  it("stays empty without a document or when it is not a pdf", () => {
    const documents = documentsWith(document("a.txt"));
    const nothing = documentsWith(null);
    const none = renderHook(() => useOpenPdf(nothing, opensEverything));
    const text = renderHook(() => useOpenPdf(documents, opensEverything));

    for (const { result } of [none, text]) {
      expect(result.current.pdf).toBeNull();
      expect(result.current.failure).toBeNull();
      expect(result.current.sizeBytes).toBeNull();
      expect(result.current.opening).toBeNull();
    }
  });

  it("delivers the failure when the pdf cannot be opened", async () => {
    const failing: PdfSource = {
      open: async () => ({
        ok: false,
        failure: { situation: "documentUnreadable", detail: "roto" },
      }),
    };

    const documents = documentsWith(document("a.pdf"));
    const { result } = renderHook(() => useOpenPdf(documents, failing));

    await waitFor(() => expect(result.current.failure?.detail).toBe("roto"));
    expect(result.current.pdf).toBeNull();
    expect(result.current.sizeBytes).toBeNull();
    expect(result.current.opening?.pageCount).toBe(0);
  });

  it("delivers a new opening each time the same document is reopened", async () => {
    const active = document("a.pdf");
    const { result, rerender } = renderHook(
      ({ documents }) => useOpenPdf(documents, opensEverything),
      { initialProps: { documents: documentsWith(active) } },
    );
    await waitFor(() => expect(result.current.opening).not.toBeNull());
    const first = result.current.opening;

    rerender({ documents: documentsWith({ ...active }) });

    await waitFor(() => expect(result.current.opening).not.toBe(first));
    expect(result.current.opening).not.toBeNull();
  });

  it("reports a failure to open", async () => {
    const documents = documentsWith(null, {
      open: () => Promise.reject(new Error("portal caído")),
    });
    const { result } = renderHook(() => useOpenPdf(documents, opensEverything));

    act(() => result.current.open());

    await waitFor(() => expect(result.current.failure).not.toBeNull());
  });

  it("reports a failure to clear the recents", async () => {
    const documents = documentsWith(null, {
      clearRecents: () => Promise.reject(new Error("disco lleno")),
    });
    const { result } = renderHook(() => useOpenPdf(documents, opensEverything));

    act(() => result.current.clearRecents());

    await waitFor(() => expect(result.current.failure).not.toBeNull());
  });
});
