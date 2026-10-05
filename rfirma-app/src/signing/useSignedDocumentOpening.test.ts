import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { SignedDocumentOpener } from "./destination";
import { useSignedDocumentOpening } from "./useSignedDocumentOpening";

function anOpener(overrides: Partial<SignedDocumentOpener> = {}): SignedDocumentOpener {
  return {
    openDocument: vi.fn(async () => {}),
    openFolder: vi.fn(async () => {}),
    ...overrides,
  };
}

const refusing = () => Promise.reject(new Error("sin portal"));

describe("useSignedDocumentOpening", () => {
  it("opens the signed document and its folder through the opener", () => {
    const opener = anOpener();
    const { result } = renderHook(() => useSignedDocumentOpening(opener));

    act(() => result.current.openDocument("doc-1"));
    act(() => result.current.openFolder());

    expect(opener.openDocument).toHaveBeenCalledWith("doc-1");
    expect(opener.openFolder).toHaveBeenCalledWith(undefined);
    expect(result.current.failure).toBeNull();
  });

  it("keeps the failure when opening is refused", async () => {
    const { result } = renderHook(() =>
      useSignedDocumentOpening(anOpener({ openDocument: refusing })),
    );

    act(() => result.current.openDocument());

    await waitFor(() => expect(result.current.failure).not.toBeNull());
  });

  it("clears the failure on the next attempt", async () => {
    const { result } = renderHook(() =>
      useSignedDocumentOpening(anOpener({ openFolder: refusing })),
    );
    act(() => result.current.openFolder());
    await waitFor(() => expect(result.current.failure).not.toBeNull());

    act(() => result.current.openDocument());

    expect(result.current.failure).toBeNull();
  });

  it("forgets the failure when told to", async () => {
    const { result } = renderHook(() =>
      useSignedDocumentOpening(anOpener({ openDocument: refusing })),
    );
    act(() => result.current.openDocument());
    await waitFor(() => expect(result.current.failure).not.toBeNull());

    act(() => result.current.forgetFailure());

    expect(result.current.failure).toBeNull();
  });
});
