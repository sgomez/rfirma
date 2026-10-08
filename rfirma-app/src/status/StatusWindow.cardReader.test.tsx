import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { StatusWindow } from "./StatusWindow";
import { memoryStatus } from "./status";

function windowWith(reader?: React.ComponentProps<typeof StatusWindow>["reader"]) {
  return <StatusWindow statusPort={memoryStatus([])} reader={reader} onClose={() => {}} />;
}

describe("StatusWindow card reader row", () => {
  it("has no card reader row when the window is not told about readers", async () => {
    renderWithCatalog(windowWith());

    await waitFor(() => expect(screen.queryByText("Comprobando")).not.toBeInTheDocument());
    expect(screen.queryByText("Lector de tarjetas")).not.toBeInTheDocument();
  });

  it("follows the reader as it is plugged and unplugged, without Volver a comprobar", async () => {
    const { rerender } = renderWithCatalog(windowWith({ kind: "noReader" }));

    const row = await screen.findByText("Lector de tarjetas");
    expect(
      within(row.closest("[role=status]") as HTMLElement).getByText("No detectado"),
    ).toBeVisible();

    rerender(windowWith({ kind: "noCard" }));

    expect(await screen.findByText("Detectado")).toBeVisible();
    expect(screen.queryByText("No detectado")).not.toBeInTheDocument();
  });

  it("does not ask for attention when there is no reader", async () => {
    renderWithCatalog(windowWith({ kind: "noReader" }));

    const row = (await screen.findByText("Lector de tarjetas")).closest(
      "[role=status]",
    ) as HTMLElement;
    expect(within(row).queryByText("Atención")).not.toBeInTheDocument();
    expect(within(row).queryByText("Incorrecto")).not.toBeInTheDocument();
  });

  it("says not supported when this version cannot watch readers", async () => {
    renderWithCatalog(windowWith({ kind: "unavailable" }));

    expect(await screen.findByText("No soportado")).toBeVisible();
  });
});
