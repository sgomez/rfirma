import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { StatusView } from "./StatusView";
import { memoryStatus, type SignalRow, type StatusPort } from "./status";

describe("StatusView", () => {
  it("renders the title and close button", () => {
    renderWithCatalog(<StatusView onClose={() => {}} />);

    expect(screen.getByRole("heading", { name: "Estado de rFirma" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cerrar" })).toBeInTheDocument();
  });

  it("calls onClose when clicking Cerrar", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderWithCatalog(<StatusView onClose={onClose} />);

    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("calls onClose when pressing Escape", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    renderWithCatalog(<StatusView onClose={onClose} />);

    await user.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalledOnce();
  });

  it("does not call onClose when Escape was default-prevented", () => {
    const onClose = vi.fn();
    renderWithCatalog(<StatusView onClose={onClose} />);

    const event = new KeyboardEvent("keydown", { key: "Escape", cancelable: true });
    event.preventDefault();
    window.dispatchEvent(event);

    expect(onClose).not.toHaveBeenCalled();
  });

  it("keeps the close button in a footer that is sibling to the scrollable body (WCAG 2.4.11)", () => {
    const { container } = renderWithCatalog(<StatusView onClose={() => {}} />);

    const body = container.querySelector(".status-view__body");
    const footer = container.querySelector(".status-view__footer");

    expect(body).not.toBeNull();
    expect(footer).not.toBeNull();
    expect(body?.nextElementSibling).toBe(footer);
    expect(footer?.parentElement).toBe(body?.parentElement);
  });

  it("renders the status table columns: Señal, Valor, Veredicto, Acción", () => {
    renderWithCatalog(<StatusView onClose={() => {}} />);

    expect(screen.getByText("Señal")).toBeInTheDocument();
    expect(screen.getByText("Valor")).toBeInTheDocument();
    expect(screen.getByText("Veredicto")).toBeInTheDocument();
    expect(screen.getByText("Acción")).toBeInTheDocument();
  });

  it("renders a row with role status showing Correcto when up to date", async () => {
    const rows: SignalRow[] = [
      {
        signal: "version",
        value: "0.4.1",
        verdict: "correct",
        action: null,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Versión")).toBeInTheDocument();
    expect(within(row).getByText("0.4.1")).toBeInTheDocument();
    expect(within(row).getByText("Correcto")).toBeInTheDocument();
    expect(within(row).queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders Atención and an update link action when an update is available", async () => {
    const rows: SignalRow[] = [
      {
        signal: "version",
        value: "0.4.1 → 0.5.0",
        verdict: "attention",
        action: {
          kind: "link",
          target: "releases",
        },
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Versión")).toBeInTheDocument();
    expect(within(row).getByText("0.4.1 → 0.5.0")).toBeInTheDocument();
    expect(within(row).getByText("Atención")).toBeInTheDocument();
    expect(within(row).getByRole("button", { name: "Actualizar" })).toBeInTheDocument();
  });

  it("renders Ninguno and Cómo instalar when no certificate stores are detected", async () => {
    const rows: SignalRow[] = [
      {
        signal: "userCertificates",
        value: "0",
        verdict: "attention",
        action: {
          kind: "link",
          target: "certificateIssuance",
        },
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Tus certificados")).toBeInTheDocument();
    expect(within(row).getByText("Ninguno")).toBeInTheDocument();
    expect(within(row).getByText("Atención")).toBeInTheDocument();
    expect(within(row).getByRole("button", { name: "Cómo instalar" })).toBeInTheDocument();
  });

  it("renders the store count and Correcto without action when certificates are detected", async () => {
    const rows: SignalRow[] = [
      {
        signal: "userCertificates",
        value: "3",
        verdict: "correct",
        action: null,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Tus certificados")).toBeInTheDocument();
    expect(within(row).getByText("3 almacenes")).toBeInTheDocument();
    expect(within(row).getByText("Correcto")).toBeInTheDocument();
    expect(within(row).queryByRole("button")).not.toBeInTheDocument();
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
        },
      ]),
      recheck: vi.fn().mockReturnValue(recheckPromise),
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
      },
    ]);

    await waitFor(() => {
      expect(within(row).getByText("Correcto")).toBeInTheDocument();
    });
    expect(within(row).getByText("1 almacén")).toBeInTheDocument();
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
        },
      ]),
      recheck: vi.fn().mockReturnValue(recheckPromise),
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
        },
      ]),
      recheck: vi.fn().mockReturnValue(recheckPromise),
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
      },
    ]);

    await waitFor(() => {
      expect(within(row).getByText("Correcto")).toBeInTheDocument();
    });
  });
});
