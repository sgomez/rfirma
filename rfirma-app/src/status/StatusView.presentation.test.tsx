import { composeStories } from "@storybook/react-vite";
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as storyModule from "./StatusView.stories";

const stories = composeStories(storyModule);

interface Expected {
  texts: string[];
  buttons: string[];
  absent?: string[];
  combobox?: string;
}

const table: Record<keyof typeof stories, Expected> = {
  EverythingCorrect: {
    texts: [
      "Versión",
      "0.4.1",
      "Aplicación predeterminada para firmar en sedes",
      "rFirma",
      "5 certificados",
      "2 de 2 navegadores",
    ],
    buttons: ["Retirar…", "Ver navegadores", "Ver dónde"],
    absent: ["Atención", "Incorrecto"],
  },
  SomethingToRepair: {
    texts: ["0.4.1 → 0.5.0", "Sin configurar", "0 de 2 navegadores", "Ninguno", "Incorrecto"],
    buttons: ["Actualizar", "Usar rFirma", "Instalar", "Ver navegadores", "Cómo instalar"],
  },
  Checking: {
    texts: ["Comprobando"],
    buttons: [],
  },
  VersionUpToDate: { texts: ["Versión", "0.4.1", "Correcto"], buttons: [] },
  VersionOutdated: {
    texts: ["0.4.1 → 0.5.0", "Atención"],
    buttons: ["Actualizar"],
  },
  SitesHandledByRfirma: {
    texts: ["Aplicación predeterminada para firmar en sedes", "rFirma", "Correcto"],
    buttons: [],
  },
  SitesNotConfigured: {
    texts: ["Sin configurar", "Atención"],
    buttons: ["Usar rFirma"],
  },
  SitesUnavailable: {
    texts: ["Aplicación predeterminada para firmar en sedes", "Gestionada por tu escritorio"],
    buttons: ["Diagnosticar problemas"],
    absent: ["No aplica"],
  },
  SitesUnavailableDiagnosisExpanded: {
    texts: [
      "Si al firmar en una sede se abre otra aplicación sin preguntar",
      "xdg-mime query default x-scheme-handler/afirma",
      "Si no es me.sgomez.rfirma.desktop, cámbiala:",
      "xdg-mime default me.sgomez.rfirma.desktop x-scheme-handler/afirma",
    ],
    buttons: ["Diagnosticar problemas"],
  },
  SitesUnavailableOutsideFlatpak: {
    texts: ["Gestionada por tu escritorio"],
    buttons: [],
    absent: ["Diagnosticar problemas", "xdg-mime query default x-scheme-handler/afirma"],
  },
  SitesWithTwoCandidates: {
    texts: [],
    buttons: ["Usar rFirma"],
    combobox: "AutoFirma",
  },
  CertificateMissing: {
    texts: ["Certificado de rFirma", "0 de 2 navegadores", "Incorrecto"],
    buttons: ["Instalar", "Ver navegadores"],
    absent: ["Firefox"],
  },
  CertificateHalfInstalled: {
    texts: ["1 de 2 navegadores", "Atención"],
    buttons: ["Instalar", "Ver navegadores"],
  },
  CertificateInstalled: {
    texts: ["2 de 2 navegadores", "Correcto"],
    buttons: ["Retirar…", "Ver navegadores"],
    absent: ["Instalar"],
  },
  CertificateInstalledNeedsFirefoxRestart: {
    texts: ["Reinicia Firefox para que surta efecto."],
    buttons: ["Retirar…", "Ver navegadores"],
  },
  CertificateDetailExpanded: {
    texts: ["Firefox", "Chrome y Chromium", "Instalado", "No instalado"],
    buttons: ["Instalar", "Ver navegadores"],
  },
  NoUserCertificates: {
    texts: ["Tus certificados", "Ninguno", "Atención"],
    buttons: ["Cómo instalar"],
  },
  SomeUserCertificates: {
    texts: ["5 certificados", "Correcto"],
    buttons: ["Ver dónde"],
  },
  UserCertificatesDetailExpanded: {
    texts: ["Almacén de Windows", "Firefox", "Tarjeta", "Fichero instalado"],
    buttons: ["Ver dónde"],
    absent: ["Instalado", "No instalado"],
  },
};

describe("StatusView stories", () => {
  for (const [name, expected] of Object.entries(table)) {
    it(`${name} shows what the person must read`, async () => {
      const Story = stories[name as keyof typeof stories];
      renderWithCatalog(<Story />);

      await waitFor(() => {
        const rows = screen.getAllByRole("status");
        const text = rows.map((row) => row.textContent).join("|");
        for (const wanted of expected.texts) {
          expect(text).toContain(wanted);
        }
        for (const unwanted of expected.absent ?? []) {
          expect(rows.flatMap((row) => within(row).queryAllByText(unwanted))).toEqual([]);
        }
        const buttons = rows.flatMap((row) =>
          within(row)
            .queryAllByRole("button")
            .map((button) => button.textContent?.trim()),
        );
        expect(buttons).toEqual(expected.buttons);
        if (expected.combobox !== undefined) {
          expect(within(rows[0] as HTMLElement).getByRole("combobox")).toHaveTextContent(
            expected.combobox,
          );
        }
      });
    });
  }

  it("keeps the browsers behind a toggle that reports its state", async () => {
    const user = userEvent.setup();
    const { CertificateMissing } = stories;
    renderWithCatalog(<CertificateMissing />);

    const toggle = await screen.findByRole("button", { name: "Ver navegadores" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");

    await user.click(toggle);

    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Chrome y Chromium")).toBeInTheDocument();
  });

  it("keeps the site signature diagnosis behind a toggle that reports its state", async () => {
    const user = userEvent.setup();
    const { SitesUnavailable } = stories;
    renderWithCatalog(<SitesUnavailable />);

    const toggle = await screen.findByRole("button", { name: "Diagnosticar problemas" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText("xdg-mime query default x-scheme-handler/afirma")).toBeNull();

    await user.click(toggle);

    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("xdg-mime query default x-scheme-handler/afirma")).toBeInTheDocument();
  });

  it("closes from the footer with Cerrar, and the signal table has no column header", async () => {
    const { EverythingCorrect } = stories;
    renderWithCatalog(<EverythingCorrect />);

    expect(await screen.findByRole("button", { name: "Cerrar" })).toBeInTheDocument();
    expect(screen.queryByText("Señal")).not.toBeInTheDocument();
    expect(screen.queryByText("Veredicto")).not.toBeInTheDocument();
  });
});
