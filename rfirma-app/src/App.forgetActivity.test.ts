import { describe, expect, it, vi } from "vitest";
import { forgetActivity } from "./App.forgetActivity";

describe("forgetActivity", () => {
  it("forgets the documents even when forgetting the preferences fails", async () => {
    const forgetDocuments = vi.fn(async () => {});

    await expect(
      forgetActivity(async () => Promise.reject(new Error("disco")), forgetDocuments),
    ).rejects.toThrow("disco");
    expect(forgetDocuments).toHaveBeenCalled();
  });

  it("rethrows the first failure when both fail", async () => {
    await expect(
      forgetActivity(
        async () => Promise.reject(new Error("primero")),
        async () => Promise.reject(new Error("segundo")),
      ),
    ).rejects.toThrow("primero");
  });

  it("rethrows a rejection with null instead of swallowing it", async () => {
    await expect(
      forgetActivity(
        async () => Promise.reject(null),
        async () => {},
      ),
    ).rejects.toBeNull();
  });
});
