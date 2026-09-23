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
  it("renders the local CA certificate signal born in Comprobando with no action", async () => {
    const rows: SignalRow[] = [
      {
        signal: "localCaCertificate",
        value: "",
        verdict: "checking",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Certificado de rFirma")).toBeInTheDocument();
    expect(within(row).getByText("Comprobando")).toBeInTheDocument();
    expect(within(row).queryByRole("button")).not.toBeInTheDocument();
  });

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

  it("renders Incorrecto with Instalar and a store detail list when the certificate is nowhere trusted", async () => {
    const rows: SignalRow[] = [
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
    const user = userEvent.setup();
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Certificado de rFirma")).toBeInTheDocument();
    expect(within(row).getByText("0 de 2 navegadores")).toBeInTheDocument();
    expect(within(row).getByText("Incorrecto")).toBeInTheDocument();
    expect(within(row).getByRole("button", { name: "Instalar" })).toBeInTheDocument();
    expect(within(row).queryByRole("button", { name: /navegadores/ })).toBeInTheDocument();

    const toggle = within(row).getByRole("button", { name: "Ver navegadores" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(within(row).queryByText("Firefox")).not.toBeInTheDocument();

    await user.click(toggle);

    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(within(row).getByText("Firefox")).toBeInTheDocument();
    expect(within(row).getByText("Chrome y Chromium")).toBeInTheDocument();
  });

  it("renders Correcto with a trusted detail list when the certificate is in every store", async () => {
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
    const user = userEvent.setup();
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("2 de 2 navegadores")).toBeInTheDocument();
    expect(within(row).getByText("Correcto")).toBeInTheDocument();
    expect(within(row).queryByRole("button", { name: "Instalar" })).not.toBeInTheDocument();

    await user.click(within(row).getByRole("button", { name: "Ver navegadores" }));

    expect(within(row).getAllByText("Instalado")).toHaveLength(2);
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
