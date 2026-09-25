import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import type { ErrandStage } from "./errand";
import { SedeWindow } from "./SedeWindow";
import { certificate, scriptedErrand, signedDocument } from "./sedeWindowFixtures";

/** Grada A: Escape es el botón de cancelar o cerrar de cada momento. */

const consent: ErrandStage = {
  kind: "consent",
  document: null,
  signs: null,
  signing: null,
  items: null,
  certificates: [certificate()],
  narrowed: false,
};

const pressEscape = () => fireEvent.keyDown(document, { key: "Escape" });

describe("Escape", () => {
  it.each<[string, ErrandStage]>([
    ["waiting", { kind: "waiting" }],
    ["unreachable", { kind: "unreachable" }],
    ["consent", consent],
    ["confirming", { kind: "confirming", messageCode: "pdfShadowAttackSuspect" }],
    ["signing", { kind: "signing", certificate: certificate(), phase: "signing" }],
    ["noCertificate", { kind: "noCertificate", reason: "none", owned: 0 }],
  ])("cancels the errand in %s", (_, stage) => {
    const { port, calls } = scriptedErrand(stage);
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEscape();

    expect(calls.cancel).toHaveBeenCalledOnce();
  });

  it("closes the outcome", () => {
    const { port, calls } = scriptedErrand({
      kind: "outcome",
      outcome: { kind: "signed", document: signedDocument },
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEscape();

    expect(calls.close).toHaveBeenCalledOnce();
  });

  it("marks no area when the site asked for one, like Cancelar", () => {
    const { port, calls } = scriptedErrand({ kind: "marking", pdf: null });
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEscape();

    expect(calls.markArea).toHaveBeenCalledWith(null);
  });

  it("does nothing while the signature goes back to the site, where there is no Cancelar", () => {
    const { port, calls } = scriptedErrand({
      kind: "signing",
      certificate: certificate(),
      phase: "returning",
    });
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEscape();

    expect(calls.cancel).not.toHaveBeenCalled();
  });

  it("neither dismisses nor cancels the old web client warning, which has no Cancelar", () => {
    const { port, calls } = scriptedErrand({ kind: "oldWebClient" });
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEscape();

    expect(calls.dismissWarning).not.toHaveBeenCalled();
    expect(calls.cancel).not.toHaveBeenCalled();
  });

  it("closes the open certificate list without cancelling the errand", () => {
    const { port, calls } = scriptedErrand(consent);
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);
    fireEvent.click(screen.getByRole("combobox"));

    fireEvent.keyDown(screen.getByRole("listbox"), { key: "Escape" });

    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(calls.cancel).not.toHaveBeenCalled();
  });
});
