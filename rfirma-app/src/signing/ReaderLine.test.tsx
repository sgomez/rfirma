import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { ReaderLine } from "./ReaderLine";

describe("the reader line under the certificate selector", () => {
  it("shows what the card is doing", () => {
    renderWithCatalog(<ReaderLine reader={{ kind: "noCard" }} />);

    expect(screen.getByRole("status")).toBeInTheDocument();
  });

  for (const kind of ["noReader", "unavailable"] as const) {
    it(`stays hidden when the status is ${kind}`, () => {
      renderWithCatalog(<ReaderLine reader={{ kind }} />);

      expect(screen.queryByRole("status")).not.toBeInTheDocument();
    });
  }

  describe("the help for a card it cannot read", () => {
    it("points to Windows Update and the DGP page on Windows", () => {
      renderWithCatalog(<ReaderLine reader={{ kind: "unreadable" }} platform="windows" />);

      expect(screen.getByRole("status")).toHaveTextContent(/Windows Update/);
      expect(screen.getByRole("status")).toHaveTextContent(/DGP/);
    });

    it("points to OpenSC on Linux", () => {
      renderWithCatalog(<ReaderLine reader={{ kind: "unreadable" }} platform="linux" />);

      expect(screen.getByRole("status")).toHaveTextContent(/OpenSC/);
    });

    it("offers nothing on a platform it has no advice for", () => {
      renderWithCatalog(<ReaderLine reader={{ kind: "unreadable" }} platform="other" />);

      expect(screen.getByRole("status")).toHaveTextContent(
        /^rFirma no puede leer la tarjeta del lector$/,
      );
    });

    for (const kind of ["noCard", "reading", "dnieReady", "cardReady"] as const) {
      it(`offers nothing when the status is ${kind}`, () => {
        renderWithCatalog(<ReaderLine reader={{ kind }} platform="windows" />);

        expect(screen.getByRole("status")).not.toHaveTextContent(/Windows Update|OpenSC/);
      });
    }
  });
});
