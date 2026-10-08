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
});
