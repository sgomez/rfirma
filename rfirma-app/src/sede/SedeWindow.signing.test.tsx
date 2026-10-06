import { composeStories } from "@storybook/react-vite";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as confirmModule from "./SedeConfirm.stories";
import * as signingModule from "./SedeSigning.stories";
import { SedeWindow } from "./SedeWindow";
import { scriptedFrom } from "./testing/fixtures/sedeWindow";

/** Grada A: el momento 2b (la confirmación que exige el validador) y el 3 (la firma), por su puerto. */

const { ShadowAttackSuspect } = composeStories(confirmModule);
const { Signing, Returning } = composeStories(signingModule);

describe("2b · confirming what the validator flags", () => {
  it("focuses Continuar, so Enter goes on: the person has not consented yet", () => {
    const { port } = scriptedFrom(ShadowAttackSuspect);
    renderWithCatalog(<SedeWindow errands={port} />);

    expect(screen.getByRole("button", { name: "Continuar" })).toHaveFocus();
  });

  it("goes on with the signature when the person confirms", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedFrom(ShadowAttackSuspect);
    renderWithCatalog(<SedeWindow errands={port} />);

    await user.click(screen.getByRole("button", { name: "Continuar" }));

    expect(calls.confirmSignatures).toHaveBeenCalled();
    expect(calls.cancel).not.toHaveBeenCalled();
  });

  it("hands the confirmation on only once, however many times the button is pressed", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedFrom(ShadowAttackSuspect);
    calls.confirmSignatures.mockReturnValue(new Promise(() => {}));
    renderWithCatalog(<SedeWindow errands={port} />);
    const going = screen.getByRole("button", { name: "Continuar" });

    await user.click(going);
    await user.click(going);

    expect(calls.confirmSignatures).toHaveBeenCalledOnce();
    expect(going).toBeDisabled();
  });

  it("abandons the errand when the person refuses, which is what the site gets", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedFrom(ShadowAttackSuspect);
    renderWithCatalog(<SedeWindow errands={port} />);

    await user.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(calls.cancel).toHaveBeenCalled();
    expect(calls.confirmSignatures).not.toHaveBeenCalled();
  });
});

describe("3 · signing", () => {
  it("can still be cancelled while rFirma signs: the site has received nothing", async () => {
    const user = userEvent.setup();
    const { port, calls } = scriptedFrom(Signing);
    renderWithCatalog(<SedeWindow errands={port} />);

    await user.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(calls.cancel).toHaveBeenCalledOnce();
  });

  it("moves the bar between the two moments so they do not look the same", () => {
    const { port } = scriptedFrom(Returning);
    const { rerender } = renderWithCatalog(<SedeWindow errands={port} />);
    const returning = screen.getByRole("progressbar").getAttribute("aria-valuenow");

    rerender(<SedeWindow errands={scriptedFrom(Signing).port} />);

    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).not.toBe(returning);
  });
});
