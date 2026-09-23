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
  it("notifies onRowsChange with the rows read at startup", async () => {
    const rows: SignalRow[] = [
      {
        signal: "version",
        value: "0.4.1",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    const onRowsChange = vi.fn();
    renderWithCatalog(
      <StatusView statusPort={memoryStatus(rows)} onClose={() => {}} onRowsChange={onRowsChange} />,
    );

    await screen.findByRole("status");

    await waitFor(() => {
      expect(onRowsChange).toHaveBeenCalledWith(rows);
    });
  });

  it("notifies onRowsChange again once Volver a comprobar remeasures", async () => {
    const user = userEvent.setup();
    const initialRow: SignalRow = {
      signal: "version",
      value: "0.4.1",
      verdict: "correct",
      action: null,
      detail: null,
      candidates: null,
      restartFirefoxNotice: false,
    };
    const initialRows: SignalRow[] = [initialRow];
    const recheckedRows: SignalRow[] = [{ ...initialRow, verdict: "attention", value: "0.5.0" }];
    const onRowsChange = vi.fn();
    renderWithCatalog(
      <StatusView
        statusPort={memoryStatus(initialRows, recheckedRows)}
        onClose={() => {}}
        onRowsChange={onRowsChange}
      />,
    );

    await screen.findByRole("status");
    onRowsChange.mockClear();

    await user.click(screen.getByRole("button", { name: "Volver a comprobar" }));

    await waitFor(() => {
      expect(onRowsChange).toHaveBeenCalledWith(recheckedRows);
    });
  });

  it("transitions through Comprobando and remeasures after clicking an action", async () => {
    const user = userEvent.setup();
    const open = vi.fn();
    let resolveRecheck!: (rows: SignalRow[]) => void;
    const recheckPromise = new Promise<SignalRow[]>((resolve) => {
      resolveRecheck = resolve;
    });

    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue([
        {
          signal: "version",
          value: "0.4.1 → 0.5.0",
          verdict: "attention",
          action: {
            kind: "link",
            target: "releases",
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

    renderWithCatalog(
      <StatusView statusPort={statusPort} externalDestinations={{ open }} onClose={() => {}} />,
    );

    const row = await screen.findByRole("status");
    const updateButton = within(row).getByRole("button", { name: "Actualizar" });
    await user.click(updateButton);

    expect(open).toHaveBeenCalledWith("releases");
    expect(within(row).getByText("Comprobando")).toBeInTheDocument();
    expect(within(row).queryByRole("button", { name: "Actualizar" })).not.toBeInTheDocument();

    resolveRecheck([
      {
        signal: "version",
        value: "0.5.0",
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
    expect(within(row).getByText("0.5.0")).toBeInTheDocument();
  });

  it("transitions row through Comprobando and updates on Volver a comprobar", async () => {
    const user = userEvent.setup();
    let resolveRecheck!: (rows: SignalRow[]) => void;
    const recheckPromise = new Promise<SignalRow[]>((resolve) => {
      resolveRecheck = resolve;
    });

    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue([
        {
          signal: "version",
          value: "0.4.1",
          verdict: "checking",
          action: null,
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
    expect(within(row).getByText("Comprobando")).toBeInTheDocument();

    const recheckBtn = screen.getByRole("button", { name: "Volver a comprobar" });
    await user.click(recheckBtn);

    expect(within(row).getByText("Comprobando")).toBeInTheDocument();

    resolveRecheck([
      {
        signal: "version",
        value: "0.4.1",
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
  });
});
