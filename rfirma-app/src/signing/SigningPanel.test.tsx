import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithCatalog } from "../testing/render";
import type { Certificate } from "./certificate";
import type { Rubric } from "./rubric";
import { SigningPanel } from "./SigningPanel";
import { DEFAULT_VISIBLE_SIGNATURE, type Layer2Composer } from "./visibleSignature";

const certificate: Certificate = {
  label: "Firma",
  holderName: "Ada Lovelace Byron",
  idNumber: "99999999R",
  issuer: "AC FNMT Usuarios",
  status: { kind: "valid" },
};

/** Un JPEG de un píxel: lo que devuelve `rubric::normalize`, ya opaco. */
const rubric: Rubric = {
  dataUrl: "data:image/jpeg;base64,/9j/4AAQSkZJRg==",
  width: 240,
  height: 80,
};

/** El compositor de verdad es Rust; aquí basta con que devuelva algo fiel. */
function composerOf(text: string | null): Layer2Composer {
  return { compose: async () => text };
}

const noop = () => {};

function renderPanel(props: Partial<Parameters<typeof SigningPanel>[0]> = {}) {
  return renderWithCatalog(
    <SigningPanel
      document={{ name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000, signatures: 0 }}
      certificate={{ kind: "chosen", certificate }}
      onChooseCertificate={noop}
      onRetryCertificates={noop}
      onChooseModule={noop}
      signature={DEFAULT_VISIBLE_SIGNATURE}
      onChangeSignature={noop}
      page={3}
      rubric={null}
      rubricFailure={null}
      onChooseRubric={noop}
      signedAt="31/08/26, 12:00:00"
      composer={composerOf(null)}
      destination={{ folder: "Documentos", writable: true }}
      onChangeDestination={noop}
      onSign={noop}
      signing={false}
      failure={null}
      {...props}
    />,
  );
}

