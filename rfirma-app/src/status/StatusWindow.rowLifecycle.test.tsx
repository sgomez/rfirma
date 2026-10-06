import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { StatusWindow } from "./StatusWindow";
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

describe("StatusWindow", () => {
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
      <StatusWindow
        statusPort={memoryStatus(rows)}
        onClose={() => {}}
        onRowsChange={onRowsChange}
      />,
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
      <StatusWindow
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
      measureVersion: vi.fn(),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn(),
      chooseSiteSignatureHandler: vi.fn(),
      withdrawRfirma: vi.fn(),
    };

    renderWithCatalog(
      <StatusWindow statusPort={statusPort} externalDestinations={{ open }} onClose={() => {}} />,
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

  it("measures the version signal on open, without clicking anything", async () => {
    let resolveMeasureVersion!: (row: SignalRow) => void;
    const measureVersionPromise = new Promise<SignalRow>((resolve) => {
      resolveMeasureVersion = resolve;
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
      recheck: vi.fn(),
      measureVersion: vi.fn().mockReturnValue(measureVersionPromise),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn(),
      chooseSiteSignatureHandler: vi.fn(),
      withdrawRfirma: vi.fn(),
    };

    renderWithCatalog(<StatusWindow statusPort={statusPort} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Comprobando")).toBeInTheDocument();

    resolveMeasureVersion({
      signal: "version",
      value: "0.4.1",
      verdict: "correct",
      action: null,
      detail: null,
      candidates: null,
      restartFirefoxNotice: false,
    });

    await waitFor(() => {
      expect(within(row).getByText("Correcto")).toBeInTheDocument();
    });
  });

  it("does not remeasure the local CA certificate while measuring the version on open", async () => {
    const measureLocalCaCertificate = vi.fn();
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
        {
          signal: "localCaCertificate",
          value: "",
          verdict: "correct",
          action: null,
          detail: null,
          candidates: null,
          restartFirefoxNotice: false,
        },
      ]),
      recheck: vi.fn(),
      measureVersion: vi.fn().mockResolvedValue({
        signal: "version",
        value: "0.4.1",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      }),
      measureLocalCaCertificate,
      installLocalCaCertificate: vi.fn(),
      chooseSiteSignatureHandler: vi.fn(),
      withdrawRfirma: vi.fn(),
    };

    renderWithCatalog(<StatusWindow statusPort={statusPort} onClose={() => {}} />);

    await waitFor(() => {
      expect(statusPort.measureVersion).toHaveBeenCalled();
    });
    expect(measureLocalCaCertificate).not.toHaveBeenCalled();
  });

  it("measures the version too on Volver a comprobar, with measureVersion", async () => {
    const user = userEvent.setup();
    const versionRow: SignalRow = {
      signal: "version",
      value: "0.4.1",
      verdict: "correct",
      action: null,
      detail: null,
      candidates: null,
      restartFirefoxNotice: false,
    };
    const statusPort: StatusPort = {
      readStatus: vi.fn().mockResolvedValue([versionRow]),
      recheck: vi.fn().mockResolvedValue([{ ...versionRow, verdict: "checking" }]),
      measureVersion: vi.fn().mockResolvedValue({ ...versionRow, value: "0.5.0" }),
      measureLocalCaCertificate: vi.fn().mockResolvedValue(stillChecking),
      installLocalCaCertificate: vi.fn(),
      chooseSiteSignatureHandler: vi.fn(),
      withdrawRfirma: vi.fn(),
    };

    renderWithCatalog(<StatusWindow statusPort={statusPort} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("0.4.1")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Volver a comprobar" }));

    await waitFor(() => {
      expect(statusPort.measureVersion).toHaveBeenCalled();
    });
    expect(within(row).getByText("0.5.0")).toBeInTheDocument();
  });
});
