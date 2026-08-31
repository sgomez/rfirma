import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { DocumentTray } from "./DocumentTray";
import type { RecentDocument } from "./recents";

function document(path: string, overrides: Partial<RecentDocument> = {}): RecentDocument {
  return {
    path,
    name: path.slice(path.lastIndexOf("/") + 1),
    badge: "Unsigned",
    modified: 1_700_000_000,
    lastUsed: 1_700_000_000,
    available: true,
    ...overrides,
  };
}

const noop = () => {};

function renderTray(props: Partial<Parameters<typeof DocumentTray>[0]> = {}) {
  return renderWithCatalog(
    <DocumentTray
      recents={[]}
      activePath={null}
      onOpen={noop}
      onSelect={noop}
      onForget={noop}
      {...props}
    />,
  );
}

// Grada A: la bandeja entera sin backend, con los recientes ya cacheados.
describe("DocumentTray", () => {
  it("shows only the drop zone and a hint while there is nothing recent", () => {
    renderTray();

    expect(
      screen.getByRole("button", { name: "Arrastra un PDF o pulsa para abrirlo" }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Aquí aparecerán los documentos que vayas firmando"),
    ).toBeInTheDocument();
    expect(screen.queryByRole("list")).not.toBeInTheDocument();
  });

  it("opens a document through the portal and never through a file input", async () => {
    const user = userEvent.setup();
    const onOpen = vi.fn();
    const { container } = renderTray({ onOpen });

    await user.click(screen.getByRole("button", { name: "Arrastra un PDF o pulsa para abrirlo" }));

    expect(onOpen).toHaveBeenCalledOnce();
    expect(container.querySelector("input[type=file]")).toBeNull();
  });

  it("paints the recents from their cached metadata", () => {
    renderTray({
      recents: [
        document("/documentos/contrato.pdf", { badge: "Signed" }),
        document("/documentos/factura.pdf"),
      ],
    });

    expect(screen.getByText("contrato.pdf")).toBeInTheDocument();
    expect(screen.getByText("Firmado")).toBeInTheDocument();
    expect(screen.getByText("factura.pdf")).toBeInTheDocument();
    expect(screen.getByText("Sin firmar")).toBeInTheDocument();
  });

  it("marks the active document as the selected row", () => {
    renderTray({
      recents: [document("/a.pdf"), document("/b.pdf")],
      activePath: "/b.pdf",
    });

    const selected = screen.getByRole("button", { name: /b\.pdf/ });
    expect(selected).toHaveAttribute("aria-current", "true");
  });

  it("changes document from a row", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    const factura = document("/factura.pdf");
    renderTray({ recents: [factura], onSelect });

    await user.click(screen.getByRole("button", { name: /factura\.pdf/ }));

    expect(onSelect).toHaveBeenCalledWith(factura);
  });

  it("badges a path that no longer answers as Unavailable and keeps it in the list", () => {
    renderTray({
      recents: [document("/usb/informe.pdf", { badge: "Signed", available: false })],
    });

    expect(screen.getByText("informe.pdf")).toBeInTheDocument();
    expect(screen.getByText("No disponible")).toBeInTheDocument();
    expect(screen.queryByText("Firmado")).not.toBeInTheDocument();
  });

  it("offers removing an unavailable row instead of purging it", async () => {
    const user = userEvent.setup();
    const onForget = vi.fn();
    renderTray({
      recents: [document("/usb/informe.pdf", { available: false })],
      onForget,
    });

    await user.click(screen.getByRole("button", { name: "Quitar de la lista" }));

    expect(onForget).toHaveBeenCalledWith("/usb/informe.pdf");
  });

  it("does not offer removing a row whose path answers", () => {
    renderTray({ recents: [document("/a.pdf")] });

    expect(screen.queryByRole("button", { name: "Quitar de la lista" })).not.toBeInTheDocument();
  });
});
