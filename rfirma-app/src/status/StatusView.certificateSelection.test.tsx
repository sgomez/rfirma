import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { StatusView } from "./StatusView";
import { type SignalRow, type StatusPort } from "./status";

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
  it("chooses rFirma from Usar rFirma and remeasures both signals through Comprobando", async () => {
    const user = userEvent.setup();
    let resolveChoose!: (rows: SignalRow[]) => void;
    const choosePromise = new Promise<SignalRow[]>((resolve) => {
      resolveChoose = resolve;
    });
    const chooseSiteSignatureHandler = vi.fn().mockReturnValue(choosePromise);

    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue([
        {
          signal: "siteSignature",
          value: "AutoFirma",
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
          detail: null,
          candidates: null,
          restartFirefoxNotice: false,
        },
      ]),
      recheck: vi.fn(),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn(),
      chooseSiteSignatureHandler,
      withdrawRfirma: vi.fn(),
    };

    renderWithCatalog(<StatusView statusPort={statusPort} onClose={() => {}} />);

    const rows = await screen.findAllByRole("status");
    const siteRow = rows[0];
    const certRow = rows[1];
    if (!siteRow || !certRow) throw new Error("deberian existir las dos filas");

    await user.click(within(siteRow).getByRole("button", { name: "Usar rFirma" }));

    expect(chooseSiteSignatureHandler).toHaveBeenCalledWith("rfirma.desktop");
    expect(within(siteRow).getByText("Comprobando")).toBeInTheDocument();
    expect(within(certRow).getByText("Comprobando")).toBeInTheDocument();

    resolveChoose([
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
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ]);

    await waitFor(() => {
      expect(within(siteRow).getByText("Correcto")).toBeInTheDocument();
    });
    expect(within(certRow).getByText("Correcto")).toBeInTheDocument();
  });

  it("chooses AutoFirma from the dropdown without touching the certificate", async () => {
    const user = userEvent.setup();
    const chooseSiteSignatureHandler = vi.fn().mockResolvedValue([
      {
        signal: "siteSignature",
        value: "AutoFirma",
        verdict: "attention",
        action: {
          kind: "choice",
          target: "rfirma.desktop",
        },
        detail: null,
        candidates: [
          { id: "rfirma.desktop", name: "rFirma", selected: false },
          { id: "autofirma.desktop", name: "AutoFirma", selected: true },
        ],
        restartFirefoxNotice: false,
      },
      {
        signal: "localCaCertificate",
        value: "2/2",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ]);

    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue([
        {
          signal: "siteSignature",
          value: "rFirma",
          verdict: "correct",
          action: null,
          detail: null,
          candidates: [
            { id: "rfirma.desktop", name: "rFirma", selected: true },
            { id: "autofirma.desktop", name: "AutoFirma", selected: false },
          ],
          restartFirefoxNotice: false,
        },
      ]),
      recheck: vi.fn(),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn(),
      chooseSiteSignatureHandler,
      withdrawRfirma: vi.fn(),
    };

    renderWithCatalog(<StatusView statusPort={statusPort} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    await user.click(within(row).getByRole("combobox"));
    await user.click(within(row).getByRole("option", { name: "AutoFirma" }));

    expect(chooseSiteSignatureHandler).toHaveBeenCalledWith("autofirma.desktop");
    await waitFor(() => {
      expect(within(row).getByText("AutoFirma")).toBeInTheDocument();
    });
  });
});
