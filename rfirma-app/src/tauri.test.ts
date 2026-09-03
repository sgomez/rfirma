import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const listen = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const {
  tauriCertificateStore,
  tauriDestinations,
  tauriDocumentDrops,
  tauriDocumentPicker,
  tauriLanguagePreference,
  tauriLayer2Composer,
  tauriPdfSource,
  tauriPreferences,
  tauriRecents,
  tauriRubricPicker,
  tauriSigningBackend,
} = await import("./tauri");

const aConfiguration = {
  language: "es",
  destination: "Documentos",
  rememberVisibleSignature: true,
  rememberActivity: true,
  theme: "system",
};

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
});

/**
 * **Grada A**: el puerto del arrastre contra un `listen` falso.
 *
 * Lo que se comprueba es la costura, y aquí la costura es sobre todo **el
 * nombre del evento**: si deja de coincidir con `commands::DOCUMENT_DROPPED`,
 * nada falla en ninguna parte —ni compila peor, ni salta un error— y arrastrar
 * simplemente no hace nada.
 */
describe("el puerto del arrastre sobre Tauri", () => {
  beforeEach(() => {
    listen.mockReset();
  });

  /** Deja escuchar y devuelve con qué dejar de hacerlo. */
  function listening() {
    const stop = vi.fn();
    let emit: ((event: { payload: unknown }) => void) | undefined;
    listen.mockImplementation((_name: string, handler: (event: { payload: unknown }) => void) => {
      emit = handler;
      return Promise.resolve(stop);
    });
    return {
      stop,
      emit: (payload: unknown) => emit?.({ payload }),
    };
  }

  it("subscribes to the drag-and-drop event of the window, by its name", () => {
    listening();

    tauriDocumentDrops().subscribe(() => {});

    expect(listen.mock.calls.map(([name]) => name)).toEqual(["document-dropped"]);
  });

  it("turns a dropped document into a tray row badged Unsigned", async () => {
    const window = listening();
    const dropped: unknown[] = [];
    tauriDocumentDrops().subscribe((drop) => dropped.push(drop));

    window.emit({
      document: { id: "0f1e2d3c", name: "contrato.pdf", modified: 1_700_000_000 },
      failure: null,
      ignored: 2,
    });

    expect(dropped).toHaveLength(1);
    expect(dropped[0]).toMatchObject({
      document: { id: "0f1e2d3c", name: "contrato.pdf", badge: "Unsigned", available: true },
      failure: null,
      ignored: 2,
    });
  });

  it("keeps the situation and the raw detail of a drop that opened nothing", () => {
    const window = listening();
    const dropped: unknown[] = [];
    tauriDocumentDrops().subscribe((drop) => dropped.push(drop));

    window.emit({
      document: null,
      failure: { situation: "droppedFileUnreadable", detail: "os error 2" },
      ignored: 0,
    });

    expect(dropped[0]).toEqual({
      document: null,
      failure: { situation: "droppedFileUnreadable", detail: "os error 2" },
      ignored: 0,
    });
  });

  it("stops listening when the subscription is dropped", async () => {
    const window = listening();

    const unsubscribe = tauriDocumentDrops().subscribe(() => {});
    unsubscribe();
    await Promise.resolve();

    expect(window.stop).toHaveBeenCalled();
  });

  /**
   * Y cancelar **antes** de que `listen` resuelva también deja de escuchar: un
   * efecto de React se limpia cuando quiere, y sin esto desmontar deprisa
   * dejaba un oyente vivo para siempre.
   */
  it("stops listening even when the subscription is dropped before it is ready", async () => {
    const stop = vi.fn();
    let ready: (() => void) | undefined;
    listen.mockImplementation(
      () =>
        new Promise((resolve) => {
          ready = () => resolve(stop);
        }),
    );

    const unsubscribe = tauriDocumentDrops().subscribe(() => {});
    unsubscribe();
    ready?.();
    await Promise.resolve();
    await Promise.resolve();

    expect(stop).toHaveBeenCalled();
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
    placement: null,
  };
}

/**
 * **Grada A**: los ajustes y el idioma son el mismo fichero debajo, y lo que
 * se comprueba aquí es justo eso —que ninguno de los dos puertos pisa lo que
 * el otro acaba de guardar—.
 */
describe("los puertos de la configuración sobre Tauri", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("reads the settings the backend remembers, destination included", async () => {
    invoke.mockResolvedValue(aConfiguration);

    const read = await tauriPreferences().read();

    expect(invoke).toHaveBeenCalledWith("read_configuration");
    expect(read).toEqual({
      theme: "system",
      destination: "Documentos",
      rememberVisibleSignature: true,
      rememberActivity: true,
    });
  });

  it("falls back to the system theme when what is stored is not one of the three", async () => {
    invoke.mockResolvedValue({ ...aConfiguration, theme: "sepia" });

    expect((await tauriPreferences().read()).theme).toBe("system");
  });

  /**
   * El idioma no es de `Preferences` sino de su propio puerto, así que
   * guardar los ajustes con una copia local en vez de releer devolvería el
   * idioma anterior y desharía el cambio.
   */
  it("keeps the language the other port saved when the settings are written", async () => {
    invoke.mockImplementation((command: string) =>
      command === "read_configuration"
        ? Promise.resolve({ ...aConfiguration, language: "en" })
        : Promise.resolve(undefined),
    );

    await tauriPreferences().save({
      theme: "dark",
      destination: "Documentos",
      rememberVisibleSignature: false,
      rememberActivity: true,
    });

    expect(invoke).toHaveBeenLastCalledWith("write_configuration", {
      configuration: {
        ...aConfiguration,
        language: "en",
        theme: "dark",
        rememberVisibleSignature: false,
      },
    });
  });

  it("saves the language without touching the rest of the settings", async () => {
    invoke.mockImplementation((command: string) =>
      command === "read_configuration"
        ? Promise.resolve({ ...aConfiguration, theme: "dark" })
        : Promise.resolve(undefined),
    );

    await tauriLanguagePreference().save("en");

    expect(invoke).toHaveBeenLastCalledWith("write_configuration", {
      configuration: { ...aConfiguration, theme: "dark", language: "en" },
    });
  });

  it("falls back to Spanish when what is stored is not one of the six", async () => {
    invoke.mockResolvedValue({ ...aConfiguration, language: "fr" });

    expect(await tauriLanguagePreference().read()).toBe("es");
  });

  it("wires forgetting the activity to its own command", async () => {
    invoke.mockResolvedValue(undefined);

    await tauriPreferences().forgetActivity();

    expect(invoke).toHaveBeenCalledWith("forget_activity");
  });

  it("picks the destination folder with the backend dialog and gets back a name", async () => {
    invoke.mockResolvedValue("Firmados");

    await expect(tauriPreferences().chooseFolder()).resolves.toBe("Firmados");
    expect(invoke.mock.calls.map(([command]) => command)).toEqual(["choose_destination"]);
  });

  it("reads a cancelled directory picker as no choice, and not as a failure", async () => {
    invoke.mockResolvedValue(null);

    await expect(tauriPreferences().chooseFolder()).resolves.toBeNull();
  });
});

/**
 * **Grada A**: el destino sobre Tauri. Quien lo compone —la carpeta comprobada
 * y el nombre con su homónimo resuelto— es `app::documents::where_it_lands`, y
 * está probado allí; aquí solo se comprueba la costura.
 */
describe("el puerto del destino sobre Tauri", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("asks where the open document will land, by its identifier and never by a path", async () => {
    invoke.mockResolvedValue({
      folder: "Documentos",
      name: "contrato-firmado.pdf",
      writable: true,
    });

    const destination = await tauriDestinations().previewFor("1e8b83b9");

    expect(invoke).toHaveBeenCalledWith("preview_destination", { id: "1e8b83b9" });
    expect(destination).toEqual({
      folder: "Documentos",
      name: "contrato-firmado.pdf",
      writable: true,
    });
  });
});