// Grada A: el panel son datos y devoluciones de llamada; no habla con nadie.
describe("SigningPanel", () => {
  it("counts the pages in singular when the document has only one", () => {
    renderPanel({
      document: { name: "instancia.pdf", pages: 1, sizeBytes: null, signatures: null },
    });

    expect(screen.getByText("1 página")).toBeInTheDocument();
  });

  it("counts the pages in plural when the document has more than one", () => {
    renderPanel();

    expect(screen.getByText(/^27 páginas/)).toBeInTheDocument();
  });

  it("covers the certificate, the visible-signature toggle, the page, the checkboxes and the reason", async () => {
    const user = userEvent.setup();
    renderPanel({
      signature: {
        ...DEFAULT_VISIBLE_SIGNATURE,
        fields: { ...DEFAULT_VISIBLE_SIGNATURE.fields, reason: true },
      },
    });

    expect(screen.getByText("Ada Lovelace Byron")).toBeInTheDocument();
    const toggle = screen.getByRole("switch", {
      name: /Estampar un recuadro de firma en el documento/,
    });
    expect(toggle).toBeInTheDocument();
    // El panel lo dibuja con `rf-gap-xs` (8 px, `Main.dc.html:306`); los 16 px
    // son de Preferencias y se piden allí con `switch--wide`.
    expect(toggle.closest(".switch")).not.toHaveClass("switch--wide");
    expect(screen.getByText("Página 3 · arrástralo para colocarlo")).toBeInTheDocument();
    for (const label of [
      "Tu rúbrica",
      "Nombre y apellidos",
      "DNI",
      "Fecha y hora de la firma",
      "Un motivo",
    ]) {
      expect(screen.getByRole("checkbox", { name: new RegExp(label) })).toBeInTheDocument();
    }
    await user.type(screen.getByLabelText("Motivo"), "!");
    expect(screen.getByRole("button", { name: "Elegir imagen" })).toBeInTheDocument();
  });

  it("is the only region with a primary button, and it comes last", () => {
    renderPanel();

    const primaries = Array.from(
      document.querySelectorAll<HTMLButtonElement>("button.rf-btn--primary"),
    );
    expect(primaries.map((button) => button.textContent)).toEqual(["Firmar documento"]);

    const buttons = screen.getAllByRole("button");
    expect(buttons.at(-1)).toBe(primaries[0]);
  });

  // El artboard enseña «27 páginas · 2,4 MB» y un resumen de firmas que hoy
  // nadie calcula. Lo desconocido **no ocupa sitio**: ni un guion, ni un «—»,
  // ni un marcador de posición.
  it("paints nothing at all in place of what nobody knows yet", () => {
    renderPanel({
      document: { name: "contrato.pdf", pages: 27, sizeBytes: null, signatures: null },
    });

    // La línea de metadatos dice las páginas y **nada más**: sin el separador
    // que precedería al tamaño, y sin tamaño.
    expect(screen.getByText("27 páginas")).toBeInTheDocument();
    expect(screen.getByText("27 páginas").textContent).toBe("27 páginas");
    expect(screen.queryByText(/—|–|\bMB\b|\bkB\b/)).not.toBeInTheDocument();
    expect(screen.queryByText(/cofirma/)).not.toBeInTheDocument();
  });

  it("shows the destination folder by its name and never by its path", () => {
    renderPanel({ destination: { folder: "Documentos", writable: true } });

    // El artboard parte la fila en dos: «Se guardará en» como rótulo y la
    // carpeta debajo, junto al icono de carpeta. Lo que la prueba defiende
    // —que se ve el nombre y nunca la ruta— no cambia.
    expect(screen.getByText("Se guardará en")).toBeInTheDocument();
    expect(screen.getByText("Documentos")).toBeInTheDocument();
    expect(screen.queryByText(/\/home\//)).not.toBeInTheDocument();
  });

  it("keeps the sign button alive when the destination cannot be written to", () => {
    renderPanel({ destination: { folder: "Documentos", writable: false } });

    expect(screen.getByText("No se puede escribir en Documentos")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Firmar documento" })).toBeEnabled();
  });

  it("does not promise a destination it has just said it cannot write to", () => {
    // «Se guardará en» y «No se puede escribir en Documentos» a la vez es una
    // contradicción: el rótulo es la promesa y desaparece con ella.
    renderPanel({ destination: { folder: "Documentos", writable: false } });

    expect(screen.queryByText("Se guardará en")).not.toBeInTheDocument();
  });

  it("never shows a wildcard, neither in the interface nor in the preview", async () => {
    renderPanel({ composer: composerOf("Firmado por: Ada Lovelace Byron\nDNI: ***9999**") });

    expect(await screen.findByText(/Firmado por: Ada Lovelace Byron/)).toBeInTheDocument();
    expect(document.body.textContent).not.toMatch(/\$\$/);
  });

  it("previews the composed text and not an imitation of its own", async () => {
    // El compositor autoritativo es `signing::layer2_text`: si el panel
    // compusiera el texto por su cuenta, esta cadena no aparecería tal cual.
    renderPanel({ composer: composerOf("Fecha: 31/08/2026 12:00:00 CEST") });

    expect(await screen.findByText("Fecha: 31/08/2026 12:00:00 CEST")).toBeInTheDocument();
  });

  it("shows the rubric already normalized, over white, before signing", () => {
    renderPanel({ rubric });

    const thumbnail = screen.getByAltText("Tu rúbrica, tal como se estampará");
    expect(thumbnail).toHaveAttribute("src", rubric.dataUrl);
    expect(screen.getByText(/Se estampa sobre blanco/)).toBeInTheDocument();
  });

  it("cannot tick a rubric that does not exist", () => {
    renderPanel({ rubric: null });

    expect(screen.getByRole("checkbox", { name: /Tu rúbrica/ })).toBeDisabled();
    expect(screen.getByText("Elige antes una imagen")).toBeInTheDocument();
  });

  it("counts the rubric failure as it is chosen, with the raw detail apart", () => {
    renderPanel({
      rubricFailure: { situation: "notAnAcceptedImage", detail: "image/gif" },
    });

    expect(screen.getByText("Esa imagen no vale como rúbrica")).toBeInTheDocument();
    expect(screen.getByText("image/gif")).toBeInTheDocument();
  });

  it("warns about the co-signature when the document already carries signatures", () => {
    renderPanel({
      document: { name: "contrato.pdf", pages: 27, sizeBytes: 2_400_000, signatures: 1 },
    });

    expect(screen.getByText("Ya lleva 1 firma: la tuya será una cofirma.")).toBeInTheDocument();
  });

  it("offers two ways out when no certificate turned up", async () => {
    const user = userEvent.setup();
    const onRetryCertificates = vi.fn();
    renderPanel({ certificate: { kind: "empty" }, onRetryCertificates });

    expect(screen.getByText(/comprueba que está insertada/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Otro módulo…" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Volver a buscar" }));

    expect(onRetryCertificates).toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Firmar documento" })).toBeDisabled();
  });

  it("shows a token failure as a translated situation with the raw CKR apart", () => {
    renderPanel({
      failure: { situation: "tokenAbsent", detail: "CKR_DEVICE_REMOVED (C_Sign)" },
    });

    expect(screen.getByText("No encontramos la tarjeta")).toBeInTheDocument();
    // El código original, ni traducido ni recortado: está para pegarlo.
    expect(screen.getByText("CKR_DEVICE_REMOVED (C_Sign)")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Volver a intentarlo" })).toBeInTheDocument();
  });

  it("warns about an expired certificate and refuses to sign with it", () => {
    renderPanel({
      certificate: {
        kind: "chosen",
        certificate: { ...certificate, status: { kind: "expired", notAfter: 1_767_225_600 } },
      },
    });

    expect(screen.getByRole("alert")).toHaveTextContent(/El certificado caducó el/);
    expect(screen.getByRole("button", { name: "Firmar documento" })).toBeDisabled();
  });

  it("warns about a revoked certificate and refuses to sign with it", () => {
    renderPanel({
      certificate: {
        kind: "chosen",
        certificate: { ...certificate, status: { kind: "revoked", reason: "keyCompromise" } },
      },
    });

    expect(screen.getByRole("alert")).toHaveTextContent(/revocado/);
    expect(screen.getByRole("button", { name: "Firmar documento" })).toBeDisabled();
  });

  it("waits for the certificates without pretending there are none", () => {
    renderPanel({ certificate: { kind: "loading" } });

    expect(screen.getByText("Buscando certificados…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Firmar documento" })).toBeDisabled();
  });
});
