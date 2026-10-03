import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { formatSignedAt } from "../App.signingOrder";
import { renderWithCatalog } from "../testing/render";
import type { PreviousSignature } from "./previousSignatures";
import { SignedPanel } from "./SignedPanel";

const noop = () => {};

const SIGNED_AT = new Date("2026-01-01T11:04:00");

function aSignature(overrides: Partial<PreviousSignature> = {}): PreviousSignature {
  return {
    name: "ADA LOVELACE",
    idNumber: "00000000T",
    organizationIdentifier: null,
    issuer: "AC FNMT Usuarios",
    certificateSerialNumber: "1",
    signingTime: "2026-09-14T10:32:05Z",
    status: "valid",
    reason: null,
    countersignatures: [],
    ...overrides,
  };
}

function renderPanel(props: Partial<Parameters<typeof SignedPanel>[0]> = {}) {
  return renderWithCatalog(
    <SignedPanel
      documentName="contrato-firmado.pdf"
      signedAt={SIGNED_AT}
      signatures={[aSignature()]}
      destination={{ folder: "Documentos", name: null, writable: true }}
      onOpenDocument={noop}
      onOpenFolder={noop}
      onSign={noop}
      onChangeDestination={noop}
      {...props}
    />,
  );
}

describe("SignedPanel", () => {
  it("heads the summary with the format and the count of signatures", () => {
    renderPanel({ signatures: [aSignature(), aSignature({ name: "GRACE HOPPER" })] });

    expect(screen.getByText("Firmas del documento")).toBeInTheDocument();
    expect(screen.getByText("PAdES")).toBeInTheDocument();
    expect(screen.getByText("2 firmas")).toBeInTheDocument();
  });

  it("says when it was signed above the heading", () => {
    renderPanel();

    expect(screen.getByText(/^Firmado a las /)).toBeInTheDocument();
  });

  it("lists every signature and marks only the last one as new", () => {
    renderPanel({
      signatures: [aSignature({ name: "GRACE HOPPER" }), aSignature({ name: "ADA LOVELACE" })],
    });

    const cards = screen.getAllByRole("listitem");
    const [first, second] = cards as [HTMLElement, HTMLElement];
    expect(cards).toHaveLength(2);
    expect(within(first).getByText("Firma 1")).toBeInTheDocument();
    expect(within(first).queryByText("Nueva")).not.toBeInTheDocument();
    expect(within(second).getByText("Firma 2")).toBeInTheDocument();
    expect(within(second).getByText("Nueva")).toBeInTheDocument();
  });

  it("shows signer, issuer and declared date, and no serial number nor status", () => {
    renderPanel();

    expect(screen.getByText("ADA LOVELACE (00000000T)")).toBeInTheDocument();
    expect(screen.getByText("AC FNMT Usuarios")).toBeInTheDocument();
    expect(screen.getByText(formatSignedAt(new Date("2026-09-14T10:32:05Z"), "es"))).toBeVisible();
    expect(screen.queryByText("Válida")).not.toBeInTheDocument();
    expect(screen.queryByText(/serie/i)).not.toBeInTheDocument();
  });

  it("shows on behalf of only when the certificate names an entity", () => {
    renderPanel({
      signatures: [
        aSignature({ organizationIdentifier: "VATES-B00000000" }),
        aSignature({ name: "GRACE HOPPER" }),
      ],
    });

    expect(screen.getAllByText("En nombre de")).toHaveLength(1);
    expect(screen.getByText("VATES-B00000000")).toBeInTheDocument();
  });

  it("does not paint a field that is absent", () => {
    renderPanel({ signatures: [aSignature({ signingTime: null })] });

    expect(screen.queryByText("Fecha declarada")).not.toBeInTheDocument();
    expect(screen.queryByText("En nombre de")).not.toBeInTheDocument();
  });

  it("no longer shows the visible signature line nor the old heading", () => {
    renderPanel();

    expect(screen.queryByText("Firma visible")).not.toBeInTheDocument();
    expect(screen.queryByText("Resumen")).not.toBeInTheDocument();
  });

  it("labels the footer Document, with Cambiar active", async () => {
    const change = vi.fn();
    renderPanel({ onChangeDestination: change });

    expect(screen.getByText("Documento")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Cambiar" }));

    expect(change).toHaveBeenCalledOnce();
  });

  it("offers the PDF, the folder and Firmar, in the hierarchy of the artboard", () => {
    renderPanel();

    expect(screen.getByRole("button", { name: "Abrir el PDF" })).toHaveClass("rf-btn--primary");
    expect(screen.getByRole("button", { name: "Abrir la carpeta" })).toHaveClass(
      "rf-btn--secondary",
    );
    expect(screen.getByRole("button", { name: "Firmar" })).toHaveClass("rf-btn--ghost");
    expect(screen.queryByRole("button", { name: "Volver a firmar" })).not.toBeInTheDocument();
  });

  it("opens the signed PDF", async () => {
    const open = vi.fn();
    renderPanel({ onOpenDocument: open });

    await userEvent.click(screen.getByRole("button", { name: "Abrir el PDF" }));

    expect(open).toHaveBeenCalledOnce();
  });

  it("opens the folder where it landed", async () => {
    const open = vi.fn();
    renderPanel({ onOpenFolder: open });

    await userEvent.click(screen.getByRole("button", { name: "Abrir la carpeta" }));

    expect(open).toHaveBeenCalledOnce();
  });

  it("goes back to sign the same document again", async () => {
    const sign = vi.fn();
    renderPanel({ onSign: sign });

    await userEvent.click(screen.getByRole("button", { name: "Firmar" }));

    expect(sign).toHaveBeenCalledOnce();
  });

  it("says why it could not open instead of leaving the button doing nothing", () => {
    renderPanel({
      failure: { situation: "unknown", detail: "no portal responded", attemptsLeft: null },
    });

    expect(screen.getByRole("alert")).toBeInTheDocument();
  });
});
