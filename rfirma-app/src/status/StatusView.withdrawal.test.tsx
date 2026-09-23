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
  it("opens the withdrawal dialog from Retirar…, and remeasures both signals when it closes", async () => {
    const user = userEvent.setup();
    const initialRows: SignalRow[] = [
      {
        signal: "siteSignature",
        value: "rFirma",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
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
    const afterWithdrawal: SignalRow[] = [
      {
        signal: "siteSignature",
        value: "",
        verdict: "attention",
        action: {
          kind: "choice",
          target: "rfirma.desktop",
        },
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
      {
        signal: "localCaCertificate",
        value: "0/2",
        verdict: "incorrect",
        action: {
          kind: "repair",
          target: "installLocalCaCertificate",
        },
        detail: {
          kind: "trust",
          stores: [
            { brand: "firefox", trusted: false },
            { brand: "chrome", trusted: false },
          ],
        },
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue(initialRows),
      recheck: vi.fn().mockResolvedValue(afterWithdrawal),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn(),
      chooseSiteSignatureHandler: vi.fn(),
      withdrawRfirma: vi.fn().mockResolvedValue({
        handler: { kind: "withdrawn" },
        stores: [
          { brand: "firefox", outcome: { kind: "withdrawn" } },
          { brand: "chrome", outcome: { kind: "withdrawn" } },
        ],
      }),
    };

    renderWithCatalog(<StatusView statusPort={statusPort} onClose={() => {}} />);

    await screen.findAllByRole("status");
    await user.click(screen.getByRole("button", { name: "Retirar…" }));

    const dialog = await screen.findByRole("alertdialog", {
      name: "Retirar el certificado de rFirma",
    });
    await user.click(within(dialog).getByRole("button", { name: "Retirar" }));
    const result = await screen.findByRole("alertdialog", { name: "Certificado retirado" });
    await user.click(within(result).getByRole("button", { name: "Cerrar" }));

    expect(statusPort.withdrawRfirma).toHaveBeenCalledOnce();
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    await waitFor(() => {
      expect(statusPort.recheck).toHaveBeenCalledOnce();
    });
    expect(await screen.findByText("Sin configurar")).toBeInTheDocument();
    expect(screen.getByText("0 de 2 navegadores")).toBeInTheDocument();
  });

  it("does not close the panel on Escape while the withdrawal dialog is open", async () => {
    const user = userEvent.setup();
    const rows: SignalRow[] = [
      {
        signal: "localCaCertificate",
        value: "2/2",
        verdict: "correct",
        action: null,
        detail: { kind: "trust", stores: [{ brand: "firefox", trusted: true }] },
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    const onClose = vi.fn();
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={onClose} />);

    const row = await screen.findByRole("status");
    await user.click(within(row).getByRole("button", { name: "Retirar…" }));
    await screen.findByRole("alertdialog");

    await user.keyboard("{Escape}");

    expect(onClose).not.toHaveBeenCalled();
  });
});
