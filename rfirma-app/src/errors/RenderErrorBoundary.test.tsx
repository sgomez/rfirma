import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { inMemoryExternalDestinationOpener } from "../desktop/externalDestination";
import { renderWithCatalog } from "../testing/render";
import { RenderErrorBoundary } from "./RenderErrorBoundary";

/**
 * Grada A: el *error boundary* de las dos ventanas, con un hijo que lanza al
 * pintarse. `SedeWindow.test.tsx` añade el caso propio de la ventana de sede,
 * que además no debe cerrarse sola.
 */

function Boom({ message }: { message: string }): never {
  throw new Error(message);
}

describe("RenderErrorBoundary", () => {
  // React registra el fallo en la consola además de pasarlo al boundary; no
  // es lo que la prueba comprueba, y lo llenaría de ruido.
  beforeEach(() => vi.spyOn(console, "error").mockImplementation(() => {}));
  afterEach(() => vi.restoreAllMocks());

  it("shows the failure screen instead of a blank window when a child throws", () => {
    renderWithCatalog(
      <RenderErrorBoundary>
        <Boom message="el árbol no se ha podido pintar" />
      </RenderErrorBoundary>,
    );

    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("el árbol no se ha podido pintar");
  });

  it("puts the focus on the failure screen", () => {
    renderWithCatalog(
      <RenderErrorBoundary>
        <Boom message="boom" />
      </RenderErrorBoundary>,
    );

    expect(screen.getByRole("alert")).toHaveFocus();
  });

  it("shows the help link, wired to the discussions destination", async () => {
    const user = userEvent.setup();
    const externalDestinations = inMemoryExternalDestinationOpener();
    renderWithCatalog(
      <RenderErrorBoundary externalDestinations={externalDestinations}>
        <Boom message="boom" />
      </RenderErrorBoundary>,
    );

    await user.click(screen.getByRole("button", { name: /Comentarios y ayuda/ }));

    expect(externalDestinations.opened).toEqual(["discussions"]);
  });

  it("reloads the window when the reload button is pressed", async () => {
    const user = userEvent.setup();
    const onReload = vi.fn();
    renderWithCatalog(
      <RenderErrorBoundary onReload={onReload}>
        <Boom message="boom" />
      </RenderErrorBoundary>,
    );

    await user.click(screen.getByRole("button", { name: /Recargar/ }));

    expect(onReload).toHaveBeenCalledOnce();
  });

  it("renders the children unchanged when nothing throws", () => {
    renderWithCatalog(
      <RenderErrorBoundary>
        <p>todo va bien</p>
      </RenderErrorBoundary>,
    );

    expect(screen.getByText("todo va bien")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
