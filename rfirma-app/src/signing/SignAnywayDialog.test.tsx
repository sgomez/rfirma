import { composeStories } from "@storybook/react-vite";
import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { fn } from "storybook/test";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as stories from "./SignAnywayDialog.stories";

const { FindingAndSignatures, OnlyExpired, UnrecognizedFormat } = composeStories(stories);

const TITLE = "¿Firmar de todos modos?";

describe("SignAnywayDialog", () => {
  it("shows an expired signature as a row with its reason, and the receiver warning", () => {
    renderWithCatalog(<OnlyExpired />);

    const dialog = screen.getByRole("dialog", { name: TITLE });
    expect(within(dialog).getByText("Firma 2 · LUIS PEREZ")).toBeInTheDocument();
    expect(within(dialog).getByText(/^El certificado caducó el .*2026$/)).toBeInTheDocument();
    expect(within(dialog).getByText("El receptor podría rechazarlo.")).toBeInTheDocument();
  });

  it("lists one row per problem, findings first, with no count and no valid signature", () => {
    renderWithCatalog(<FindingAndSignatures />);

    const dialog = screen.getByRole("dialog", { name: TITLE });
    const rows = within(dialog).getAllByRole("listitem");
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent("Se ha modificado después de la última firma");
    expect(rows[1]).toHaveTextContent("Firma 2 · LUIS PEREZ");
    expect(rows[2]).toHaveTextContent("Firma 3 · MARTA RUIZ");
    expect(within(dialog).queryByText(/ADA LOVELACE/)).toBeNull();
    expect(within(dialog).queryByText(/firmas que no son válidas/)).toBeNull();
  });

  it("shows a finding as a row without a second line", () => {
    renderWithCatalog(<FindingAndSignatures />);

    const [finding] = within(screen.getByRole("dialog", { name: TITLE })).getAllByRole("listitem");
    expect(finding?.querySelectorAll("div")).toHaveLength(1);
  });

  it("says that rFirma does not know a signature of an unknown type", () => {
    renderWithCatalog(<UnrecognizedFormat />);

    const dialog = screen.getByRole("dialog", { name: TITLE });
    expect(within(dialog).getByText("Firma 2 · NOTARIA XYZ")).toBeInTheDocument();
    expect(within(dialog).getByText("rFirma no conoce este tipo de firma")).toBeInTheDocument();
  });

  it("signs anyway on confirm, and cancels without signing", async () => {
    const user = userEvent.setup();
    const onConfirm = fn();
    const onCancel = fn();
    renderWithCatalog(<FindingAndSignatures onConfirm={onConfirm} onCancel={onCancel} />);

    await user.click(screen.getByRole("button", { name: "Firmar de todos modos" }));
    expect(onConfirm).toHaveBeenCalledOnce();
    expect(onCancel).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("signs anyway with Intro, with the focus already on it, and cancels with Escape", async () => {
    const user = userEvent.setup();
    const onConfirm = fn();
    const onCancel = fn();
    renderWithCatalog(<FindingAndSignatures onConfirm={onConfirm} onCancel={onCancel} />);

    expect(screen.getByRole("button", { name: "Firmar de todos modos" })).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(onConfirm).toHaveBeenCalledOnce();
    expect(onCancel).not.toHaveBeenCalled();

    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalledOnce();
  });
});
