import { composeStories } from "@storybook/react-vite";
import { screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as stories from "./SignedPanel.stories";

const {
  JustSigned,
  JustSignedAfterChangedDocument,
  VerifyWithSignatures,
  VerifyWithProblems,
  OpenFailed,
} = composeStories(stories);

const cards = () =>
  (screen.getAllByRole("listitem") as HTMLElement[]).filter((item) =>
    /Firma \d/.test(item.textContent ?? ""),
  );

describe("the signed panel, by state", () => {
  describe("just signed", () => {
    it("heads the summary with the format and the count, and says when it was signed above", () => {
      renderWithCatalog(<JustSigned />);

      expect(screen.getByText("Firmas del documento")).toBeInTheDocument();
      expect(screen.getByText("PAdES")).toBeInTheDocument();
      expect(screen.getByText("2 firmas")).toBeInTheDocument();
      expect(screen.getByText(/^Firmado a las /)).toBeInTheDocument();
      expect(screen.queryByText("Firma visible")).not.toBeInTheDocument();
      expect(screen.queryByText("Resumen")).not.toBeInTheDocument();
    });

    it("lists every signature and marks only the last one as new", () => {
      renderWithCatalog(<JustSigned />);

      const [first, second] = cards() as [HTMLElement, HTMLElement];
      expect(cards()).toHaveLength(2);
      expect(within(first).getByText("Firma 1")).toBeInTheDocument();
      expect(within(first).queryByText("Nueva")).not.toBeInTheDocument();
      expect(within(second).getByText("Firma 2")).toBeInTheDocument();
      expect(within(second).getByText("Nueva")).toBeInTheDocument();
    });

    it("shows signer, issuer, validity and the declared date, and no serial number", () => {
      renderWithCatalog(<JustSigned />);

      const [, own] = cards() as [HTMLElement, HTMLElement];
      expect(within(own).getByText("LOVELACE BYRON ADA (00000000T)")).toBeInTheDocument();
      expect(within(own).getByText("AC FNMT Usuarios")).toBeInTheDocument();
      expect(within(own).getByText("Fecha")).toBeVisible();
      expect(within(own).getByText(/^14 sept 2026/)).toBeVisible();
      expect(within(own).getByText("Válida")).toBeVisible();
      expect(screen.queryByText("Fecha declarada")).not.toBeInTheDocument();
      expect(screen.queryByText(/serie/i)).not.toBeInTheDocument();
    });

    it("marks the signature that closes the document, once", () => {
      renderWithCatalog(<JustSigned />);

      expect(screen.getAllByText("No admite más firmas")).toHaveLength(1);
    });

    it("offers the PDF, the folder and Firmar, in that hierarchy, and labels the footer Documento", () => {
      renderWithCatalog(<JustSigned />);

      expect(screen.getByRole("button", { name: "Abrir el PDF" })).toHaveClass("rf-btn--primary");
      expect(screen.getByRole("button", { name: "Abrir la carpeta" })).toHaveClass(
        "rf-btn--secondary",
      );
      expect(screen.getByRole("button", { name: "Firmar" })).toHaveClass("rf-btn--ghost");
      expect(screen.queryByRole("button", { name: "Volver a firmar" })).not.toBeInTheDocument();
      expect(screen.getByText("Documento")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Cambiar" })).toBeInTheDocument();
    });

    it("puts the finding above the cards and says the change came before your signature", () => {
      renderWithCatalog(<JustSignedAfterChangedDocument />);

      const finding = screen.getByText("Se modificó antes de tu firma");
      expect(
        finding.compareDocumentPosition(screen.getByText("Firma 1")) &
          Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
    });
  });

  describe("only verifying", () => {
    it("keeps the plain wording of a finding and has no «Cambiar»", () => {
      renderWithCatalog(<VerifyWithProblems />);

      expect(screen.getByText("Se ha rellenado el formulario después de firmar")).toBeVisible();
      expect(screen.queryByRole("button", { name: "Cambiar" })).not.toBeInTheDocument();
      expect(screen.queryByText(/^Firmado a las /)).not.toBeInTheDocument();
    });

    it("labels the date Sellada with the TSA when time-stamped, and puts «En nombre de» only where an entity is named", () => {
      renderWithCatalog(<VerifyWithProblems />);

      const [, expired] = cards() as [HTMLElement, HTMLElement, HTMLElement];
      expect(within(expired).getByText("Sellada")).toBeVisible();
      expect(within(expired).getByText(/· TSA de pruebas$/)).toBeVisible();
      expect(within(expired).queryByText("Fecha")).not.toBeInTheDocument();
      expect(screen.getAllByText("En nombre de")).toHaveLength(1);
      expect(screen.getByText("B12345678")).toBeInTheDocument();
    });

    it("puts the validity in each header and the reason as the last row", () => {
      renderWithCatalog(<VerifyWithProblems />);

      const [valid, expired, invalid] = cards() as [HTMLElement, HTMLElement, HTMLElement];
      expect(within(valid).getByText("Válida")).toBeVisible();
      expect(within(expired).getByText("Caducada")).toBeVisible();
      expect(within(invalid).getByText("No válida")).toBeVisible();
      const reasonRow = within(invalid).getByText("Motivo").parentElement;
      expect(reasonRow).toHaveTextContent("ANA LOPEZ GARCIA");
      expect(reasonRow?.nextElementSibling).toBeNull();
    });

    it("paints no field that is absent", () => {
      renderWithCatalog(<VerifyWithSignatures />);

      expect(screen.queryByText("En nombre de")).not.toBeInTheDocument();
      expect(screen.queryByText("Motivo")).not.toBeInTheDocument();
    });
  });

  describe("a failure to open", () => {
    it("says why it could not open instead of leaving the button doing nothing", () => {
      renderWithCatalog(<OpenFailed />);

      expect(screen.getByRole("alert")).toBeInTheDocument();
    });
  });
});
