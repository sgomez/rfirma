import { act, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { SedeWindow } from "./SedeWindow";
import { certificate, consentStage, scriptedErrand } from "./testing/fixtures/sedeWindow";

/** Grada A: la línea del lector y la lista en caliente en el consentimiento. */

const ada = certificate({ id: "ada", remembered: true });
const grace = certificate({
  id: "grace",
  holderName: "GRACE BREWSTER HOPPER",
  stampedSigner: "GRACE BREWSTER HOPPER",
  givenName: "GRACE",
  surname: "BREWSTER HOPPER",
  stores: ["dnie"],
});

function consentWith(...certificates: ReturnType<typeof certificate>[]) {
  const scripted = scriptedErrand(consentStage({ certificates }));
  renderWithCatalog(<SedeWindow errands={scripted.port} consentCountdown={false} />);
  return scripted;
}

describe("2 · consent, with the card readers", () => {
  it("draws no reader line while there is no reader", () => {
    consentWith(ada);

    expect(screen.queryByRole("status")).toBeNull();
  });

  it("says what the reader does under the selector", () => {
    const { announce } = consentWith(ada);

    act(() => announce({ reader: { kind: "noCard" }, certificates: null }));

    expect(screen.getByRole("status")).toHaveTextContent("Lector conectado, sin tarjeta");
    act(() => announce({ reader: { kind: "reading" }, certificates: null }));
    expect(screen.getByRole("status")).toHaveTextContent("Leyendo la tarjeta…");
  });

  it("lists the certificates of a card that arrives, and keeps the chosen one", async () => {
    const user = userEvent.setup();
    const { announce } = consentWith(ada);

    act(() => announce({ reader: { kind: "dnieReady" }, certificates: [ada, grace] }));

    expect(screen.getByRole("status")).toHaveTextContent("DNIe listo");
    expect(screen.getByRole("combobox", { name: "Certificado" })).toHaveTextContent(
      "ADA LOVELACE BYRON",
    );
    await user.click(screen.getByRole("combobox", { name: "Certificado" }));
    expect(screen.getByRole("option", { name: /GRACE BREWSTER HOPPER/ })).toBeInTheDocument();
  });

  it("leaves nothing chosen when the card of the chosen certificate leaves", async () => {
    const user = userEvent.setup();
    const { announce } = consentWith(grace, ada);
    await user.click(screen.getByRole("combobox", { name: "Certificado" }));
    await user.click(screen.getByRole("option", { name: /GRACE BREWSTER HOPPER/ }));

    act(() => announce({ reader: { kind: "noCard" }, certificates: [ada] }));

    expect(screen.getByRole("combobox", { name: "Certificado" })).not.toHaveTextContent(
      "GRACE BREWSTER HOPPER",
    );
    expect(screen.getByRole("button", { name: "Firmar" })).toBeDisabled();
  });

  it("chooses the remembered certificate that arrives when nothing is chosen", () => {
    const { announce } = consentWith(grace);
    act(() => announce({ reader: { kind: "noCard" }, certificates: [] }));

    act(() => announce({ reader: { kind: "cardReady" }, certificates: [ada] }));

    expect(screen.getByRole("combobox", { name: "Certificado" })).toHaveTextContent(
      "ADA LOVELACE BYRON",
    );
  });
});