/**
 * **Grada A**: la bandeja sobre Tauri. Lo que se comprueba es la frontera —qué
 * orden se llama y con qué—, no las reglas de la lista: esas son de
 * `memory::recents` y ya están probadas allí.
 */
describe("la bandeja sobre Tauri", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  const aStoredRow = {
    id: "0f1e2d3c",
    name: "contrato.pdf",
    badge: "Unsigned" as const,
    modified: 1_700_000_000,
    lastUsed: 1_700_000_000,
    available: false,
    placement: {
      rect: [72, 500, 272, 600] as [number, number, number, number],
      pages: { only: [3] },
    },
  };

  it("lists the tray with the availability the backend just recomputed", async () => {
    invoke.mockResolvedValue([aStoredRow]);

    const rows = await tauriRecents().list();

    expect(invoke).toHaveBeenCalledWith("list_recents");
    expect(rows[0]?.available).toBe(false);
    expect(rows[0]?.placement).toEqual({
      rect: { x0: 72, y0: 500, x1: 272, y1: 600 },
      pages: { only: [3] },
    });
  });

  it("records a document by its opaque identifier and never by a path", async () => {
    invoke.mockResolvedValue({ ...aStoredRow, available: true });

    await tauriRecents().record({
      ...aRow(),
      placement: { rect: { x0: 72, y0: 500, x1: 272, y1: 600 }, pages: { only: [3] } },
    });

    expect(invoke).toHaveBeenCalledWith("record_recent", {
      id: "0f1e2d3c",
      placement: { rect: [72, 500, 272, 600], pages: { only: [3] } },
    });
  });

  it("hands back the row the backend already had, box included", async () => {
    invoke.mockResolvedValue({ ...aStoredRow, available: true });

    const noted = await tauriRecents().record(aRow());

    expect(invoke).toHaveBeenCalledWith("record_recent", { id: "0f1e2d3c", placement: null });
    expect(noted.placement).toEqual({
      rect: { x0: 72, y0: 500, x1: 272, y1: 600 },
      pages: { only: [3] },
    });
  });

  it("forgets a single row by its identifier", async () => {
    invoke.mockResolvedValue(undefined);

    await tauriRecents().forget("0f1e2d3c");

    expect(invoke).toHaveBeenCalledWith("forget_recent", { id: "0f1e2d3c" });
  });

  it("empties the whole list through the order that already did it", () => {
    // «Vaciar la lista» no estrena orden: es `forget_activity`, la misma
    // promesa que apagar «Recordar mi actividad» (ID-34).
    invoke.mockResolvedValue(undefined);

    void tauriRecents().clear();

    expect(invoke).toHaveBeenCalledWith("forget_activity");
  });
});
