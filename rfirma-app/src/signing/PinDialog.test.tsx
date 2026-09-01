import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import type { Certificate } from "./certificate";
import { PinDialog } from "./PinDialog";
import type { TokenFailure } from "./token";

const certificate: Certificate = {
  id: "0123456789abcdef0123456789abcdef",
  label: "Firma",
  holderName: "Ada Lovelace Byron",
  idNumber: "99999999R",
  issuer: "AC FNMT Usuarios",
  store: "card",
  status: { kind: "valid" },
};

const wrongPin: TokenFailure = {
  situation: "incorrectPin",
  detail: "CKR_PIN_INCORRECT (C_Login)",
  attemptsLeft: 2,
};

const noop = () => {};

function renderDialog(props: Partial<Parameters<typeof PinDialog>[0]> = {}) {
  return renderWithCatalog(
    <PinDialog
      certificate={certificate}
      failure={null}
      onSubmit={noop}
      onCancel={noop}
      {...props}
    />,
  );
}

// Grada A: el diálogo no habla con el token, solo con quien lo montó.
describe("PinDialog", () => {
  it("says with which identity the document is being signed", () => {
    renderDialog();

    expect(screen.getByRole("dialog", { name: "Introduce el PIN de la tarjeta" })).toBeVisible();
    expect(screen.getByText("Ada Lovelace Byron · 99999999R")).toBeInTheDocument();
    expect(screen.getByText(/no se guarda en ningún sitio/)).toBeInTheDocument();
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
    renderDialog({ onSubmit });

    const submit = screen.getByRole("button", { name: "Firmar" });
    expect(submit).toBeEnabled();
    await user.click(submit);

    expect(onSubmit).toHaveBeenCalledWith("");
  });

  it("retries a wrong PIN without restarting the journey", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    renderDialog({ failure: wrongPin, onSubmit });

    // El diálogo sigue en pie, con el campo vacío y el aviso de intentos.
    expect(screen.getByText(/Te quedan 2 intentos/)).toBeInTheDocument();
    const field = screen.getByLabelText("PIN");
    expect(field).toHaveValue("");
    expect(field).toHaveAttribute("aria-invalid", "true");

    await user.type(field, "5678");
    await user.click(screen.getByRole("button", { name: "Firmar" }));

    expect(onSubmit).toHaveBeenCalledWith("5678");
  });

  it("counts the last attempt in the singular", () => {
    renderDialog({ failure: { ...wrongPin, attemptsLeft: 1 } });

    expect(screen.getByText(/Te queda 1 intento/)).toBeInTheDocument();
  });

  it("says it plainly when the module does not count the attempts", () => {
    renderDialog({ failure: { ...wrongPin, attemptsLeft: null } });

    expect(screen.getByText(/se bloquea tras varios intentos fallidos/)).toBeInTheDocument();
  });

  it("keeps the raw CKR apart and untranslated", () => {
    renderDialog({ failure: wrongPin });

    expect(screen.getByText("CKR_PIN_INCORRECT (C_Login)")).toBeInTheDocument();
    expect(screen.getByText("Detalle técnico")).toBeInTheDocument();
  });

  it("tells a locked card apart from a wrong PIN, and stops asking", () => {
    renderDialog({
      failure: { situation: "pinLocked", detail: "CKR_PIN_LOCKED (C_Login)", attemptsLeft: 0 },
    });

    expect(screen.getByText("La tarjeta está bloqueada")).toBeInTheDocument();
    expect(screen.getByText(/PUK/)).toBeInTheDocument();
    expect(screen.getByText("CKR_PIN_LOCKED (C_Login)")).toBeInTheDocument();
    expect(screen.queryByLabelText("PIN")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Firmar" })).not.toBeInTheDocument();
  });
});
