import { composeStories } from "@storybook/react-vite";
import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithCatalog } from "../testing/render";
import * as stories from "./SetupWizard.stories";

/** Presentación: lo que enseña cada historia del asistente, una fila por historia. */

const {
  Welcome,
  NothingDone,
  BothDone,
  CertificateDoneHandlerPending,
  WithoutAutoFirma,
  ProtectionOff,
} = composeStories(stories);

const stepButtons = ["Instalar", "Usar rFirma", "Ahora no"];

describe("the welcome", () => {
  it("welcomes with the version it is compatible with, the independence notice and the language", () => {
    renderWithCatalog(<Welcome />);

    expect(screen.getByText("Configurar rFirma")).toBeInTheDocument();
    expect(screen.getByText(/compatible con AutoFirma 1\.9\.2/)).toBeInTheDocument();
    expect(screen.getByText("Proyecto independiente")).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "Idioma" })).toHaveTextContent("Español");
    expect(screen.getByText("Paso 1 de 2")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Omitir configuración" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Continuar" })).toBeInTheDocument();
  });

  it("welcomes in the language it starts with", () => {
    renderWithCatalog(<Welcome />, "gl");

    expect(screen.getByText(/rFirma é compatible con AutoFirma 1\.9\.2/)).toBeInTheDocument();
    expect(screen.getByText("Paso 1 de 2")).toBeInTheDocument();
  });

  it("leaves the header to the gtk titlebar on linux", () => {
    renderWithCatalog(<Welcome menuAnchor="titlebar" />);

    expect(screen.getByText("Configurar rFirma")).toBeInTheDocument();
    expect(screen.queryByText("rFirma")).toBeNull();
    expect(screen.queryByRole("button", { name: "Menú" })).toBeNull();
  });
});

describe("the two steps", () => {
  it("starts pending, with numbered markers and both actions, when nothing is done", async () => {
    renderWithCatalog(<NothingDone />);

    expect(screen.getByText("Paso 2 de 2")).toBeInTheDocument();
    expect(screen.getByText("Certificado de rFirma")).toBeInTheDocument();
    expect(screen.getByText("Conexión segura entre el navegador y rFirma.")).toBeInTheDocument();
    expect(screen.getByText("Usar rFirma por defecto")).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "Instalar" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Usar rFirma" })).toBeInTheDocument();
    expect(screen.getByText("1")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Terminar" })).toBeInTheDocument();
  });

  it("starts both steps done and without buttons when the CA is installed and rFirma already opens the sites", async () => {
    renderWithCatalog(<BothDone />);

    expect(await screen.findByText("Instalado en tus navegadores.")).toBeInTheDocument();
    expect(screen.getByText("Ahora abren rFirma.")).toBeInTheDocument();
    for (const name of stepButtons) {
      expect(screen.queryByRole("button", { name })).not.toBeInTheDocument();
    }
  });

  it("leaves only the handler step pending when the certificate is installed", async () => {
    renderWithCatalog(<CertificateDoneHandlerPending />);

    expect(await screen.findByText("Instalado en tus navegadores.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Usar rFirma" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Instalar" })).not.toBeInTheDocument();
  });

  it("shows no hint on the handler step when AutoFirma does not appear among the candidates", async () => {
    renderWithCatalog(<WithoutAutoFirma />);
    await screen.findByRole("button", { name: "Usar rFirma" });

    const handlerTitle = screen.getByText("Usar rFirma por defecto");
    expect(handlerTitle.parentElement?.querySelector(".rf-hint")).toBeNull();
  });
});

describe("the protection against accidental signing", () => {
  it("shows the switch on by default and off when the preference says so", async () => {
    const on = renderWithCatalog(<NothingDone />);
    expect(await screen.findByRole("switch")).toBeChecked();
    on.unmount();

    renderWithCatalog(<ProtectionOff />);
    await screen.findByRole("button", { name: "Instalar" });
    expect(await screen.findByRole("switch")).not.toBeChecked();
  });
});
