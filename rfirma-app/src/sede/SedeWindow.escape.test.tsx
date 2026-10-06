import { composeStories } from "@storybook/react-vite";
import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as confirmModule from "./SedeConfirm.stories";
import * as consentModule from "./SedeConsent.stories";
import * as markingModule from "./SedeMarking.stories";
import * as noCertificateModule from "./SedeNoCertificate.stories";
import * as outcomeModule from "./SedeOutcome.stories";
import * as signingModule from "./SedeSigning.stories";
import * as waitingModule from "./SedeWaiting.stories";
import { SedeWindow } from "./SedeWindow";
import { scriptedFrom } from "./testing/fixtures/sedeWindow";

/** Grada A: Escape es el botón de cancelar o cerrar de cada momento. */

const { OldWebClient, Waiting, Unreachable } = composeStories(waitingModule);
const { Consent } = composeStories(consentModule);
const { ShadowAttackSuspect } = composeStories(confirmModule);
const { Signing, Returning } = composeStories(signingModule);
const { NoneInstalled } = composeStories(noCertificateModule);
const { Signed } = composeStories(outcomeModule);
const { UnreadableDocument } = composeStories(markingModule);

const pressEscape = () => fireEvent.keyDown(document, { key: "Escape" });

describe("Escape", () => {
  it.each([
    ["waiting", Waiting],
    ["unreachable", Unreachable],
    ["consent", Consent],
    ["confirming", ShadowAttackSuspect],
    ["signing", Signing],
    ["noCertificate", NoneInstalled],
  ])("cancels the errand in %s", (_, story) => {
    const { port, calls } = scriptedFrom(story);
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEscape();

    expect(calls.cancel).toHaveBeenCalledOnce();
  });

  it("closes the outcome", () => {
    const { port, calls } = scriptedFrom(Signed);
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEscape();

    expect(calls.close).toHaveBeenCalledOnce();
  });

  it("marks no area when the site asked for one, like Cancelar", () => {
    const { port, calls } = scriptedFrom(UnreadableDocument);
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEscape();

    expect(calls.markArea).toHaveBeenCalledWith(null);
  });

  it("does nothing while the signature goes back to the site, where there is no Cancelar", () => {
    const { port, calls } = scriptedFrom(Returning);
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEscape();

    expect(calls.cancel).not.toHaveBeenCalled();
  });

  it("neither dismisses nor cancels the old web client warning, which has no Cancelar", () => {
    const { port, calls } = scriptedFrom(OldWebClient);
    renderWithCatalog(<SedeWindow errands={port} />);

    pressEscape();

    expect(calls.dismissWarning).not.toHaveBeenCalled();
    expect(calls.cancel).not.toHaveBeenCalled();
  });

  it("closes the open certificate list without cancelling the errand", () => {
    const { port, calls } = scriptedFrom(Consent);
    renderWithCatalog(<SedeWindow errands={port} consentCountdown={false} />);
    fireEvent.click(screen.getByRole("combobox"));

    fireEvent.keyDown(screen.getByRole("combobox"), { key: "Escape" });

    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(calls.cancel).not.toHaveBeenCalled();
  });
});
