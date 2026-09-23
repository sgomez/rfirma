import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const listen = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const { tauriDocumentDrops, tauriDocumentPicker, tauriPdfSource, tauriRecents } = await import(
  "./tauriDocuments"
);

/** El documento que se tiene delante, no la fila que se guarda (ID-287). */
function aDocument() {
  return {
    id: "0f1e2d3c",
    name: "contrato.pdf",
    badge: "Unsigned" as const,
    modified: 1_700_000_000,
    placement: null,
    remembered: true,
  };
}

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

  it("turns what the portal granted into a document in hand badged Unsigned", async () => {
    // Detectar si un PDF ya trae firmas es otro trabajo: se anota lo que se
    // sabe y no se inventa (ID-71).
    invoke.mockResolvedValue({ id: "0f1e2d3c", name: "contrato.pdf", modified: 1_700_000_000 });

    const chosen = await tauriDocumentPicker().choose();

    expect(chosen).toEqual({
      id: "0f1e2d3c",
      name: "contrato.pdf",
      badge: "Unsigned",
      modified: 1_700_000_000,
      placement: null,
      // Lo eligió una persona, así que de esto queda rastro (ID-34).
      remembered: true,
    });
  });

  it("reads a cancelled dialog as no document, and not as a failure", async () => {
    invoke.mockResolvedValue(null);

    await expect(tauriDocumentPicker().choose()).resolves.toBeNull();
  });

  it("asks for the bytes of a document by its identifier, never by a path", async () => {
    invoke.mockResolvedValue(new Uint8Array([1, 2, 3]).buffer);

    await tauriPdfSource().open(aDocument());

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

    const outcome = await tauriPdfSource().open(aDocument());

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

    const outcome = await tauriPdfSource().open(aDocument());

    expect(outcome.ok).toBe(false);
    if (outcome.ok) return;
    expect(outcome.failure.situation).toBe("documentUnreadable");
    expect(outcome.failure.detail).not.toBe("");
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
      alsoEntering: [],
      failure: null,
      discarded: 2,
    });

    expect(dropped).toHaveLength(1);
    expect(dropped[0]).toMatchObject({
      document: { id: "0f1e2d3c", name: "contrato.pdf", badge: "Unsigned", remembered: true },
      failure: null,
      discarded: 2,
    });
  });

  /** ID-306: cada PDF del mismo gesto entra igual en Recientes. */
  it("also turns the rest of the dropped PDFs into tray rows", () => {
    const window = listening();
    const dropped: unknown[] = [];
    tauriDocumentDrops().subscribe((drop) => dropped.push(drop));

    window.emit({
      document: { id: "0f1e2d3c", name: "contrato.pdf", modified: 1_700_000_000 },
      alsoEntering: [{ id: "1a2b3c4d", name: "factura.pdf", modified: 1_700_000_001 }],
      failure: null,
      discarded: 0,
    });

    expect(dropped[0]).toMatchObject({
      alsoEntering: [{ id: "1a2b3c4d", name: "factura.pdf", badge: "Unsigned", remembered: true }],
    });
  });

  it("keeps the situation and the raw detail of a drop that opened nothing", () => {
    const window = listening();
    const dropped: unknown[] = [];
    tauriDocumentDrops().subscribe((drop) => dropped.push(drop));

    window.emit({
      document: null,
      alsoEntering: [],
      failure: { situation: "droppedFileUnreadable", detail: "os error 2" },
      discarded: 0,
    });

    expect(dropped[0]).toEqual({
      document: null,
      alsoEntering: [],
      failure: { situation: "droppedFileUnreadable", detail: "os error 2" },
      discarded: 0,
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
      ...aDocument(),
      placement: { rect: { x0: 72, y0: 500, x1: 272, y1: 600 }, pages: { only: [3] } },
    });

    expect(invoke).toHaveBeenCalledWith("record_recent", {
      id: "0f1e2d3c",
      placement: { rect: [72, 500, 272, 600], pages: { only: [3] } },
    });
  });

  it("hands back the row the backend already had, box included", async () => {
    invoke.mockResolvedValue({ ...aStoredRow, available: true });

    const noted = await tauriRecents().record(aDocument());

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

/**
 * **Grada A**: la invocación desde fuera, contra un `invoke` falso.
 *
 * La costura es el nombre de la orden, `read_invocation`, y que lo que vuelve
 * se traduzca **igual** que un arrastre: ese «igual» es el ID-159, y es lo
 * único que hace que invocar y arrastrar dejen la misma ventana.
 */
describe("el documento con el que se invocó a la aplicación", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("asks the backend for it, and reads it like a drop", async () => {
    invoke.mockResolvedValue({
      document: { id: "0f1e2d3c", name: "contrato.pdf", modified: 1_700_000_000 },
      alsoEntering: [],
      failure: null,
      discarded: 0,
    });

    const invoked = await tauriDocumentDrops().pending();

    expect(invoke).toHaveBeenCalledWith("read_invocation");
    expect(invoked).toMatchObject({
      document: { id: "0f1e2d3c", name: "contrato.pdf", badge: "Unsigned" },
      failure: null,
      discarded: 0,
    });
  });

  it("brings nothing when the application was opened without a document", async () => {
    invoke.mockResolvedValue(null);

    expect(await tauriDocumentDrops().pending()).toBeNull();
  });
});
