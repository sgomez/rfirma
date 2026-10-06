import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { PreviousSignaturesNotice } from "./PreviousSignaturesNotice";
import { certificate, previousSignatureOf, reportOf } from "./testing/harness";

const CLOSED = "El documento no admite más firmas.";

describe("PreviousSignaturesNotice, con un documento cerrado", () => {
  it("says so in the panel", () => {
    renderWithCatalog(
      <PreviousSignaturesNotice
        report={reportOf([previousSignatureOf({ closesDocument: true })], { closed: true })}
        certificate={certificate}
      />,
    );

    expect(screen.getByText(CLOSED)).toBeInTheDocument();
  });

  it("leaves it out of the site, where the request may allow signing it", () => {
    renderWithCatalog(
      <PreviousSignaturesNotice
        report={reportOf([previousSignatureOf({ closesDocument: true })], { closed: true })}
        certificate={certificate}
        presentation="site"
      />,
    );

    expect(screen.queryByText(CLOSED)).toBeNull();
  });
});
