import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { TrustNotice } from "./TrustNotice";

// Grada A: el aviso no habla con nada, solo se muestra y se descarta.
describe("TrustNotice", () => {
  it("explains the local CA and the local network permission together, unconditionally", () => {
    renderWithCatalog(<TrustNotice />);

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText(/entidad de confianza local/i)).toBeInTheDocument();
    expect(screen.getByText(/permiso para acceder a la red local/i)).toBeInTheDocument();
  });

  it("says one grant covers the three ports the site may try", () => {
    renderWithCatalog(<TrustNotice />);

    expect(screen.getByText(/con un solo permiso basta/i)).toBeInTheDocument();
  });

  it("never promises to diagnose a denial: it says it cannot tell", () => {
    renderWithCatalog(<TrustNotice />);

    expect(screen.getByText(/rfirma no podrá saberlo ni decírtelo/i)).toBeInTheDocument();
  });

  it("dismisses on acknowledgement and does not come back", async () => {
    const user = userEvent.setup();
    renderWithCatalog(<TrustNotice />);

    await user.click(screen.getByRole("button", { name: "Entendido" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
