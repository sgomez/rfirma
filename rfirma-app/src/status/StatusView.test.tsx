import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { StatusView } from "./StatusView";
import { memoryStatus, type SignalRow } from "./status";

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
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
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
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Versión")).toBeInTheDocument();
    expect(within(row).getByText("0.4.1 → 0.5.0")).toBeInTheDocument();
    expect(within(row).getByText("Atención")).toBeInTheDocument();
    expect(within(row).getByRole("button", { name: "Actualizar" })).toBeInTheDocument();
  });

  it("renders rFirma and Correcto when rFirma signs at sites", async () => {
    const rows: SignalRow[] = [
      {
        signal: "siteSignature",
        value: "rFirma",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Firma en sedes")).toBeInTheDocument();
    expect(within(row).getByText("rFirma")).toBeInTheDocument();
    expect(within(row).getByText("Correcto")).toBeInTheDocument();
    expect(within(row).queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders Sin configurar and Atención when no program is configured", async () => {
    const rows: SignalRow[] = [
      {
        signal: "siteSignature",
        value: "",
        verdict: "attention",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Sin configurar")).toBeInTheDocument();
    expect(within(row).getByText("Atención")).toBeInTheDocument();
    expect(within(row).queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders No se puede consultar and No aplica without action inside the sandbox", async () => {
    const rows: SignalRow[] = [
      {
        signal: "siteSignature",
        value: "",
        verdict: "notApplicable",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("No se puede consultar")).toBeInTheDocument();
    expect(within(row).getByText("No aplica")).toBeInTheDocument();
    expect(within(row).queryByRole("button")).not.toBeInTheDocument();
  });

  it("renders plain text with no dropdown when there is only one candidate", async () => {
    const rows: SignalRow[] = [
      {
        signal: "siteSignature",
        value: "rFirma",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("rFirma")).toBeInTheDocument();
    expect(within(row).queryByRole("combobox")).not.toBeInTheDocument();
  });

  it("renders a dropdown with the current candidate selected when there are two or more", async () => {
    const rows: SignalRow[] = [
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
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    const dropdown = within(row).getByRole("combobox");
    expect(dropdown).toHaveTextContent("AutoFirma");
    expect(within(row).getByRole("button", { name: "Usar rFirma" })).toBeInTheDocument();
  });
});
