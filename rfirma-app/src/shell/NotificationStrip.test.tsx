import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { NotificationStrip } from "./NotificationStrip";

// Grada A. Lo que se comprueba aquí es que la franja es **el patrón**
// (ID-207): recibe su frase y su acción de fuera, así que no sabe nada del
// aviso de versión, que es sólo su primer inquilino.
describe("NotificationStrip", () => {
  it("shows the sentence it is given, whatever the notification is", () => {
    renderWithCatalog(
      <NotificationStrip
        message="Se ha quedado algo a medias"
        dismissLabel="Descartar"
        onDismiss={() => {}}
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent("Se ha quedado algo a medias");
  });

  it("carries at most one action", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    renderWithCatalog(
      <NotificationStrip
        message="Hay algo que contar"
        action={{ label: "Cómo se arregla", onSelect }}
        dismissLabel="Descartar"
        onDismiss={() => {}}
      />,
    );

    const strip = screen.getByRole("status");
    // Los dos botones de la franja: la acción y la `×`. Ni uno más.
    expect(screen.getAllByRole("button")).toHaveLength(2);

    await user.click(screen.getByRole("button", { name: "Cómo se arregla" }));
    expect(onSelect).toHaveBeenCalledOnce();
    expect(strip).toBeInTheDocument();
  });

  it("can be dismissed", async () => {
    const user = userEvent.setup();
    const onDismiss = vi.fn();
    renderWithCatalog(
      <NotificationStrip
        message="Hay algo que contar"
        dismissLabel="Descartar el aviso"
        onDismiss={onDismiss}
      />,
    );

    await user.click(screen.getByRole("button", { name: "Descartar el aviso" }));
    expect(onDismiss).toHaveBeenCalledOnce();
  });

  // Una notificación sin nada que hacer sigue siendo una notificación: la
  // acción es opcional y la `×` no.
  it("works without an action", () => {
    renderWithCatalog(
      <NotificationStrip
        message="Hay algo que contar"
        dismissLabel="Descartar"
        onDismiss={() => {}}
      />,
    );

    expect(screen.getAllByRole("button")).toHaveLength(1);
  });
});
