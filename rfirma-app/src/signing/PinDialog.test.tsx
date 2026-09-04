import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import type { Certificate } from "./certificate";
import { PinDialog } from "./PinDialog";
import type { TokenFailure } from "./token";

const cardCertificate: Certificate = {
  id: "0123456789abcdef0123456789abcdef",
  label: "Firma",
  holderName: "Ada Lovelace Byron",
  idNumber: "99999999R",
  issuer: "AC FNMT Usuarios",
  store: "card",
  status: { kind: "valid", notAfter: 1_894_752_000 },
  remembered: false,
};

const firefoxCertificate: Certificate = { ...cardCertificate, store: "firefox" };

const chromeCertificate: Certificate = { ...cardCertificate, store: "chrome" };

const p12Certificate: Certificate = { ...cardCertificate, store: "nssdb" };

const wrongPin: TokenFailure = {
  situation: "incorrectPin",
  detail: "CKR_PIN_INCORRECT (C_Login)",
  attemptsLeft: 2,
};

const noop = () => {};

function renderDialog(props: Partial<Parameters<typeof PinDialog>[0]> = {}) {
  return renderWithCatalog(
    <PinDialog
      certificate={cardCertificate}
      failure={null}
      onSubmit={noop}
      onCancel={noop}
      {...props}
    />,
  );
}

// Grada A: el diálogo no habla con el token, solo con quien lo montó.
describe("PinDialog", () => {
  it("calls it a PIN for a PKCS#11 module, without naming the module itself", () => {
    renderDialog({ certificate: cardCertificate });

    expect(screen.getByRole("dialog", { name: "Introduce el PIN" })).toBeVisible();
    expect(screen.getByLabelText("PIN")).toBeInTheDocument();
    // Ni la clase de almacén, ni el nombre del token.
    expect(screen.queryByText(/PKCS#11/)).not.toBeInTheDocument();
    expect(screen.queryByText(/SoftHSM/)).not.toBeInTheDocument();
  });

  it("calls it a password for a file, and says whose certificates it is", () => {
    renderDialog({ certificate: firefoxCertificate });

    expect(screen.getByRole("dialog", { name: "Introduce la contraseña" })).toBeVisible();
    expect(screen.getByLabelText("Contraseña")).toBeInTheDocument();
    expect(screen.getByText("Tus certificados de Firefox")).toBeInTheDocument();
  });

  // ID-188: la palabra la elige la clase de almacén, no el hardware. Chrome
  // es otro perfil NSS, igual que Firefox, y también pide "contraseña".
  it("calls it a password for a Chrome profile too, by store class", () => {
    renderDialog({ certificate: chromeCertificate });

    expect(screen.getByRole("dialog", { name: "Introduce la contraseña" })).toBeVisible();
    expect(screen.getByText("Tus certificados de Chrome")).toBeInTheDocument();
  });

  it("says which certificate it is unlocking for a .p12 file", () => {
    renderDialog({ certificate: p12Certificate });

    expect(screen.getByText("Ada Lovelace Byron · 99999999R")).toBeInTheDocument();
  });

  it("names nothing about a PKCS#11 module: not even the identity it signs with", () => {
    renderDialog({ certificate: cardCertificate });

    expect(screen.queryByText("Ada Lovelace Byron · 99999999R")).not.toBeInTheDocument();
  });

  it("carries no hint of any kind, reassuring or otherwise", () => {
    renderDialog();

    expect(screen.queryByText(/se usa solo para esta firma/)).not.toBeInTheDocument();
    expect(screen.queryByText(/no se guarda en ningún sitio/)).not.toBeInTheDocument();
  });

  it("hands the typed PIN over and never shows it in clear", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    renderDialog({ onSubmit });

    const field = screen.getByLabelText("PIN");
    expect(field).toHaveAttribute("type", "password");
    await user.type(field, "1234");
    await user.click(screen.getByRole("button", { name: "Firmar" }));

    expect(onSubmit).toHaveBeenCalledWith("1234");
  });

  // Quien firma con un certificado de Firefox sin contraseña maestra puesta no
  // tiene ningún secreto que teclear. El diálogo tiene que dejarle seguir y
  // entregar la CADENA VACÍA: para `C_Login` no es lo mismo que «sin PIN», y un
  // botón deshabilitado aquí sería un diálogo imposible de completar (#99).
  it("hands over the empty string when there is no master password to type", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    renderDialog({ certificate: firefoxCertificate, onSubmit });

    const submit = screen.getByRole("button", { name: "Firmar" });
    expect(submit).toBeEnabled();
    await user.click(submit);

    expect(onSubmit).toHaveBeenCalledWith("");
  });

  it("retries a wrong PIN without restarting the journey", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    renderDialog({ failure: wrongPin, onSubmit });

    // El diálogo sigue en pie, con el campo vacío.
    const field = screen.getByLabelText("PIN");
    expect(field).toHaveValue("");
    expect(field).toHaveAttribute("aria-invalid", "true");

    await user.type(field, "5678");
    await user.click(screen.getByRole("button", { name: "Firmar" }));

    expect(onSubmit).toHaveBeenCalledWith("5678");
  });

  // ID-191: PKCS#11 no cuenta los intentos que quedan, ni con una tarjeta
  // delante. No hay contador que enseñar, y no se inventa ninguno.
  it("shows the wrong-secret error in one line, with no remedy underneath", () => {
    renderDialog({ failure: wrongPin });

    expect(screen.getByText("PIN incorrecto")).toBeInTheDocument();
    expect(screen.queryByText(/intento/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Detalle técnico/)).not.toBeInTheDocument();
  });

  it("calls the wrong secret a password when the store asks for one", () => {
    renderDialog({ certificate: firefoxCertificate, failure: wrongPin });

    expect(screen.getByText("Contraseña incorrecta")).toBeInTheDocument();
  });
});
