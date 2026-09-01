import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const {
  tauriCertificateStore,
  tauriDocumentPicker,
  tauriLayer2Composer,
  tauriPdfSource,
  tauriSigningBackend,
} = await import("./tauri");

const anOrder = {
  document: "/run/user/1000/doc/1e8b83b9/contrato.pdf",
  certificate: "Firma",
  placement: {
    page: 3,
    mediaBox: [0, 0, 595, 842] as const,
    rotation: 0,
    rect: [72, 500, 272, 600] as const,
  },
  fields: { signerName: true, idNumber: true, signedAt: true, reason: false },
  reason: "",
  signedAt: "31/08/26, 12:00:00",
  rubric: null,
  language: "es",
};

/**
 * **Grada A**: `invoke` es un doble, así que lo que se prueba es la costura —qué
 * orden se llama, con qué, y cómo vuelve un fallo—, no el backend.
 */
describe("los puertos de firma sobre Tauri", () => {
  // Con cuerpo de bloque, y no `() => invoke.mockReset()`: esa forma devuelve
  // el propio doble, y vitest llama a lo que devuelve un hook como su función
  // de limpieza. El doble se invocaría otra vez al terminar la prueba —fuera
  // de todo `try`— y una implementación que lanza pondría en rojo una prueba
  // que ya había pasado.
  beforeEach(() => {
    invoke.mockReset();
  });

  it("asks each stage by its own command, and in the order the ADR fixes", async () => {
    invoke.mockResolvedValue(undefined);
    const backend = tauriSigningBackend();

    await backend.presign(anOrder);
    await backend.sign("1234");
    await backend.postsign();

    expect(invoke.mock.calls.map(([command]) => command)).toEqual([
      "begin_signing",
      "sign_with_pin",
      "finish_signing",
    ]);
  });

  /**
   * La cuarta operación del puerto: la salida. La orden existía y estaba
   * registrada en `lib.rs` sin que nadie la llamara, que es exactamente el
   * agujero que deja el ciclo a medias vivo en memoria.
   */
  it("wires the discard of the half-open cycle to cancel_signing", async () => {
    invoke.mockResolvedValue(undefined);

    await tauriSigningBackend().discard();

    expect(invoke).toHaveBeenCalledWith("cancel_signing");
  });

  it("sends the order whole to the presignature and nothing else after it", () => {
    invoke.mockResolvedValue(undefined);

    void tauriSigningBackend().presign(anOrder);

    expect(invoke).toHaveBeenCalledWith("begin_signing", { order: anOrder });
  });

  it("never sends the PIN with anything else", async () => {
    // El PIN va solo, en su propia orden y después de la prefirma: mandarlo
    // junto al documento sería pedir el secreto que desbloquea la clave antes
    // de saber si el documento se puede firmar.
    invoke.mockResolvedValue(undefined);

    await tauriSigningBackend().sign("1234");

    expect(invoke).toHaveBeenCalledWith("sign_with_pin", { pin: "1234" });
  });

  it("keeps the situation and the raw detail that the backend classified", async () => {
    invoke.mockImplementation(() =>
      Promise.reject({
        situation: "incorrectPin",
        detail: "CKR_PIN_INCORRECT (C_Login)",
        attemptsLeft: 2,
      }),
    );

    const outcome = await tauriSigningBackend().sign("0000");

    expect(outcome).toEqual({
      ok: false,
      failure: {
        situation: "incorrectPin",
        detail: "CKR_PIN_INCORRECT (C_Login)",
        attemptsLeft: 2,
      },
    });
  });

  it("falls back to unknown without losing the text of what it could not classify", async () => {
    // Lo que no venga clasificado —una excepción del propio puente de Tauri,
    // una orden que no existe— cae en `unknown` **con su texto**. Perderlo
    // sería quedarse sin lo único que sirve para diagnosticarlo (ADR-0009).
    invoke.mockImplementation(() => Promise.reject(new Error("command begin_signing not found")));

    const outcome = await tauriSigningBackend().presign(anOrder);

    expect(outcome).toEqual({
      ok: false,
      failure: {
        situation: "unknown",
        detail: "command begin_signing not found",
        attemptsLeft: null,
      },
    });
  });

  it("asks the token for its certificates without a PIN in sight", async () => {
    invoke.mockResolvedValue([]);

    await tauriCertificateStore().list();

    expect(invoke).toHaveBeenCalledWith("list_certificates");
  });

  it("composes the preview with the chosen certificate and the instant it was given", async () => {
    invoke.mockResolvedValue("Firmado por: ADA LOVELACE");
    const signer = { certificate: "Firma", signedAt: "31/08/26, 12:00:00", language: "es" };

    const text = await tauriLayer2Composer().compose(
      { enabled: true, rubric: false, fields: anOrder.fields, reason: "" },
      signer,
    );

    expect(text).toBe("Firmado por: ADA LOVELACE");
    const [, payload] = invoke.mock.calls[0] ?? [];
    expect(payload).toMatchObject({
      order: { certificate: "Firma", signedAt: "31/08/26, 12:00:00", language: "es" },
    });
  });

  it("leaves the preview empty rather than raising an error notice", async () => {
    // La vista previa no es sitio para un aviso de error: si el token se ha
    // retirado mientras se miraba, el recuadro se queda vacío y lo cuenta el
    // intento de firmar.
    invoke.mockImplementation(() => Promise.reject(new Error("CKR_DEVICE_REMOVED")));

    const text = await tauriLayer2Composer().compose(
      { enabled: true, rubric: false, fields: anOrder.fields, reason: "" },
      { certificate: "Firma", signedAt: "31/08/26, 12:00:00", language: "es" },
    );

    expect(text).toBeNull();
  });
});

