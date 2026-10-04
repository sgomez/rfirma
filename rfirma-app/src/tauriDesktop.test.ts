import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const { tauriVersionCheck } = await import("./tauriDesktop");

describe("el puerto de la versión sobre Tauri", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("asks the backend whether there is a newer published version, and nothing else", async () => {
    invoke.mockResolvedValue({ version: "0.4.1" });

    expect(await tauriVersionCheck().latest()).toEqual({ version: "0.4.1" });
    expect(invoke).toHaveBeenCalledExactlyOnceWith("check_for_new_version");
  });

  it("reads no answer as nothing to say, and not as a failure", async () => {
    invoke.mockResolvedValue(null);

    expect(await tauriVersionCheck().latest()).toBeNull();
  });
});
