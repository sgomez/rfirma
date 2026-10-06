import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { StatusWindow } from "./StatusWindow";
import { memoryStatus, type SignalRow, type StatusPort } from "./status";
import { caMissing, sitesWithTwoCandidates, versionUpToDate } from "./testing/fixtures";

const rows: SignalRow[] = [versionUpToDate, sitesWithTwoCandidates, caMissing];

function rejecting(override: Partial<StatusPort>): StatusPort {
  const port = memoryStatus(rows, undefined, undefined, undefined, undefined, caMissing);
  return { ...port, ...override };
}

function rejectWith(situation: string) {
  return () => Promise.reject({ situation, detail: `raw ${situation}` });
}

describe("StatusWindow when an action fails", () => {
  it.each([
    ["handlerNotAvailable", "No se puede cambiar quién atiende los enlaces"],
    ["handlerListUnreadable", "Algo ha fallado"],
    ["handlerListUnwritable", "Algo ha fallado"],
  ])("tells %s when choosing the handler and restores the rows", async (situation, title) => {
    const user = userEvent.setup();
    renderWithCatalog(
      <StatusWindow
        statusPort={rejecting({ chooseSiteSignatureHandler: rejectWith(situation) })}
        onClose={() => {}}
      />,
    );

    await user.click(await screen.findByRole("combobox"));
    await user.click(await screen.findByRole("option", { name: "rFirma" }));

    const alert = await screen.findByRole("alert");
    expect(within(alert).getByText(title)).toBeInTheDocument();
    expect(within(alert).getByText(`raw ${situation}`)).toBeInTheDocument();
    expect(screen.queryByText("Comprobando")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Instalar" })).toBeInTheDocument();
    expect(screen.getByRole("combobox")).toBeInTheDocument();
  });

  it("tells the failure of rechecking, restores the rows and re-enables the button", async () => {
    const user = userEvent.setup();
    renderWithCatalog(
      <StatusWindow
        statusPort={rejecting({ recheck: rejectWith("bridgeFailed") })}
        onClose={() => {}}
      />,
    );

    await user.click(await screen.findByRole("button", { name: "Volver a comprobar" }));

    expect(await screen.findByRole("alert")).toBeInTheDocument();
    expect(screen.queryByText("Comprobando")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Instalar" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Volver a comprobar" })).toBeEnabled();
  });

  it("tells the failure of repairing the local CA and offers the repair again", async () => {
    const user = userEvent.setup();
    renderWithCatalog(
      <StatusWindow
        statusPort={rejecting({ installLocalCaCertificate: rejectWith("bridgeFailed") })}
        onClose={() => {}}
      />,
    );

    await user.click(await screen.findByRole("button", { name: "Instalar" }));

    expect(await screen.findByRole("alert")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Instalar" })).toBeInTheDocument();
    expect(screen.queryByText("Comprobando")).not.toBeInTheDocument();
  });

  it("removes the notice when the action is tried again and succeeds", async () => {
    const user = userEvent.setup();
    let attempts = 0;
    renderWithCatalog(
      <StatusWindow
        statusPort={rejecting({
          installLocalCaCertificate: () => {
            attempts += 1;
            return attempts === 1
              ? rejectWith("bridgeFailed")()
              : Promise.resolve({ ...caMissing, verdict: "correct", action: null });
          },
        })}
        onClose={() => {}}
      />,
    );

    await user.click(await screen.findByRole("button", { name: "Instalar" }));
    await screen.findByRole("alert");
    await user.click(screen.getByRole("button", { name: "Instalar" }));

    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
  });
});