/**
 * **Grada A**: los dos puertos del documento contra el mismo `invoke` falso
 * (TD-16), con el caso bueno, la cancelación y el fallo de lectura, que es
 * justo lo que cubren las pruebas de los tres puertos de firma de arriba.
 */
describe("los puertos del documento sobre Tauri", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("asks the backend to open the dialog, and nothing else", async () => {
    invoke.mockResolvedValue({ id: "0f1e2d3c", name: "contrato.pdf", modified: 1_700_000_000 });

    await tauriDocumentPicker().choose();

    expect(invoke.mock.calls.map(([command]) => command)).toEqual(["open_document"]);
  });

  it("turns what the portal granted into a tray row badged Unsigned", async () => {
    // Detectar si un PDF ya trae firmas es otro trabajo: se anota lo que se
    // sabe y no se inventa (ID-71).
    invoke.mockResolvedValue({ id: "0f1e2d3c", name: "contrato.pdf", modified: 1_700_000_000 });

    const chosen = await tauriDocumentPicker().choose();

    expect(chosen).toMatchObject({
      id: "0f1e2d3c",
      name: "contrato.pdf",
      badge: "Unsigned",
      modified: 1_700_000_000,
      available: true,
    });
  });

  it("reads a cancelled dialog as no document, and not as a failure", async () => {
    invoke.mockResolvedValue(null);

    await expect(tauriDocumentPicker().choose()).resolves.toBeNull();
  });

  it("asks for the bytes of a document by its identifier, never by a path", async () => {
    invoke.mockResolvedValue(new Uint8Array([1, 2, 3]).buffer);

    await tauriPdfSource().open(aRow());

    expect(invoke).toHaveBeenCalledWith("read_document", { id: "0f1e2d3c" });
  });

  it("keeps the situation and the raw detail when the bytes cannot be read", async () => {
    invoke.mockImplementation(() =>
      Promise.reject({
        situation: "documentUnreadable",
        detail: "No such file or directory (os error 2)",
        attemptsLeft: null,
      }),
    );

    const outcome = await tauriPdfSource().open(aRow());

    expect(outcome).toEqual({
      ok: false,
      failure: {
        situation: "documentUnreadable",
        detail: "No such file or directory (os error 2)",
      },
    });
  });

  it("names the failure of a corrupt PDF instead of coming back empty", async () => {
    // Los bytes llegaron: lo que falla es abrirlos, y `pdf.js` no clasifica
    // nada. Sin nombre, el visor se quedaba en su estado vacío, que es el mismo
    // que cuando no se ha abierto nada.
    invoke.mockResolvedValue(new Uint8Array([0x25, 0x21, 0x3f]).buffer);

    const outcome = await tauriPdfSource().open(aRow());

    expect(outcome.ok).toBe(false);
    if (outcome.ok) return;
    expect(outcome.failure.situation).toBe("documentUnreadable");
    expect(outcome.failure.detail).not.toBe("");
  });
});

/** Una fila de la bandeja, que es lo que entra por el puerto del visor. */
function aRow() {
  return {
    id: "0f1e2d3c",
    name: "contrato.pdf",
    badge: "Unsigned" as const,
    modified: 1_700_000_000,
    lastUsed: 1_700_000_000,
    available: true,
  };
}
