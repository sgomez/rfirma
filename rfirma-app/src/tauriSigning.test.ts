import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const {
  tauriCertificateStore,
  tauriRubricPicker,
  tauriSigningBackend,
  tauriStampComposer,
  tauriVisibleSignatureMemory,
} = await import("./tauriSigning");

const anOrder = {
  document: "/run/user/1000/doc/1e8b83b9/contrato.pdf",
  certificate: "Firma",
  placement: {
    page: 3,
    pages: { only: [3] },
    pageCount: 10,
    mediaBox: [0, 0, 595, 842] as const,
    rotation: 0,
    rect: [72, 500, 272, 600] as const,
  },
  content: { model: "complete" as const },
  withRubric: false,
  signedAt: "31/08/26, 12:00:00",
  rubric: null,
  language: "es",
  allowUnregisteredSignatures: false,
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

  it("lands the postsignature on the preference's folder without a single destination", async () => {
    invoke.mockResolvedValue(undefined);

    await tauriSigningBackend().postsign();

    expect(invoke).toHaveBeenCalledWith("finish_signing", { destination: null });
  });

  it("lands the postsignature on the single destination chosen for this signature", async () => {
    invoke.mockResolvedValue(undefined);

    await tauriSigningBackend().postsign("single-42");

    expect(invoke).toHaveBeenCalledWith("finish_signing", { destination: "single-42" });
  });

  /**
   * ID-190: la ventana decide entre abrir el diálogo del secreto y firmar
   * directo con lo que devuelve la prefirma, así que ese valor tiene que
   * cruzar tal cual y no perderse en `void`.
   */
  it("passes the store's secret shape from the presignature through, unchanged", async () => {
    invoke.mockResolvedValue({ kind: "notNeeded" });

    const outcome = await tauriSigningBackend().presign(anOrder);

    expect(outcome).toEqual({ ok: true, value: { kind: "notNeeded" } });
  });

  /**
   * ID-105: la conversión a puntos PAdES no tiene copia en TypeScript, así
   * que el diálogo de páginas sin sello la pide por esta orden, en vez de
   * recalcularla.
   */
  it("asks the backend for the PAdES corner instead of computing it", async () => {
    invoke.mockResolvedValue([50, 145]);

    const lowerLeft = await tauriSigningBackend().padesLowerLeft(anOrder.placement);

    expect(invoke).toHaveBeenCalledWith("pades_lower_left", { placement: anOrder.placement });
    expect(lowerLeft).toEqual([50, 145]);
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
});

/**
 * **Grada A**: el ciclo en seco que compone el sello, contra el mismo `invoke`
 * falso. Lo que se prueba es la costura —qué orden se llama y cómo vuelve un
 * fallo—, no el puente: los bytes de verdad los mide el sondeo del #115.
 */
describe("el puerto del sello sobre Tauri", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("asks for the dry run with the very order that would be signed", async () => {
    invoke.mockResolvedValue(new Uint8Array([0x25, 0x50, 0x44, 0x46]).buffer);

    await tauriStampComposer().compose(anOrder);

    expect(invoke).toHaveBeenCalledWith("preview_signature", { order: anOrder });
  });

  /**
   * ID-111: la vista previa **no es una puerta**. El puerto no relanza nada, así
   * que quien lo llama no tiene un `catch` que decida si se puede firmar.
   */
  it("names the failure instead of rejecting, so signing stays possible", async () => {
    invoke.mockImplementation(() =>
      Promise.reject({
        situation: "documentUnreadable",
        detail: "el documento tiene contraseña",
        attemptsLeft: null,
      }),
    );

    const composed = await tauriStampComposer().compose(anOrder);

    expect(composed).toEqual({
      ok: false,
      failure: {
        situation: "documentUnreadable",
        detail: "el documento tiene contraseña",
      },
    });
  });
});

/**
 * **Grada A**: el puerto de la rúbrica contra el mismo `invoke` falso.
 *
 * Ni el caso bueno ni el rechazado revientan la promesa: las seis
 * situaciones de `RubricSituation` llegan clasificadas dentro de la propia
 * respuesta, así que `choose` nunca rechaza por una imagen que no vale.
 */
describe("el puerto de la rúbrica sobre Tauri", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("asks the backend to open the dialog, and nothing else", async () => {
    invoke.mockResolvedValue(null);

    await tauriRubricPicker().choose();

    expect(invoke.mock.calls.map(([command]) => command)).toEqual(["choose_rubric"]);
  });

  it("reads a cancelled dialog as no rubric, and not as a failure", async () => {
    invoke.mockResolvedValue(null);

    await expect(tauriRubricPicker().choose()).resolves.toBeNull();
  });

  it("turns the adopted image into a rubric with a data url and its size", async () => {
    invoke.mockResolvedValue({
      rubric: { base64: "/9j/", width: 200, height: 80 },
      failure: null,
    });

    const choice = await tauriRubricPicker().choose();

    expect(choice).toEqual({
      rubric: { dataUrl: "data:image/jpeg;base64,/9j/", width: 200, height: 80 },
    });
  });

  it("keeps the situation and the raw detail of an image that was refused", async () => {
    invoke.mockResolvedValue({
      rubric: null,
      failure: { situation: "notAnAcceptedImage", detail: "no es PNG ni JPEG" },
    });

    const choice = await tauriRubricPicker().choose();

    expect(choice).toEqual({
      failure: { situation: "notAnAcceptedImage", detail: "no es PNG ni JPEG" },
    });
  });

  it("asks the backend for what a previous session adopted, and nothing else", async () => {
    invoke.mockResolvedValue(null);

    await tauriRubricPicker().stored();

    expect(invoke.mock.calls.map(([command]) => command)).toEqual(["read_rubric"]);
  });

  it("reads no stored rubric as null", async () => {
    invoke.mockResolvedValue(null);

    await expect(tauriRubricPicker().stored()).resolves.toBeNull();
  });

  it("turns the stored image into the same rubric shape as choosing one", async () => {
    invoke.mockResolvedValue({ base64: "/9j/", width: 200, height: 80 });

    const found = await tauriRubricPicker().stored();

    expect(found).toEqual({ dataUrl: "data:image/jpeg;base64,/9j/", width: 200, height: 80 });
  });

  it("asks the backend for the visible signature a previous session configured", async () => {
    invoke.mockResolvedValue({ content: null, withRubric: false });

    await tauriVisibleSignatureMemory().read();

    expect(invoke.mock.calls.map(([command]) => command)).toEqual(["remembered_visible_signature"]);
  });

  it("returns what the backend remembers, unchanged", async () => {
    const remembered = { content: { model: "rubricOnly" as const }, withRubric: true };
    invoke.mockResolvedValue(remembered);

    await expect(tauriVisibleSignatureMemory().read()).resolves.toEqual(remembered);
  });
});
