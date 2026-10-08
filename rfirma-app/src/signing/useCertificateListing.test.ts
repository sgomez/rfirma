import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { anInstalledCertificate } from "../preferences/testing/fixtures";
import type { Certificate, CertificateStore } from "./certificate";
import { announcingCertificateStore } from "./testing/readers";
import { useCertificateListing } from "./useCertificateListing";

const installed = anInstalledCertificate({ id: "installed" });
const onToken = anInstalledCertificate({ id: "token", stores: ["card"] });

function aStore(found: Certificate[], overrides: Partial<CertificateStore> = {}): CertificateStore {
  return {
    list: vi.fn(async () => [...found]),
    install: vi.fn(async () => false),
    remove: vi.fn(async () => {}),
    emptyStore: vi.fn(async () => {}),
    followReaders: () => () => {},
    ...overrides,
  };
}

const dnie = anInstalledCertificate({ id: "dnie", stores: ["dnie"] });

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

describe("useCertificateListing · los lectores", () => {
  it("is unavailable until the backend says what it sees", async () => {
    const { result } = await listed(announcingCertificateStore([onToken]));

    expect(result.current.reader).toEqual({ kind: "unavailable" });
  });

  it("follows what the backend says about the reader, and keeps the list while it reads", async () => {
    const store = announcingCertificateStore([onToken]);
    const { result } = await listed(store);

    act(() => store.announce({ reader: { kind: "reading" }, certificates: null }));

    expect(result.current.reader).toEqual({ kind: "reading" });
    expect(result.current.listing).toMatchObject({ kind: "listed", certificates: [onToken] });
  });

  it("changes the list by itself when the card arrives and when it leaves", async () => {
    const store = announcingCertificateStore([onToken]);
    const { result } = await listed(store);

    act(() => store.announce({ reader: { kind: "dnieReady" }, certificates: [onToken, dnie] }));
    expect(result.current.listing).toMatchObject({ certificates: [onToken, dnie] });

    act(() => store.announce({ reader: { kind: "noCard" }, certificates: [onToken] }));
    expect(result.current.listing).toMatchObject({ certificates: [onToken] });
    expect(result.current.reader).toEqual({ kind: "noCard" });
  });

  it("keeps the list the card brought when the first search answers later", async () => {
    let answer: (found: Certificate[]) => void = () => {};
    const store = {
      ...announcingCertificateStore(),
      list: () => new Promise<Certificate[]>((resolve) => (answer = resolve)),
    };
    const { result } = renderHook(() => useCertificateListing(store));

    act(() => store.announce({ reader: { kind: "dnieReady" }, certificates: [dnie] }));
    await act(async () => answer([onToken]));

    expect(result.current.listing).toMatchObject({ kind: "listed", certificates: [dnie] });
  });

  it("stops listening to the readers when it goes away", async () => {
    const stop = vi.fn();
    const { unmount } = await listed(aStore([], { followReaders: () => stop }));

    unmount();

    expect(stop).toHaveBeenCalledOnce();
  });
});
