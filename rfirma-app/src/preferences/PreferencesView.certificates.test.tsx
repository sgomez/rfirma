import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { anInstalledCertificate, IN_2020, openTab, renderView } from "./testSupport";

/**
 * Certificados en fichero (docs/design/preferencias.md): una lista y dos
 * gestos, y ni una casilla por almacén ni un diálogo anidado (ID-198).
 */
describe("certificates in a file", () => {
  it("lists an installed certificate by its holder and never by its file", async () => {
    const user = userEvent.setup();
    renderView({ installedCertificates: [anInstalledCertificate()] });
    await openTab(user, "Certificados");

    const certificates = screen.getByRole("tabpanel", { name: "Certificados" });
    expect(within(certificates).getByText("Ada Lovelace Byron")).toBeInTheDocument();
    expect(
      within(certificates).getByText(/IDCES-00000000T · Emitido por FNMT-RCM · caduca el /),
    ).toBeInTheDocument();
    expect(certificates.textContent).not.toMatch(/[/\\]/);
  });

  it("offers the two gestures and nothing else", async () => {
    const user = userEvent.setup();
    renderView({ installedCertificates: [anInstalledCertificate()] });
    await openTab(user, "Certificados");

    const certificates = screen.getByRole("tabpanel", { name: "Certificados" });
    expect(within(certificates).getByRole("button", { name: "Añadir…" })).toBeInTheDocument();
    expect(
      within(certificates).getByRole("button", {
        name: "Quitar el certificado de Ada Lovelace Byron",
      }),
    ).toBeInTheDocument();
    expect(within(certificates).queryAllByRole("checkbox")).toHaveLength(0);
    expect(within(certificates).queryAllByRole("switch")).toHaveLength(0);
  });

  it("says nothing is installed yet, without instructions inside the box", async () => {
    const user = userEvent.setup();
    renderView({ installedCertificates: [] });
    await openTab(user, "Certificados");

    const certificates = screen.getByRole("tabpanel", { name: "Certificados" });
    expect(within(certificates).getByText("Todavía no has instalado ninguno")).toBeInTheDocument();
  });

  /** Un caducado se queda: que desaparezca no le explica nada a quien lo instaló. */
  it("keeps an expired certificate in the list, with its badge", async () => {
    const user = userEvent.setup();
    renderView({
      installedCertificates: [
        anInstalledCertificate({ status: { kind: "expired", notAfter: IN_2020 } }),
      ],
    });
    await openTab(user, "Certificados");

    const certificates = screen.getByRole("tabpanel", { name: "Certificados" });
    expect(within(certificates).getByText("Ada Lovelace Byron")).toBeInTheDocument();
    expect(within(certificates).getByText("Caducado")).toBeInTheDocument();
  });

  it("asks for the password of the file and installs with it", async () => {
    const user = userEvent.setup();
    const onInstallCertificate = vi.fn(async () => true);
    renderView({ onInstallCertificate });
    await openTab(user, "Certificados");

    await user.click(screen.getByRole("button", { name: "Añadir…" }));
    await user.type(screen.getByLabelText("Contraseña"), "hunter2");
    await user.click(screen.getByRole("button", { name: "Continuar" }));

    expect(onInstallCertificate).toHaveBeenCalledWith("hunter2");
  });

  it("calls the password off with Escape, without closing the screen", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    const onInstallCertificate = vi.fn(async () => true);
    renderView({ onClose, onInstallCertificate });
    await openTab(user, "Certificados");

    await user.click(screen.getByRole("button", { name: "Añadir…" }));
    await user.keyboard("{Escape}");

    expect(screen.queryByLabelText("Contraseña")).not.toBeInTheDocument();
    expect(onInstallCertificate).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  /**
   * ID-197 + ID-211: se rechaza al instalar y en un solo renglón. Ni la
   * curva, ni el mecanismo, ni «instala uno de clave RSA».
   */
  it("says an elliptic key does not work, in a single line", async () => {
    const user = userEvent.setup();
    const onInstallCertificate = vi.fn(async () => {
      throw { situation: "keyNotRsa", detail: "FIRMA: la clave no es RSA" };
    });
    renderView({ onInstallCertificate });
    await openTab(user, "Certificados");

    await user.click(screen.getByRole("button", { name: "Añadir…" }));
    await user.click(screen.getByRole("button", { name: "Continuar" }));

    const notice = await screen.findByRole("alert");
    expect(notice).toHaveTextContent("Ese certificado no es compatible con rFirma");
    expect(notice.textContent).not.toMatch(/RSA|elíptica|curva/);
    expect(within(notice).queryByText("Detalle técnico")).not.toBeInTheDocument();
    expect(screen.getByRole("tabpanel", { name: "Certificados" })).toHaveTextContent(
      "Todavía no has instalado ninguno",
    );
  });

  /** Cerrar el selector sin elegir nada no es un fallo: no se cuenta nada. */
  it("says nothing when the file picker was closed without choosing anything", async () => {
    const user = userEvent.setup();
    renderView({ onInstallCertificate: async () => false });
    await openTab(user, "Certificados");

    await user.click(screen.getByRole("button", { name: "Añadir…" }));
    await user.click(screen.getByRole("button", { name: "Continuar" }));

    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("removes an installed certificate by its handle", async () => {
    const user = userEvent.setup();
    const onRemoveCertificate = vi.fn(async () => {});
    renderView({
      installedCertificates: [anInstalledCertificate({ id: "2a01" })],
      onRemoveCertificate,
    });
    await openTab(user, "Certificados");

    await user.click(
      screen.getByRole("button", { name: "Quitar el certificado de Ada Lovelace Byron" }),
    );

    expect(onRemoveCertificate).toHaveBeenCalledWith("2a01");
  });

  it("shows in the section that the certificate could not be removed", async () => {
    const user = userEvent.setup();
    renderView({
      installedCertificates: [anInstalledCertificate()],
      onRemoveCertificate: async () => {
        throw { situation: "certificateNotFound", detail: "ya no esta" };
      },
    });
    await openTab(user, "Certificados");

    await user.click(
      screen.getByRole("button", { name: "Quitar el certificado de Ada Lovelace Byron" }),
    );

    const certificates = screen.getByRole("tabpanel", { name: "Certificados" });
    expect(await within(certificates).findByRole("alert")).toBeInTheDocument();
  });
});
