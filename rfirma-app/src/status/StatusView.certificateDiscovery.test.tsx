import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { StatusView } from "./StatusView";
import { memoryStatus, type SignalRow } from "./status";

describe("StatusView", () => {
  it("renders Ninguno and Cómo instalar when no certificate is found", async () => {
    const rows: SignalRow[] = [
      {
        signal: "userCertificates",
        value: "0",
        verdict: "attention",
        action: {
          kind: "link",
          target: "certificateIssuance",
        },
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Certificados de firma electrónica")).toBeInTheDocument();
    expect(within(row).getByText("Ninguno")).toBeInTheDocument();
    expect(within(row).getByText("Atención")).toBeInTheDocument();
    expect(within(row).getByRole("button", { name: "Cómo instalar" })).toBeInTheDocument();
  });

  it("renders how many certificates were found, and Correcto without action", async () => {
    const rows: SignalRow[] = [
      {
        signal: "userCertificates",
        value: "3",
        verdict: "correct",
        action: null,
        detail: null,
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("Certificados de firma electrónica")).toBeInTheDocument();
    expect(within(row).getByText("3 certificados")).toBeInTheDocument();
    expect(within(row).getByText("Correcto")).toBeInTheDocument();
    expect(within(row).queryByRole("button")).not.toBeInTheDocument();
  });

  it("lists each place with its own count behind Ver dónde", async () => {
    const rows: SignalRow[] = [
      {
        signal: "userCertificates",
        value: "4",
        verdict: "correct",
        action: null,
        detail: {
          kind: "certificates",
          stores: [
            { brand: "firefox", certificates: 2 },
            { brand: "card", certificates: 1 },
            { brand: "installed", certificates: 1 },
          ],
        },
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    const user = userEvent.setup();
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    expect(within(row).getByText("4 certificados")).toBeInTheDocument();

    await user.click(within(row).getByRole("button", { name: "Ver dónde" }));

    const places = within(row).getAllByRole("listitem");
    expect(places.map((place) => place.textContent)).toEqual([
      "Firefox2",
      "Tarjeta1",
      "Fichero instalado1",
    ]);
  });

  it("marks nothing in the list of places: nothing was attempted there", async () => {
    const rows: SignalRow[] = [
      {
        signal: "userCertificates",
        value: "2",
        verdict: "correct",
        action: null,
        detail: { kind: "certificates", stores: [{ brand: "firefox", certificates: 2 }] },
        candidates: null,
        restartFirefoxNotice: false,
      },
    ];
    const user = userEvent.setup();
    renderWithCatalog(<StatusView statusPort={memoryStatus(rows)} onClose={() => {}} />);

    const row = await screen.findByRole("status");
    await user.click(within(row).getByRole("button", { name: "Ver dónde" }));

    expect(within(row).queryByText("Instalado")).not.toBeInTheDocument();
    expect(within(row).queryByText("No instalado")).not.toBeInTheDocument();
  });
});
