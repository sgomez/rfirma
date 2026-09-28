import { screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { certificate } from "./sedeWindowFixtures";
import type { SiteErrandView } from "./siteErrandView";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const listen = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const configuration = {
  language: "es",
  destination: "Documentos",
  destinationMode: "next_to_the_original",
  rememberVisibleSignature: true,
  rememberActivity: true,
  notifyNewVersion: true,
  theme: "system",
  offersTheOriginalFolder: false,
  setupWizardSeen: false,
  consentCountdown: false,
  honourAutomaticSelection: false,
};

describe("el punto de entrada de la ventana de sede", () => {
  let pushErrand: ((view: SiteErrandView) => void) | undefined;

  beforeEach(() => {
    vi.resetModules();
    invoke.mockReset();
    listen.mockReset();
    pushErrand = undefined;

    document.body.innerHTML = '<div id="root"></div>';

    invoke.mockImplementation(async (command: string) => {
      if (command === "read_configuration") return configuration;
      if (command === "read_site_errand") return null;
      return null;
    });
    listen.mockImplementation(
      async (_event: string, handler: (event: { payload: SiteErrandView }) => void) => {
        pushErrand = (view) => handler({ payload: view });
        return () => {};
      },
    );
  });

  it("never asks about a new version while the window is idle", async () => {
    await import("./main");

    await waitFor(() => expect(invoke).toHaveBeenCalledWith("read_configuration"));

    expect(invoke.mock.calls.map(([command]) => command)).not.toContain("check_for_new_version");
  });

  it("never asks about a new version through the normal course of an errand", async () => {
    await import("./main");
    await waitFor(() => expect(listen).toHaveBeenCalled());

    pushErrand?.({
      origin: "sede.ejemplo.gob.es",
      stage: { kind: "askingForConsent", certificates: [certificate()] },
    });

    await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());

    expect(invoke.mock.calls.map(([command]) => command)).not.toContain("check_for_new_version");
  });
});
