import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import { CertificateSelect } from "./CertificateSelect";
import type { Certificate } from "./certificate";

/**
 * **Grada A**: el desplegable son datos y una devolución de llamada; no habla
 * con el token. Las etiquetas repetidas **de verdad** —dos claves con el mismo
 * `CKA_LABEL`— las prueban `tests/pkcs11_token.rs` y `tests/nss_store.rs`, que
 * son grada B.
 */
function aCertificate(overrides: Partial<Certificate> = {}): Certificate {
  return {
    id: "0123456789abcdef0123456789abcdef",
    label: "Firma",
    holderName: "Ada Lovelace Byron",
    idNumber: "99999999R",
    issuer: "AC FNMT Usuarios",
    store: "card",
    status: { kind: "valid" },
    remembered: false,
    ...overrides,
  };
}

/**
 * Dos certificados con **la misma etiqueta** y el mismo titular, en dos
 * almacenes distintos: es el caso que el asa existe para resolver.
 *
 * El orden ya es el que produce `groupCertificates` —mismo titular, empate
 * por almacén, «chrome» antes que «firefox»— para que los índices de fila de
 * estas pruebas coincidan con el orden de inserción sin sorpresas.
 */
const twins: readonly Certificate[] = [
  aCertificate({ id: "aaaa", store: "chrome" }),
  aCertificate({ id: "bbbb", store: "firefox" }),
];

function renderSelect(props: Partial<Parameters<typeof CertificateSelect>[0]> = {}) {
  const onChoose = vi.fn();
  renderWithCatalog(
    <CertificateSelect certificates={twins} chosen={null} onChoose={onChoose} {...props} />,
  );
  return { onChoose };
}

const trigger = () => screen.getByRole("combobox", { name: "Certificado" });

/** La fila que ocupa ese sitio en la lista. Falla diciéndolo si no está. */
function row(index: number): HTMLElement {
  const rows = screen.getAllByRole("option");
  const found = rows[index];
  if (found === undefined) throw new Error(`la lista no tiene fila ${index}`);
  return found;
}

