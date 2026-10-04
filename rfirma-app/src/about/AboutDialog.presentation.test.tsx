import { composeStories } from "@storybook/react-vite";
import { screen } from "@testing-library/react";
import type { ComponentType } from "react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as stories from "./AboutDialog.stories";

/** Presentación: lo que enseña cada historia de Acerca de, una fila por historia. */

const { UpToDate, NewVersionInstallable, NewVersionAnnouncedOnly, NewVersionWithoutOffer } =
  composeStories(stories);

const update = "Actualizar ahora";

const rows: { label: string; Story: ComponentType; shows: string[]; offersUpdate: boolean }[] = [
  {
    label: "up to date",
    Story: UpToDate,
    shows: ["Estás en la última versión"],
    offersUpdate: false,
  },
  {
    label: "an installable new version",
    Story: NewVersionInstallable,
    shows: ["Hay una versión nueva: 0.4.1"],
    offersUpdate: true,
  },
  {
    label: "a new version that is only announced",
    Story: NewVersionAnnouncedOnly,
    shows: ["Hay una versión nueva: 0.4.1"],
    offersUpdate: false,
  },
  {
    label: "a new version with notifications off",
    Story: NewVersionWithoutOffer,
    shows: ["Hay una versión nueva: 0.4.1"],
    offersUpdate: false,
  },
];

describe("the version status", () => {
  for (const { label, Story, shows, offersUpdate } of rows) {
    it(`${label} shows its status and ${offersUpdate ? "offers" : "does not offer"} to update`, async () => {
      renderWithCatalog(<Story />);

      for (const text of shows) expect(await screen.findByText(text)).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: update }) !== null).toBe(offersUpdate);
      expect(screen.queryByText(/flatpak install/)).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Copiar" })).not.toBeInTheDocument();
    });
  }
});

describe("the identity, the licences and the independence notice", () => {
  it("shows the version, both licences without unfolding anything, and the independence notice", () => {
    renderWithCatalog(<UpToDate />);

    expect(screen.getByText("Versión 0.4.0")).toBeInTheDocument();
    expect(screen.getByText("EUPL-1.2")).toBeInTheDocument();
    expect(screen.getByText("Cliente @firma")).toBeInTheDocument();
    expect(screen.getByText("GPL-2.0+ / EUPL-1.1")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Ver las licencias" })).not.toBeInTheDocument();
    expect(screen.getByText("Proyecto independiente")).toBeInTheDocument();
    const notice = screen.getByText(/no está relacionada con AutoFirma/);
    expect(notice).toHaveTextContent(/ni cuenta con su respaldo/);
    expect(notice).toHaveTextContent(
      /Avisa de sus fallos en nuestro repositorio, no al equipo de AutoFirma\./,
    );
  });

  it("does not narrate that the document and the private key stay on the computer", () => {
    renderWithCatalog(<UpToDate />);

    expect(screen.queryByText(/no salen de tu ordenador/)).not.toBeInTheDocument();
  });
});
