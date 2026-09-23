import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { StatusView } from "./StatusView";
import { memoryStatus, type SignalRow, type StatusPort } from "./status";

const stillChecking: SignalRow = {
  signal: "localCaCertificate",
  value: "",
  verdict: "checking",
  action: null,
  detail: null,
  candidates: null,
  restartFirefoxNotice: false,
};

describe("StatusView", () => {
  it("installs the local CA certificate and shows the restart notice when Firefox was alive", async () => {
    const user = userEvent.setup();
    let resolveInstall!: (row: SignalRow) => void;
    const installPromise = new Promise<SignalRow>((resolve) => {
      resolveInstall = resolve;
    });

    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue([
        {
          signal: "localCaCertificate",
          value: "0/2",
          verdict: "incorrect",
          action: {
            kind: "repair",
            target: "installLocalCaCertificate",
          },
          detail: null,
          candidates: null,
          restartFirefoxNotice: false,
        },
      ]),
      recheck: vi.fn(),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn().mockReturnValue(installPromise),
      chooseSiteSignatureHandler: vi.fn(),
      withdrawRfirma: vi.fn(),
    };

    renderWithCatalog(<StatusView statusPort={statusPort} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    await user.click(within(row).getByRole("button", { name: "Instalar" }));

    expect(within(row).getByText("Comprobando")).toBeInTheDocument();
    expect(statusPort.installLocalCaCertificate).toHaveBeenCalledOnce();

    resolveInstall({
      signal: "localCaCertificate",
      value: "2/2",
      verdict: "correct",
      action: null,
      detail: null,
      candidates: null,
      restartFirefoxNotice: true,
    });

    await waitFor(() => {
      expect(within(row).getByText("Correcto")).toBeInTheDocument();
    });
    expect(within(row).getByText("Reinicia Firefox para que surta efecto.")).toBeInTheDocument();
  });

  it("does not show the restart notice when Firefox was not alive", async () => {
    const rows: SignalRow[] = [
      {
        signal: "localCaCertificate",
        value: "2/2",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Correcto")).toBeInTheDocument();
    expect(screen.queryByText("Reinicia Firefox para que surta efecto.")).not.toBeInTheDocument();
  });

  // ID-363: el botón lo fija el veredicto, nunca los dos a la vez.
  it("offers Retirar… instead of Instalar when the certificate is trusted everywhere", async () => {
    const rows: SignalRow[] = [
      {
        signal: "localCaCertificate",
        value: "2/2",
        verdict: "correct",
        action: null,
        detail: {
          kind: "trust",
          stores: [
            { brand: "firefox", trusted: true },
            { brand: "chrome", trusted: true },
          ],
        },
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByRole("button", { name: "Retirar…" })).toBeInTheDocument();
    expect(within(row).queryByRole("button", { name: "Instalar" })).not.toBeInTheDocument();
  });
});
