import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { TrustNotice } from "./TrustNotice";

// Grada A: el aviso no habla con nada, solo se muestra y se descarta.
describe("TrustNotice", () => {
  it("explains the local CA and the local network permission together, unconditionally", () => {
    renderWithCatalog(<TrustNotice seen={false} onAcknowledge={() => {}} />);

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText(/entidad de confianza local/i)).toBeInTheDocument();
    expect(screen.getByText(/permiso para acceder a la red local/i)).toBeInTheDocument();
  });

  it("says one grant covers the three ports the site may try", () => {
    renderWithCatalog(<TrustNotice seen={false} onAcknowledge={() => {}} />);

    expect(screen.getByText(/con un solo permiso basta/i)).toBeInTheDocument();
  });

  it("never promises to diagnose a denial: it says it cannot tell", () => {
    renderWithCatalog(<TrustNotice seen={false} onAcknowledge={() => {}} />);

    expect(screen.getByText(/rfirma no podrá saberlo ni decírtelo/i)).toBeInTheDocument();
  });

  it("dismisses on acknowledgement and persists it so it does not come back", async () => {
    const user = userEvent.setup();
    const onAcknowledge = vi.fn();
    renderWithCatalog(<TrustNotice seen={false} onAcknowledge={onAcknowledge} />);

    await user.click(screen.getByRole("button", { name: "Entendido" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(onAcknowledge).toHaveBeenCalledOnce();
  });

  it("does not mount at all once a previous run has seen it", () => {
    renderWithCatalog(<TrustNotice seen={true} onAcknowledge={() => {}} />);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
