import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { anInstalledCertificate } from "../preferences/preferencesFixtures";
import type { Certificate, CertificateStore } from "./certificate";
import { useCertificateListing } from "./useCertificateListing";

const installed = anInstalledCertificate({ id: "installed" });
const onToken = anInstalledCertificate({ id: "token", stores: ["card"] });

function aStore(found: Certificate[], overrides: Partial<CertificateStore> = {}): CertificateStore {
  return {
    list: vi.fn(async () => [...found]),
    install: vi.fn(async () => false),
    remove: vi.fn(async () => {}),
    emptyStore: vi.fn(async () => {}),
    ...overrides,
  };
}

async function listed(store: CertificateStore) {
  const hook = renderHook(() => useCertificateListing(store));
  await waitFor(() => expect(hook.result.current.listing.kind).toBe("listed"));
  return hook;
}

describe("useCertificateListing", () => {
  it("starts looking for certificates as soon as it mounts", async () => {
    const store = aStore([onToken]);
    const { result } = renderHook(() => useCertificateListing(store));

    expect(result.current.listing).toEqual({ kind: "loading" });
    await waitFor(() =>
      expect(result.current.listing).toEqual({ kind: "listed", certificates: [onToken] }),
    );
  });

  it("keeps the failure when looking for certificates is rejected", async () => {
    const store = aStore([], { list: async () => Promise.reject(new Error("sin lector")) });
    const { result } = renderHook(() => useCertificateListing(store));

    await waitFor(() => expect(result.current.listing.kind).toBe("failed"));
  });

  it("looks again when asked to", async () => {
    const found: Certificate[] = [];
    const store = aStore([], { list: async () => [...found] });
    const { result } = await listed(store);

    found.push(onToken);
    await act(() => result.current.lookAgain());

    expect(result.current.listing).toEqual({ kind: "listed", certificates: [onToken] });
  });

  it("offers only the installed certificates to the preferences", async () => {
    const { result } = await listed(aStore([onToken, installed]));

    expect(result.current.installed).toEqual([installed]);
  });

  it("lists again after installing one", async () => {
    const found: Certificate[] = [];
    const store = aStore([], {
      list: async () => [...found],
      install: async () => {
        found.push(installed);
        return true;
      },
    });
    const { result } = await listed(store);

    let added = false;
    await act(async () => {
      added = await result.current.install();
    });

    expect(added).toBe(true);
    expect(result.current.installed).toEqual([installed]);
  });

  it("does not list again when nothing was installed", async () => {
    const store = aStore([onToken]);
    const { result } = await listed(store);

    await act(() => result.current.install());

    expect(store.list).toHaveBeenCalledTimes(1);
  });

  it("lists again after removing one", async () => {
    const found = [installed];
    const store = aStore([], {
      list: async () => [...found],
      remove: async () => {
        found.pop();
      },
    });
    const { result } = await listed(store);

    await act(() => result.current.remove("installed"));

    expect(result.current.installed).toEqual([]);
  });

  it("lists again after emptying the store", async () => {
    const found = [installed, onToken];
    const store = aStore([], {
      list: async () => [...found],
      emptyStore: async () => {
        found.splice(0, 1);
      },
    });
    const { result } = await listed(store);

    await act(() => result.current.emptyStore());

    expect(result.current.listing).toEqual({ kind: "listed", certificates: [onToken] });
  });
});