describe("CertificateSelect", () => {
  it("lists every certificate and chooses the one that is clicked", async () => {
    const { onChoose } = renderSelect();

    await userEvent.click(trigger());

    expect(screen.getAllByRole("option")).toHaveLength(2);
    await userEvent.click(row(1));
    expect(onChoose).toHaveBeenCalledWith(twins[1]);
  });

  /** El caso que hace falta el asa: dos filas con la misma etiqueta y el mismo
   * titular, y elegir la segunda tiene que elegir **la segunda**. */
  it("tells two certificates with the same label apart by their handle", async () => {
    const { onChoose } = renderSelect();

    await userEvent.click(trigger());
    await userEvent.click(row(1));

    expect(onChoose.mock.calls.at(0)?.at(0)).toMatchObject({ id: "bbbb" });
  });

  it("shows the holder, the id, the issuer and the store on every row", async () => {
    renderSelect();

    await userEvent.click(trigger());

    expect(row(0)).toHaveTextContent("Ada Lovelace Byron");
    expect(row(0)).toHaveTextContent("99999999R · Emitido por AC FNMT Usuarios · Chrome");
    expect(row(1)).toHaveTextContent("Firefox");
  });

  /** El almacén lo traduce la ventana desde el catálogo: lo que cruza la
   * frontera es la clase en inglés, y en inglés se lee en inglés. */
  it("translates the store class instead of showing it raw", async () => {
    renderWithCatalog(
      <CertificateSelect
        certificates={[aCertificate({ store: "card" })]}
        chosen={null}
        onChoose={vi.fn()}
      />,
      "en",
    );

    await userEvent.click(screen.getByRole("combobox", { name: "Certificate" }));

    expect(screen.getByRole("option")).toHaveTextContent("Card");
    expect(screen.getByRole("option")).not.toHaveTextContent("card ·");
  });

  it("says «choose certificate» while nothing is chosen", () => {
    renderSelect();

    expect(trigger()).toHaveTextContent("Elegir certificado");
  });

  it("shows the chosen one without its store: chosen, it disambiguates nothing", () => {
    renderSelect({ chosen: twins[0] });

    expect(trigger()).toHaveTextContent("Ada Lovelace Byron");
    expect(trigger()).toHaveTextContent("99999999R · Emitido por AC FNMT Usuarios");
    expect(trigger()).not.toHaveTextContent("Chrome");
  });

  /** Que falte de la lista no le explica nada a quien viene a firmar justo con
   * ese: se lista, dice por qué, y no se deja elegir. */
  it("lists an expired certificate, says why, and refuses to choose it", async () => {
    const expired = aCertificate({
      id: "cccc",
      status: { kind: "expired", notAfter: 1_767_225_600 },
    });
    const { onChoose } = renderSelect({ certificates: [expired] });

    await userEvent.click(trigger());
    const row = screen.getByRole("option");

    expect(row).toHaveTextContent(/El certificado caducó el/);
    expect(row).toHaveAttribute("aria-disabled", "true");
    await userEvent.click(row);
    expect(onChoose).not.toHaveBeenCalled();
  });

  it("lists a revoked certificate the same way", async () => {
    const revoked = aCertificate({
      id: "dddd",
      status: { kind: "revoked", reason: "keyCompromise" },
    });
    const { onChoose } = renderSelect({ certificates: [revoked] });

    await userEvent.click(trigger());
    await userEvent.click(screen.getByRole("option"));

    expect(screen.getByRole("option")).toHaveTextContent(/revocado/);
    expect(onChoose).not.toHaveBeenCalled();
  });

  it("closes when one is chosen", async () => {
    renderSelect();

    await userEvent.click(trigger());
    await userEvent.click(row(0));

    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("closes on Escape without choosing anything", async () => {
    const { onChoose } = renderSelect();

    await userEvent.click(trigger());
    await userEvent.keyboard("{Escape}");

    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(onChoose).not.toHaveBeenCalled();
  });

  it("closes when something outside is pressed", async () => {
    renderSelect();

    await userEvent.click(trigger());
    await userEvent.click(document.body);

    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  /** Un `<div>` con un `onClick` no es un desplegable: se recorre con las
   * flechas y se elige con Intro. */
  it("is walkable with the keyboard from the trigger", async () => {
    const { onChoose } = renderSelect();

    trigger().focus();
    await userEvent.keyboard("{ArrowDown}");
    await userEvent.keyboard("{ArrowDown}");
    await userEvent.keyboard("{Enter}");

    expect(onChoose).toHaveBeenCalledWith(twins[1]);
  });

  it("puts the cursor on what is already chosen when it opens", async () => {
    renderSelect({ chosen: twins[1] });

    await userEvent.click(trigger());

    const list = screen.getByRole("listbox");
    const cursor = list.getAttribute("aria-activedescendant");
    expect(row(1)).toHaveAttribute("id", cursor);
  });

  /** Prior art: los encabezados agrupan lo que la función pura de
   * `certificate.ts` ya ordenó; aquí solo se comprueba que aparecen. */
  it("groups the list under two headers, available first and unusable below", async () => {
    const expired = aCertificate({ id: "cccc", status: { kind: "expired", notAfter: 0 } });
    renderSelect({ certificates: [expired, ...twins] });

    await userEvent.click(trigger());

    expect(screen.getByText("Disponibles")).toBeVisible();
    expect(screen.getByText("No utilizables")).toBeVisible();
    expect(screen.getAllByRole("option")).toHaveLength(3);
  });

  /** Sin ninguno de los dos grupos vacío, no sale su encabezado: no hay nada
   * que titular. */
  it("does not show the unusable header when every certificate can be used", async () => {
    renderSelect();

    await userEvent.click(trigger());

    expect(screen.queryByText("No utilizables")).not.toBeInTheDocument();
  });

  /** «Deshabilitada de verdad»: la fila lleva la clase que en
   * `CertificateSelect.css` le pone `pointer-events: none`, así que el
   * navegador ni siquiera le entrega el puntero, y un clic no llega nunca a
   * intentar elegirla. */
  it("marks an unusable row so the pointer never reaches it", async () => {
    const expired = aCertificate({ id: "cccc", status: { kind: "expired", notAfter: 0 } });
    const { onChoose } = renderSelect({ certificates: [expired] });

    await userEvent.click(trigger());
    const disabledRow = screen.getByRole("option");

    expect(disabledRow).toHaveClass("certificate-select__option--unusable");
    await userEvent.click(disabledRow);
    expect(onChoose).not.toHaveBeenCalled();
  });
});
