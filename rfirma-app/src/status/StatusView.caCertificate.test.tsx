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
  it("measures the local CA certificate signal on opening, without Volver a comprobar", async () => {
    const born: SignalRow = {
      signal: "localCaCertificate",
      value: "",
      verdict: "checking",
      action: null,
      detail: null,
      candidates: null,
      restartFirefoxNotice: false,
    };
    const measured: SignalRow = {
      signal: "localCaCertificate",
      value: "3/3",
      verdict: "correct",
      action: null,
      detail: null,
      candidates: null,
      restartFirefoxNotice: false,
    };
    renderWithCatalog(
      <StatusView
        statusPort={memoryStatus([born], undefined, undefined, undefined, undefined, measured)}
        onClose={() => {}}
      />,
    );

    const row = await screen.findByRole("status");
    await waitFor(() => {
      expect(within(row).getByText("3 de 3 navegadores")).toBeInTheDocument();
    });
    expect(within(row).queryByText("Comprobando")).not.toBeInTheDocument();
  });

  it("remeasures the certificate stores row on Volver a comprobar", async () => {
    const user = userEvent.setup();
    let resolveRecheck!: (rows: SignalRow[]) => void;
    const recheckPromise = new Promise<SignalRow[]>((resolve) => {
      resolveRecheck = resolve;
    });

    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue([
        {
          signal: "userCertificates",
          value: "0",
          verdict: "attention",
          action: {
            kind: "link",
            target: "certificateIssuance",
          },
          detail: null,
          candidates: null,
          restartFirefoxNotice: false,
        },
      ]),
      recheck: vi.fn().mockReturnValue(recheckPromise),
      measureVersion: vi.fn(),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn(),
      chooseSiteSignatureHandler: vi.fn(),
      withdrawRfirma: vi.fn(),
    };

    renderWithCatalog(<StatusView statusPort={statusPort} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Ninguno")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Volver a comprobar" }));

    expect(within(row).getByText("Comprobando")).toBeInTheDocument();

    resolveRecheck([
      {
        signal: "userCertificates",
        value: "1",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ]);

    await waitFor(() => {
      expect(within(row).getByText("Correcto")).toBeInTheDocument();
    });
    expect(within(row).getByText("1 certificado")).toBeInTheDocument();
  });
});
