import { composeStories } from "@storybook/react-vite";
import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as stories from "./PreferencesView.stories";

/** Presentación: lo que enseña cada historia de Preferencias, una fila por historia. */

const {
  General,
  GeneralWithActivityOff,
  SigningWithoutOriginalFolder,
  SigningNextToTheOriginal,
  SigningInTheDestinationFolder,
  NoCertificates,
  CertificatesInstalled,
  Appearance,
} = composeStories(stories);

const panelOf = (name: string) => within(screen.getByRole("tabpanel", { name }));

describe("the index", () => {
  it("lists the four sections with a permanent index, opens on General and shows one panel", () => {
    renderWithCatalog(<General />);

    const index = screen.getByRole("navigation", { name: "Secciones" });
    expect(
      within(index)
        .getAllByRole("tab")
        .map((tab) => tab.textContent),
    ).toEqual(["General", "Firma", "Certificados", "Apariencia"]);
    expect(screen.getByRole("tab", { name: "General" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getAllByRole("tabpanel")).toHaveLength(1);
  });

  it("keeps Cerrar in a footer outside the column that scrolls", () => {
    const { container } = renderWithCatalog(<General />);

    const close = screen.getByRole("button", { name: "Cerrar" });
    expect(close.closest(".preferences__footer")).not.toBeNull();
    expect(container.querySelector(".preferences__content")?.contains(close)).toBe(false);
  });
});

describe("General", () => {
  it("groups the privacy settings, with the switches in their wide spacing and no Save", () => {
    renderWithCatalog(<General />);

    const privacy = panelOf("General").getByRole("group", { name: "Privacidad" });
    const remember = within(privacy).getByRole("switch", { name: /Recordar mi actividad/ });
    expect(remember).toBeChecked();
    expect(remember.closest(".switch")).toHaveClass("switch--wide");
    expect(within(privacy).getByRole("button", { name: "Vaciar la lista" })).toBeInTheDocument();
    expect(
      within(privacy).getByRole("switch", { name: /Avisar de versiones nuevas/ }),
    ).toBeChecked();
    expect(screen.queryByRole("button", { name: "Guardar" })).not.toBeInTheDocument();
  });

  it("shows both switches off when the preferences say so", () => {
    renderWithCatalog(<GeneralWithActivityOff />);

    expect(screen.getByRole("switch", { name: /Recordar mi actividad/ })).not.toBeChecked();
    expect(screen.getByRole("switch", { name: /Avisar de versiones nuevas/ })).not.toBeChecked();
  });
});

describe("Firma", () => {
  const switches = [
    [/Recordar la firma visible/, true],
    [/Esperar 3 segundos antes de firmar/, true],
    [/Si solo sirve un certificado y la sede lo permite/, false],
  ] as const;

  it("shows the folder by its name and never by its path, as a button and not as a dropdown", () => {
    renderWithCatalog(<SigningWithoutOriginalFolder />);

    const signing = panelOf("Firma");
    expect(signing.getByText("Dónde guardar")).toBeInTheDocument();
    expect(signing.getByText("Documentos")).toBeInTheDocument();
    expect(signing.getByRole("button", { name: "Cambiar carpeta…" })).toBeInTheDocument();
    expect(signing.queryByRole("combobox")).not.toBeInTheDocument();
    expect(signing.queryByRole("radiogroup")).not.toBeInTheDocument();
    expect(signing.queryByText(/\/home\//)).not.toBeInTheDocument();
  });

  it("shows the three switches in their wide spacing, with no hint under them", () => {
    renderWithCatalog(<SigningWithoutOriginalFolder />);

    for (const [name, checked] of switches) {
      const control = screen.getByRole("switch", { name });
      expect(control.closest(".switch")).toHaveClass("switch--wide");
      expect(control.getAttribute("aria-checked")).toBe(String(checked));
    }
    expect(screen.queryByText(/pausa de 3 segundos/)).not.toBeInTheDocument();
    expect(screen.queryByText(/se usa sin preguntarte/)).not.toBeInTheDocument();
    expect(screen.queryByText(/se reutilizan en el siguiente documento/)).not.toBeInTheDocument();
    expect(
      screen.queryByRole("switch", { name: /Protección contra firmas/ }),
    ).not.toBeInTheDocument();
  });

  const modes = [
    { Story: SigningNextToTheOriginal, checked: "Junto al original", unchecked: "En esta carpeta" },
    {
      Story: SigningInTheDestinationFolder,
      checked: "En esta carpeta",
      unchecked: "Junto al original",
    },
  ];

  for (const { Story, checked, unchecked } of modes) {
    it(`offers the two destination modes as a radio group, with ${checked} chosen`, () => {
      renderWithCatalog(<Story />);

      const group = screen.getByRole("radiogroup", { name: "Dónde guardar" });
      expect(within(group).getByRole("radio", { name: checked })).toBeChecked();
      expect(within(group).getByRole("radio", { name: unchecked })).not.toBeChecked();
      expect(screen.getByRole("button", { name: "Cambiar carpeta…" })).toBeInTheDocument();
    });
  }
});

describe("Certificados", () => {
  it("says nothing is installed yet and offers only to add one", () => {
    renderWithCatalog(<NoCertificates />);

    const certificates = panelOf("Certificados");
    expect(certificates.getByText("Todavía no has instalado ninguno")).toBeInTheDocument();
    expect(certificates.getByRole("button", { name: "Añadir…" })).toBeInTheDocument();
    expect(certificates.queryAllByRole("button", { name: /Quitar/ })).toHaveLength(0);
  });

  it("lists each one by its holder or entity, never by its file, with the two gestures and nothing else", () => {
    renderWithCatalog(<CertificatesInstalled />);

    const certificates = panelOf("Certificados");
    expect(certificates.getByText("Ada Lovelace Byron")).toBeInTheDocument();
    expect(certificates.getByText("Analytical Engines S.L. · B00000000")).toBeInTheDocument();
    expect(
      certificates.getByText("Representante · Ada Lovelace Byron · 00000000T"),
    ).toBeInTheDocument();
    expect(screen.getByRole("tabpanel", { name: "Certificados" }).textContent).not.toMatch(
      /[/\\][A-Za-z.]|\.p12/,
    );
    expect(
      certificates.getByRole("button", { name: "Quitar el certificado de Charles Babbage" }),
    ).toBeInTheDocument();
    expect(certificates.getAllByRole("button", { name: /Quitar/ })).toHaveLength(3);
    expect(certificates.queryAllByRole("checkbox")).toHaveLength(0);
    expect(certificates.queryAllByRole("switch")).toHaveLength(0);
  });

  it("keeps an expired certificate in the list, with the reason it cannot sign", () => {
    renderWithCatalog(<CertificatesInstalled />);

    expect(panelOf("Certificados").getByText(/Caducó el /)).toBeInTheDocument();
  });
});

describe("Apariencia", () => {
  it("offers the three themes and every language whose catalog is complete", async () => {
    const user = userEvent.setup();
    renderWithCatalog(<Appearance />);

    const theme = screen.getByRole("combobox", { name: "Tema" });
    expect(theme).toHaveTextContent("El del sistema");
    await user.click(theme);
    expect(screen.getAllByRole("option").map((option) => option.textContent)).toEqual([
      "El del sistema",
      "Claro",
      "Oscuro",
    ]);
    await user.keyboard("{Escape}");

    const language = screen.getByRole("combobox", { name: "Idioma" });
    expect(language).toHaveTextContent("Español");
    await user.click(language);
    expect(screen.getAllByRole("option").map((option) => option.textContent)).toEqual([
      "Español",
      "Català",
      "Euskara",
      "Galego",
      "English",
    ]);
  });
});
