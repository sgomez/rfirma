import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import type { PreviousSignature, PreviousSignaturesReport } from "./previousSignatures";
import { SignaturesDialog } from "./SignaturesDialog";

function aSignature(overrides: Partial<PreviousSignature> = {}): PreviousSignature {
  return {
    name: "ADA LOVELACE",
    idNumber: "00000000T",
    organizationIdentifier: null,
    issuer: "AC FNMT Usuarios",
    certificateSerialNumber: "1",
    signingTime: "2026-09-14T10:32:05Z",
    validity: "valid",
    validityReason: null,
    signingDate: { kind: "declared", at: "2026-09-14T10:32:05Z" },
    closesDocument: false,
    countersignatures: [],
    ...overrides,
  };
}

function aReport(overrides: Partial<PreviousSignaturesReport> = {}): PreviousSignaturesReport {
  return {
    signatures: [aSignature()],
    warningCount: 0,
    tone: "information",
    changedAfterLastSignature: false,
    findings: [],
    ...overrides,
  };
}

describe("SignaturesDialog", () => {
  it("titles itself with the format and the count of signatures", () => {
    renderWithCatalog(
      <SignaturesDialog
        report={aReport({ signatures: [aSignature(), aSignature()] })}
        onClose={() => {}}
      />,
    );

    const dialog = screen.getByRole("dialog", { name: "Firmas del documento" });
    expect(within(dialog).getByText("PAdES · 2 firmas")).toBeVisible();
  });

  it("shows the findings above the cards with validity and reason", () => {
    renderWithCatalog(
      <SignaturesDialog
        report={aReport({
          signatures: [
            aSignature({ validity: "invalid", validityReason: { kind: "modifiedAfterSigning" } }),
          ],
          findings: ["formFilledAfterSigning"],
        })}
        onClose={() => {}}
      />,
    );

    expect(screen.getByText("Se ha rellenado el formulario después de firmar")).toBeVisible();
    expect(screen.getByText("No válida")).toBeVisible();
    expect(screen.getByText("Se ha modificado después de firmarse")).toBeVisible();
    expect(screen.queryByText("Nueva")).not.toBeInTheDocument();
  });

  it("has Cerrar as its only button", async () => {
    const onClose = vi.fn();
    renderWithCatalog(<SignaturesDialog report={aReport()} onClose={onClose} />);

    expect(screen.getAllByRole("button")).toHaveLength(1);
    await userEvent.click(screen.getByRole("button", { name: "Cerrar" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
