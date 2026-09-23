import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { CHROME_LOCAL_NETWORK_SETTINGS, noErrand } from "./errand";
import { SedeWindow } from "./SedeWindow";
import { scriptedErrand } from "./sedeWindowFixtures";

/**
 * Grada A: la ventana de sede entera, **por su puerto** (TD-63). No hay
 * backend, no hay canal y no hay Tauri: un doble de `SiteErrandPort` que emite
 * los momentos, y las conductas se leen en la pantalla.
 *
 * Los momentos 2 en adelante tienen su propio fichero: `SedeWindow.consent.test.tsx`,
 * `SedeWindow.signing.test.tsx`, `SedeWindow.outcome.test.tsx` y
 * `SedeWindow.noCertificate.test.tsx`.
 */

describe("SedeWindow", () => {
  it("does not mount at all when no site has called", () => {
    renderWithCatalog(<SedeWindow errands={noErrand()} />);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  describe("the old web client warning", () => {
    it("says the page is out of date and that Got it lets it continue", () => {
      const { port } = scriptedErrand({ kind: "oldWebClient" });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Esta página está desactualizada")).toBeInTheDocument();
      expect(screen.getByText(/pulsa entendido para continuar/i)).toBeInTheDocument();
    });

    it("is dismissed with its only button, without cancelling or closing the errand", () => {
      const { port, calls } = scriptedErrand({ kind: "oldWebClient" });
      renderWithCatalog(<SedeWindow errands={port} />);

      fireEvent.click(screen.getByRole("button", { name: "Entendido" }));

      expect(calls.dismissWarning).toHaveBeenCalledOnce();
      expect(calls.cancel).not.toHaveBeenCalled();
      expect(calls.close).not.toHaveBeenCalled();
    });
  });

  describe("1 · waiting for the channel", () => {
    it("shows connecting when waiting for the browser", () => {
      const { port } = scriptedErrand({ kind: "waiting" });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("Conectando con la sede")).toBeInTheDocument();
    });

    it("crosses into «the request has not arrived» when published by the backend, and never closes", () => {
      const { port, calls } = scriptedErrand({ kind: "unreachable" });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText("La petición no ha llegado")).toBeInTheDocument();
      expect(calls.close).not.toHaveBeenCalled();
      expect(calls.cancel).not.toHaveBeenCalled();
    });

    it("offers two recipes and never diagnoses which one is the problem", () => {
      const { port } = scriptedErrand({ kind: "unreachable" });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByRole("button", { name: "Chrome" })).toHaveAttribute(
        "aria-pressed",
        "true",
      );
      expect(screen.getByRole("button", { name: "Firefox" })).toHaveAttribute(
        "aria-pressed",
        "false",
      );
      expect(screen.getByText(/franja bajo la barra de direcciones/)).toBeInTheDocument();
    });

    it("gives the local CA the screen's only main action, because nothing works without it", () => {
      const { port, calls } = scriptedErrand({ kind: "unreachable" });
      renderWithCatalog(<SedeWindow errands={port} />);

      const install = screen.getByRole("button", { name: "Instalar…" });
      expect(install).toHaveClass("rf-btn--primary");
      fireEvent.click(install);
      expect(calls.installLocalCa).toHaveBeenCalledOnce();
    });

    it("puts the mandatory sentence in the footer, and has no Retry button of its own", () => {
      const { port } = scriptedErrand({ kind: "unreachable" });
      renderWithCatalog(<SedeWindow errands={port} />);

      expect(screen.getByText(/vuelve a la sede y pulsa Reintentar/)).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Reintentar" })).not.toBeInTheDocument();
    });

    it("abandons the errand when closed while waiting, with no confirmation", () => {
      const { port, calls } = scriptedErrand({ kind: "waiting" });
      renderWithCatalog(<SedeWindow errands={port} />);

      fireEvent.click(screen.getByRole("button", { name: "Cancelar" }));

      expect(calls.cancel).toHaveBeenCalledOnce();
    });
  });

  describe("1b · the channel that will never open", () => {
    it("shows the repair screen straight away, without waiting for the threshold", () => {
      const { port } = scriptedErrand({ kind: "noChannel", reason: "channelNotOpened" });
      renderWithCatalog(<SedeWindow errands={port} />);

      // Sin relojes falsos y sin adelantar nada: el backend ya sabe que no hay
      // canal, así que «Conectando» sería mentira desde el primer píxel.
      expect(screen.getByText("La petición no ha llegado")).toBeInTheDocument();
    });

    it("gives Chrome's local-network address to copy, and never as something to click", () => {
      const { port } = scriptedErrand({ kind: "noChannel", reason: "localCaMissing" });
      renderWithCatalog(<SedeWindow errands={port} />);

      const address = screen.getByText(CHROME_LOCAL_NETWORK_SETTINGS);
      expect(address).toBeInTheDocument();
      expect(address.closest("a")).toBeNull();
      expect(screen.getByRole("button", { name: /Copiar/ })).toBeInTheDocument();
    });

    it("abandons the errand when closed: nothing has been answered", () => {
      const { port, calls } = scriptedErrand({ kind: "noChannel", reason: "channelNotOpened" });
      renderWithCatalog(<SedeWindow errands={port} />);

      fireEvent.click(screen.getByRole("button", { name: "Cerrar" }));

      expect(calls.cancel).toHaveBeenCalledOnce();
      expect(calls.close).not.toHaveBeenCalled();
    });
  });
});
